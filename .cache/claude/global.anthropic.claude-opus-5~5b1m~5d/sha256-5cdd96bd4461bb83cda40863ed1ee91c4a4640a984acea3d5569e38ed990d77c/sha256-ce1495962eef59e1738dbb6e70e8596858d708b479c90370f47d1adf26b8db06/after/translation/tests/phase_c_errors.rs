//! Phase C — error / rejection differential tests.
//! One test per row of ERRORS.md that is not already covered in the Phase B
//! files, plus the generic FFI boundaries (NULL pointers, zero / oversized
//! lengths, one-past-range and out-of-range enum values).
mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::{c_char, c_int};


// NOTE on `mode >= 2` + `stbds_hmdel_key`:
// When an element has to be swapped in from the end (`old_index != final_index`)
// the C re-finds the moved element's slot with
//     mode == STBDS_HM_STRING ? *(char**)(a+elemsize*old_index+keyoffset)
//                             :  (char* )(a+elemsize*old_index+keyoffset)
// For `mode >= 2` the first test is FALSE while `stbds_hm_find_slot` still takes
// the *string* path, so it hashes the wrong bytes, returns -1 and trips
// `STBDS_ASSERT(slot >= 0)` -> abort. That row is verified for abort-parity in
// tests/phase_c_aborts.rs. The in-process tests below therefore delete in
// reverse insertion order, which keeps `old_index == final_index` and never
// reaches the re-find.

const ELEMSIZE: usize = 16;
const KEYSIZE: usize = 8;

/// Unique NUL-terminated key (uniqueness matters for the tests that delete in
/// reverse insertion order — see the `mode >= 2` note above).
fn uniq_cstring(rng: &mut Rng, i: usize) -> Vec<u8> {
    let mut v = format!("u{i}_").into_bytes();
    let tail = rng.cstring_range(0, 14, ASCII);
    v.extend_from_slice(&tail[..tail.len() - 1]);
    v.push(0);
    v
}

/// Unique binary key of `n` bytes (first 4 bytes encode `i`).
fn uniq_bytes(rng: &mut Rng, i: usize, n: usize) -> Vec<u8> {
    let mut v = rng.bytes(n);
    for (j, b) in (i as u32).to_le_bytes().iter().enumerate() {
        if j < n {
            v[j] = *b;
        }
    }
    v
}


/// Build a raw array (no hash table) and return the "map user pointer" view.
unsafe fn tableless_map(p: &Pair, elemsize: usize) -> (*mut c_void, *mut c_void) {
    let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
    let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4);
    // one "default" row, like the library's own bootstrap
    (*((ca as usize - HEADER_SIZE) as *mut CHeader)).length = 1;
    (*((ra as usize - HEADER_SIZE) as *mut CHeader)).length = 1;
    std::ptr::write_bytes(ca as *mut u8, 0, elemsize);
    std::ptr::write_bytes(ra as *mut u8, 0, elemsize);
    (
        (ca as usize + elemsize) as *mut c_void,
        (ra as usize + elemsize) as *mut c_void,
    )
}

// ===========================================================================
// E07 / E08 / E09 : stbds_hmfree_func
// ===========================================================================

#[test]
fn e07_hmfree_null() {
    let p = fresh_pair(0x07);
    unsafe {
        for elemsize in [0usize, 1, 8, 16, usize::MAX] {
            // must return immediately; no crash, no free
            (p.c.hmfree_func)(std::ptr::null_mut(), elemsize);
            (p.r.hmfree_func)(std::ptr::null_mut(), elemsize);
        }
    }
}

#[test]
fn e08_hmfree_no_table() {
    let p = fresh_pair(0x08);
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            let ca = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            let ra = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 4, 0);
            same_val(
                &format!("e08 hash_table NULL elemsize={elemsize}"),
                header_of(ca).hash_table.is_null(),
                header_of(ra).hash_table.is_null(),
            );
            (p.c.hmfree_func)(ca, elemsize);
            (p.r.hmfree_func)(ra, elemsize);
        }
    }
}

#[test]
fn e09_hmfree_table_not_strdup() {
    let p = fresh_pair(0x09);
    for &shmode in &[SH_NONE, SH_DEFAULT, SH_ARENA] {
        let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, shmode, KeyRepr::Auto);
        let sk = SelfKeys::new(10);
        for (i, &k) in sk.keys.iter().enumerate() {
            m.put(k, &[i as u8; 8]);
        }
        m.check(&format!("e09 shmode={shmode} before free"));
        // freeing must not touch the element key pointers for non-STRDUP tables
        m.free();
    }
}

// ===========================================================================
// E10 / E11 / E12 / E13 / E14 : hmget_key / hmget_key_ts
// ===========================================================================

#[test]
fn e10_find_slot_miss() {
    let p = fresh_pair(0x10);
    let mut rng = Rng::new(0x10);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    for _ in 0..30 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    for i in 0..200 {
        let miss = ka.add(&rng.bytes(KEYSIZE));
        let (gc, gr) = m.get(miss);
        same_val(&format!("e10 miss#{i} temp"), gc, gr);
        same_val(&format!("e10 miss#{i} temp == -1"), gc, -1isize);
        m.check(&format!("e10 miss#{i}"));
    }
    m.free();
}

#[test]
fn e11_hmget_ts_null_a() {
    let p = fresh_pair(0x11);
    unsafe {
        for elemsize in [1usize, 8, 16, 24] {
            for mode in [HM_BINARY, HM_STRING, -1, 2, c_int::MAX, c_int::MIN] {
                let mut key = *b"abcdefg\0";
                let mut tc: isize = 0x1234;
                let mut tr: isize = 0x1234;
                let ct = (p.c.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    KEYSIZE,
                    &mut tc,
                    mode,
                );
                let rt = (p.r.hmget_key_ts)(
                    std::ptr::null_mut(),
                    elemsize,
                    key.as_mut_ptr() as *mut c_void,
                    KEYSIZE,
                    &mut tr,
                    mode,
                );
                let ctx = format!("e11 elemsize={elemsize} mode={mode}");
                same_val(&format!("{ctx} temp"), tc, tr);
                same_val(&format!("{ctx} temp == -1"), tc, -1isize);
                same_val(&format!("{ctx} null-ness"), ct.is_null(), rt.is_null());
                same(
                    &ctx,
                    &snap_map(ct, elemsize, KeyRepr::Inline),
                    &snap_map(rt, elemsize, KeyRepr::Inline),
                );
                (p.c.hmfree_func)((ct as usize - elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)((rt as usize - elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

#[test]
fn e12_hmget_ts_no_table() {
    let p = fresh_pair(0x12);
    unsafe {
        for mode in [HM_BINARY, HM_STRING, -7, 3, c_int::MAX] {
            let (ct0, rt0) = tableless_map(&p, ELEMSIZE);
            let mut key = *b"zzzzzzz\0";
            let mut tc: isize = 0x4242;
            let mut tr: isize = 0x4242;
            let ct = (p.c.hmget_key_ts)(
                ct0,
                ELEMSIZE,
                key.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                &mut tc,
                mode,
            );
            let rt = (p.r.hmget_key_ts)(
                rt0,
                ELEMSIZE,
                key.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                &mut tr,
                mode,
            );
            let ctx = format!("e12 mode={mode}");
            same_val(&format!("{ctx} temp"), tc, tr);
            same_val(&format!("{ctx} temp == -1"), tc, -1isize);
            same_val(&format!("{ctx} returns a unchanged"), ct == ct0, rt == rt0);
            same(
                &ctx,
                &snap_map(ct, ELEMSIZE, KeyRepr::Inline),
                &snap_map(rt, ELEMSIZE, KeyRepr::Inline),
            );
            (p.c.hmfree_func)((ct as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((rt as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
}

#[test]
fn e13_hmget_ts_absent_key() {
    let p = fresh_pair(0x13);
    let mut rng = Rng::new(0x13);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    for _ in 0..25 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    for i in 0..150 {
        let miss = ka.add(&rng.bytes(KEYSIZE));
        let (tc, tr) = m.get_ts(miss);
        same_val(&format!("e13 miss#{i} temp"), tc, tr);
        same_val(&format!("e13 miss#{i} == -1"), tc, -1isize);
        m.check(&format!("e13 miss#{i}"));
    }
    m.free();
}

#[test]
fn e14_hmget_key_writes_temp() {
    let p = fresh_pair(0x14);
    let mut rng = Rng::new(0x14);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    for _ in 0..25 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        keys.push(k);
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    for (i, k) in keys.iter().enumerate() {
        let (ts_c, ts_r) = m.get_ts(*k);
        let (g_c, g_r) = m.get(*k);
        same_val(&format!("e14 #{i} get_ts"), ts_c, ts_r);
        same_val(&format!("e14 #{i} get"), g_c, g_r);
        // hmget_key must have written header->temp == the _ts value
        same_val(&format!("e14 #{i} temp mirrors _ts"), g_c, ts_c);
        m.check(&format!("e14 #{i}"));
    }
    // also on a NULL map
    unsafe {
        let mut key = *b"nope\0\0\0\0";
        let mut tsc: isize = 5;
        let mut tsr: isize = 5;
        let a = (p.c.hmget_key_ts)(
            std::ptr::null_mut(),
            ELEMSIZE,
            key.as_mut_ptr() as *mut c_void,
            KEYSIZE,
            &mut tsc,
            HM_BINARY,
        );
        let b = (p.r.hmget_key_ts)(
            std::ptr::null_mut(),
            ELEMSIZE,
            key.as_mut_ptr() as *mut c_void,
            KEYSIZE,
            &mut tsr,
            HM_BINARY,
        );
        let c = (p.c.hmget_key)(
            std::ptr::null_mut(),
            ELEMSIZE,
            key.as_mut_ptr() as *mut c_void,
            KEYSIZE,
            HM_BINARY,
        );
        let d = (p.r.hmget_key)(
            std::ptr::null_mut(),
            ELEMSIZE,
            key.as_mut_ptr() as *mut c_void,
            KEYSIZE,
            HM_BINARY,
        );
        same_val("e14 null _ts temp", tsc, tsr);
        same_val(
            "e14 null hmget_key temp",
            map_header(c, ELEMSIZE).temp,
            map_header(d, ELEMSIZE).temp,
        );
        same_val(
            "e14 null hmget_key temp == -1",
            map_header(c, ELEMSIZE).temp,
            -1isize,
        );
        for t in [a, b, c, d] {
            let lib = if t == a || t == c { &p.c } else { &p.r };
            (lib.hmfree_func)((t as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
    m.free();
}

// ===========================================================================
// E15 / E16 : out-of-range `mode`
// ===========================================================================

#[test]
fn e15_mode_negative() {
    let p = fresh_pair(0x15);
    // negative modes must take the BINARY path (mode >= STBDS_HM_STRING is false)
    for mode in [-1, -2, -100, c_int::MIN, c_int::MIN + 1] {
        let mut rng = Rng::new(0x15 ^ mode as i64 as u64);
        let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, mode, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut keys = Vec::new();
        for _ in 0..30 {
            let k = ka.add(&rng.bytes(KEYSIZE));
            keys.push(k);
            let (a, b) = m.put(k, &rng.bytes(ELEMSIZE));
            same_val(&format!("e15 mode={mode} put temp"), a, b);
            m.check(&format!("e15 mode={mode} put"));
        }
        // the table must have string.mode 0 (binary), i.e. keys stored by memcpy
        unsafe {
            same_val(
                &format!("e15 mode={mode} string.mode"),
                table_of(m.ct, ELEMSIZE).unwrap().string.mode,
                0u8,
            );
            same_val(
                &format!("e15 mode={mode} string.mode parity"),
                table_of(m.ct, ELEMSIZE).unwrap().string.mode,
                table_of(m.rt, ELEMSIZE).unwrap().string.mode,
            );
        }
        for k in &keys {
            let (a, b) = m.get(*k);
            same_val(&format!("e15 mode={mode} get temp"), a, b);
            let (c, d) = m.del(*k, 0);
            same_val(&format!("e15 mode={mode} del temp"), c, d);
            m.check(&format!("e15 mode={mode} del"));
        }
        m.free();
    }
}

#[test]
fn e16_mode_above_range() {
    let p = fresh_pair(0x16);
    // any mode >= 1 must take the STRING path
    for mode in [1, 2, 3, 7, 1000, c_int::MAX, c_int::MAX - 1] {
        let mut rng = Rng::new(0x16 ^ mode as u64);
        let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, mode, KeyRepr::Auto);
        let mut ka = KeyArena::new();
        let mut keys = Vec::new();
        for i in 0..30 {
            let kb = uniq_cstring(&mut rng, i);
            let k = ka.add(&kb);
            keys.push(k);
            let (a, b) = m.put(k, &rng.bytes(ELEMSIZE));
            same_val(&format!("e16 mode={mode} put temp"), a, b);
            m.check(&format!("e16 mode={mode} put"));
        }
        unsafe {
            // implicit table => string.mode == SH_DEFAULT
            same_val(
                &format!("e16 mode={mode} string.mode"),
                table_of(m.ct, ELEMSIZE).unwrap().string.mode,
                1u8,
            );
        }
        for k in keys.iter().rev() {
            let (a, b) = m.get(*k);
            same_val(&format!("e16 mode={mode} get temp"), a, b);
            let (c, d) = m.del(*k, 0);
            same_val(&format!("e16 mode={mode} del temp"), c, d);
            m.check(&format!("e16 mode={mode} del"));
        }
        m.free();
        std::mem::forget(ka);
    }
}

// ===========================================================================
// E20 : hmput_key with a == NULL
// ===========================================================================

#[test]
fn e20_hmput_key_null_a() {
    let p = fresh_pair(0x20);
    unsafe {
        for elemsize in [1usize, 4, 8, 16, 24] {
            for keysize in [0usize, 1, 4, 8] {
                if keysize > elemsize {
                    continue;
                }
                for mode in [HM_BINARY, HM_STRING] {
                    // STRING mode stores a `char *` at element offset 0, so the
                    // element must be able to hold a pointer; anything smaller
                    // would be a heap overflow in the C itself.
                    if mode >= HM_STRING && (elemsize < 8 || keysize != 8) {
                        continue;
                    }
                    let mut key = *b"key12345\0\0\0\0\0\0\0\0";
                    let ct = (p.c.hmput_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_mut_ptr() as *mut c_void,
                        keysize,
                        mode,
                    );
                    let rt = (p.r.hmput_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        key.as_mut_ptr() as *mut c_void,
                        keysize,
                        mode,
                    );
                    let ctx = format!("e20 elemsize={elemsize} keysize={keysize} mode={mode}");
                    same_val(&format!("{ctx} null-ness"), ct.is_null(), rt.is_null());
                    same_val(
                        &format!("{ctx} temp == 0"),
                        map_header(ct, elemsize).temp,
                        0isize,
                    );
                    same_val(
                        &format!("{ctx} temp parity"),
                        map_header(ct, elemsize).temp,
                        map_header(rt, elemsize).temp,
                    );
                    let kr = if mode >= HM_STRING {
                        KeyRepr::InlineKeyOnly(8)
                    } else {
                        KeyRepr::InlineKeyOnly(keysize)
                    };
                    same(&ctx, &snap_map(ct, elemsize, kr), &snap_map(rt, elemsize, kr));
                    (p.c.hmfree_func)((ct as usize - elemsize) as *mut c_void, elemsize);
                    (p.r.hmfree_func)((rt as usize - elemsize) as *mut c_void, elemsize);
                }
            }
        }
    }
}

// ===========================================================================
// E23 / E24 / E25 : engineered probe paths (deterministic via rand_seed)
// ===========================================================================

/// Builds a map with keys landing on exactly the requested slots.
fn engineered(p: &Pair, seed: usize, slots: &[usize]) -> (Vec<[u8; 4]>, usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
        let mut counter = 1u32;
        let keys: Vec<[u8; 4]> = slots
            .iter()
            .map(|&s| key_at_slot_bin(&p.c, seed, 8, s, &mut counter))
            .collect();
        // sanity: the Rust hash must agree, otherwise the whole setup is void
        for (k, &s) in keys.iter().zip(slots) {
            let mut kk = *k;
            let hr = effective_hash_bytes(&p.r, &mut kk, seed);
            same_val("engineered: rust probe slot", hr & 7, s);
        }
        (keys, seed)
    }
}

#[test]
fn e23_hmput_dup_first_scan() {
    let p = pair();
    let (keys, _) = engineered(&p, 0xABCD_1234, &[3, 4, 5]);
    let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let ks: Vec<*mut u8> = keys.iter().map(|k| ka.add(k)).collect();
    for (i, &k) in ks.iter().enumerate() {
        m.put(k, &[i as u8; 4]);
        m.check(&format!("e23 put#{i}"));
    }
    // each key sits exactly at its probe slot -> duplicate found in the FIRST scan
    for (i, &k) in ks.iter().enumerate() {
        let (a, b) = m.put(k, &[0xF0 | i as u8; 4]);
        same_val(&format!("e23 dup#{i} temp"), a, b);
        same_val(&format!("e23 dup#{i} temp == {i}"), a, i as isize);
        m.check(&format!("e23 dup#{i}"));
    }
    m.free();
}

#[test]
fn e24_hmput_dup_wrap_scan() {
    let p = pair();
    // slots 5,6,7 get occupied first; the 4th key also probes at 5, finds
    // 5..7 taken, and is placed at slot 0. Re-putting it therefore hits the
    // wrap-around duplicate branch (which does NOT update temp_key).
    let (keys, _) = engineered(&p, 0x5EED_0001, &[5, 6, 7, 5]);
    let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let ks: Vec<*mut u8> = keys.iter().map(|k| ka.add(k)).collect();
    for (i, &k) in ks.iter().enumerate() {
        m.put(k, &[i as u8; 4]);
        m.check(&format!("e24 put#{i}"));
    }
    unsafe {
        let b = bucket(m.ct, 8, 0);
        assert!(b.index[0] >= 0, "4th key must have wrapped to slot 0: {:?}", b.index);
        same_val("e24 bucket layout", b.index, bucket(m.rt, 8, 0).index);
        same_val("e24 bucket hashes", b.hash, bucket(m.rt, 8, 0).hash);
    }
    // re-put the wrapped key: duplicate found in the wrap-around scan
    let (a, bb) = m.put(ks[3], &[0x99; 4]);
    same_val("e24 wrap dup temp", a, bb);
    same_val("e24 wrap dup temp == 3", a, 3isize);
    m.check("e24 wrap dup");
    // ... and via hmget / hmdel too
    let (g1, g2) = m.get(ks[3]);
    same_val("e24 wrap get temp", g1, g2);
    let (d1, d2) = m.del(ks[3], 0);
    same_val("e24 wrap del temp", d1, d2);
    m.check("e24 wrap del");
    m.free();
}

#[test]
fn e25_hmput_reuses_tombstone() {
    let p = pair();
    let (keys, _) = engineered(&p, 0x7031_0002, &[5, 6, 5]);
    let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let ks: Vec<*mut u8> = keys.iter().map(|k| ka.add(k)).collect();
    m.put(ks[0], &[1u8; 4]);
    m.put(ks[1], &[2u8; 4]);
    m.check("e25 filled 5,6");
    // delete the key at slot 5 -> tombstone at slot 5
    let (d1, d2) = m.del(ks[0], 0);
    same_val("e25 del temp", d1, d2);
    m.check("e25 tombstone at 5");
    unsafe {
        let b = bucket(m.ct, 8, 0);
        same_val("e25 hash[5] == STBDS_HASH_DELETED", b.hash[5], 1usize);
        same_val("e25 index[5] == STBDS_INDEX_DELETED", b.index[5], -2isize);
        same_val("e25 tombstone_count", table_of(m.ct, 8).unwrap().tombstone_count, 1usize);
        same_val(
            "e25 tombstone parity",
            table_of(m.ct, 8).unwrap().tombstone_count,
            table_of(m.rt, 8).unwrap().tombstone_count,
        );
    }
    // insert a key that probes at 5: must reuse the tombstone
    let (a, b) = m.put(ks[2], &[3u8; 4]);
    same_val("e25 reuse temp", a, b);
    m.check("e25 tombstone reused");
    unsafe {
        let bk = bucket(m.ct, 8, 0);
        assert!(bk.index[5] >= 0, "tombstone slot must be reused: {:?}", bk.index);
        same_val("e25 tombstone_count back to 0", table_of(m.ct, 8).unwrap().tombstone_count, 0usize);
        same_val("e25 bucket parity", bk.index, bucket(m.rt, 8, 0).index);
    }
    m.free();
}

// ===========================================================================
// E27..E30 : key-storage modes
// ===========================================================================

#[test]
fn e27_put_mode_default() {
    let p = fresh_pair(0x27);
    let mut ka = KeyArena::new();
    let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, SH_DEFAULT, KeyRepr::Inline);
    for i in 0..12 {
        let kb = format!("key{i}\0").into_bytes();
        let k = ka.add(&kb);
        m.put(k, &[i as u8; 8]);
        // SH_DEFAULT stores the caller's pointer verbatim, so element bytes
        // (compared with KeyRepr::Inline) must be bit-identical
        m.check(&format!("e27 put#{i}"));
        unsafe {
            let raw = (m.ct as usize) as *const u8;
            let stored = *(raw.add(ELEMSIZE * (i)) as *const usize);
            same_val(&format!("e27 #{i} stored ptr == caller ptr"), stored, k as usize);
        }
    }
    m.free();
    std::mem::forget(ka);
}

#[test]
fn e28_put_mode_strdup() {
    let p = fresh_pair(0x28f);
    let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, SH_STRDUP, KeyRepr::CharPtr);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    for i in 0..12 {
        let kb = format!("dup{i}\0").into_bytes();
        let k = ka.add(&kb);
        keys.push((k, kb));
        m.put(k, &[i as u8; 8]);
        m.check(&format!("e28 put#{i}"));
        unsafe {
            // the stored pointer must NOT be the caller's
            let stored = *(m.ct as *const usize).byte_add(ELEMSIZE * i);
            assert_ne!(stored, k as usize, "SH_STRDUP must copy the key");
        }
    }
    // scribble the caller buffers: the map must be unaffected
    for (k, kb) in &keys {
        unsafe {
            for i in 0..kb.len() {
                *k.add(i) = b'?';
            }
            *k.add(kb.len() - 1) = 0;
        }
    }
    // Both libraries kept their own copies, so both snapshots must still show
    // the original strings and must still agree.
    m.check("e28 after scribbling caller buffers");
    m.free();
}

#[test]
fn e29_put_mode_arena() {
    let p = fresh_pair(0x29f);
    let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, SH_ARENA, KeyRepr::CharPtr);
    let mut ka = KeyArena::new();
    let mut rng = Rng::new(0x29f);
    for i in 0..60 {
        let kb = rng.cstring_range(1, 40, ASCII);
        let k = ka.add(&kb);
        m.put(k, &[i as u8; 8]);
        m.check(&format!("e29 put#{i}"));
        unsafe {
            let cti = table_of(m.ct, ELEMSIZE).unwrap();
            let rti = table_of(m.rt, ELEMSIZE).unwrap();
            same_val(
                &format!("e29 #{i} arena scalars"),
                (cti.string.remaining, cti.string.block, cti.string.mode),
                (rti.string.remaining, rti.string.block, rti.string.mode),
            );
        }
    }
    m.free();
}

#[test]
fn e30_put_mode_default_label() {
    let p = fresh_pair(0x30f);
    // string.mode values that fall through to the `default:` memcpy label
    for shmode in [SH_NONE, 4, 5, 17, 200, 255] {
        let sk = SelfKeys::new(12);
        let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, shmode, KeyRepr::Auto);
        unsafe {
            same_val(
                &format!("e30 shmode={shmode} string.mode parity"),
                table_of(m.ct, ELEMSIZE).unwrap().string.mode,
                table_of(m.rt, ELEMSIZE).unwrap().string.mode,
            );
        }
        for (i, &k) in sk.keys.iter().enumerate() {
            let (a, b) = m.put(k, &[i as u8; 8]);
            same_val(&format!("e30 shmode={shmode} put#{i} temp"), a, b);
            m.check(&format!("e30 shmode={shmode} put#{i}"));
        }
        for (i, &k) in sk.keys.iter().enumerate() {
            let (a, b) = m.get(k);
            same_val(&format!("e30 shmode={shmode} get#{i} temp"), a, b);
        }
        m.check(&format!("e30 shmode={shmode} after gets"));
        m.free();
    }
}

// ===========================================================================
// E32..E42 : hmdel_key
// ===========================================================================

#[test]
fn e32_hmdel_null_a() {
    let p = fresh_pair(0x32);
    unsafe {
        for elemsize in [0usize, 1, 8, 16] {
            for keysize in [0usize, 4, 8] {
                for keyoffset in [0usize, 4, 1000] {
                    for mode in [HM_BINARY, HM_STRING, -1, 5, c_int::MIN, c_int::MAX] {
                        let mut key = *b"x\0\0\0\0\0\0\0";
                        let c = (p.c.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_mut_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                        let r = (p.r.hmdel_key)(
                            std::ptr::null_mut(),
                            elemsize,
                            key.as_mut_ptr() as *mut c_void,
                            keysize,
                            keyoffset,
                            mode,
                        );
                        same_val(
                            &format!("e32 hmdel_key(NULL) elemsize={elemsize} mode={mode}"),
                            c as usize,
                            r as usize,
                        );
                        assert!(c.is_null(), "C must return NULL");
                    }
                }
            }
        }
    }
}

#[test]
fn e33_hmdel_no_table() {
    let p = fresh_pair(0x33);
    unsafe {
        for mode in [HM_BINARY, HM_STRING, -1, 9] {
            let (ct0, rt0) = tableless_map(&p, ELEMSIZE);
            // pre-set temp to a poison value; the C sets it to 0
            (*((ct0 as usize - ELEMSIZE - HEADER_SIZE) as *mut CHeader)).temp = 0x7777;
            (*((rt0 as usize - ELEMSIZE - HEADER_SIZE) as *mut CHeader)).temp = 0x7777;
            let mut key = *b"q\0\0\0\0\0\0\0";
            let ct = (p.c.hmdel_key)(
                ct0,
                ELEMSIZE,
                key.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                0,
                mode,
            );
            let rt = (p.r.hmdel_key)(
                rt0,
                ELEMSIZE,
                key.as_mut_ptr() as *mut c_void,
                KEYSIZE,
                0,
                mode,
            );
            let ctx = format!("e33 mode={mode}");
            same_val(&format!("{ctx} returns a"), ct == ct0, rt == rt0);
            assert!(ct == ct0, "C must return `a` unchanged");
            same_val(
                &format!("{ctx} temp == 0"),
                map_header(ct, ELEMSIZE).temp,
                0isize,
            );
            same(
                &ctx,
                &snap_map(ct, ELEMSIZE, KeyRepr::Inline),
                &snap_map(rt, ELEMSIZE, KeyRepr::Inline),
            );
            (p.c.hmfree_func)((ct as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((rt as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
}

#[test]
fn e34_hmdel_absent_key() {
    let p = fresh_pair(0x34);
    let mut rng = Rng::new(0x34);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    for _ in 0..25 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    let before = m.snaps().0;
    for i in 0..120 {
        let miss = ka.add(&rng.bytes(KEYSIZE));
        let (a, b) = m.del(miss, 0);
        same_val(&format!("e34 miss#{i} temp"), a, b);
        same_val(&format!("e34 miss#{i} temp == 0"), a, 0isize);
        m.check(&format!("e34 miss#{i}"));
    }
    // nothing changed except `temp`
    let after = m.snaps().0;
    let filt = |s: &Snap| -> Vec<String> {
        s.0.iter().filter(|l| !l.starts_with("temp=")).cloned().collect()
    };
    assert_eq!(filt(&before), filt(&after), "absent-key delete must be a no-op");
    m.free();
}

#[test]
fn e35_hmdel_present() {
    let p = fresh_pair(0x35);
    let mut rng = Rng::new(0x35);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    for _ in 0..40 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        keys.push(k);
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    for (i, k) in keys.iter().enumerate() {
        let lc = unsafe { map_header(m.ct, ELEMSIZE).length };
        let uc = unsafe { table_of(m.ct, ELEMSIZE).unwrap().used_count };
        let (a, b) = m.del(*k, 0);
        same_val(&format!("e35 #{i} temp"), a, b);
        same_val(&format!("e35 #{i} temp == 1"), a, 1isize);
        let lc2 = unsafe { map_header(m.ct, ELEMSIZE).length };
        let uc2 = unsafe { table_of(m.ct, ELEMSIZE).unwrap().used_count };
        same_val(&format!("e35 #{i} length-1"), lc2, lc - 1);
        same_val(&format!("e35 #{i} used_count-1"), uc2, uc - 1);
        m.check(&format!("e35 #{i}"));
    }
    m.free();
}

#[test]
fn e36_hmdel_last_element() {
    let p = pair();
    let (keys, _) = engineered(&p, 0x3600_0001, &[1, 2, 3]);
    let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let ks: Vec<*mut u8> = keys.iter().map(|k| ka.add(k)).collect();
    for (i, &k) in ks.iter().enumerate() {
        m.put(k, &[i as u8; 4]);
    }
    m.check("e36 filled");
    // delete in reverse insertion order -> always old_index == final_index
    for i in (0..ks.len()).rev() {
        let (a, b) = m.del(ks[i], 0);
        same_val(&format!("e36 del#{i} temp"), a, b);
        m.check(&format!("e36 del#{i}"));
    }
    m.free();
}

#[test]
fn e37_hmdel_swap_in_last() {
    let p = pair();
    let (keys, _) = engineered(&p, 0x3700_0001, &[1, 2, 3, 4]);
    let mut m = DiffMap::lazy(&p, 8, 4, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let ks: Vec<*mut u8> = keys.iter().map(|k| ka.add(k)).collect();
    for (i, &k) in ks.iter().enumerate() {
        m.put(k, &[0x10 | i as u8; 4]);
    }
    m.check("e37 filled");
    // delete in insertion order -> always old_index != final_index
    for i in 0..ks.len() {
        let (a, b) = m.del(ks[i], 0);
        same_val(&format!("e37 del#{i} temp"), a, b);
        m.check(&format!("e37 del#{i}"));
    }
    m.free();
}

#[test]
fn e38_hmdel_shrink() {
    let p = fresh_pair(0x38a);
    let mut rng = Rng::new(0x38a);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    for _ in 0..10 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        keys.push(k);
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    unsafe {
        same_val(
            "e38 slot_count 16 after 10 inserts",
            table_of(m.ct, ELEMSIZE).unwrap().slot_count,
            16usize,
        );
    }
    let mut ladder = Vec::new();
    for (i, k) in keys.iter().enumerate() {
        m.del(*k, 0);
        m.check(&format!("e38 del#{i}"));
        unsafe {
            let c = table_of(m.ct, ELEMSIZE).unwrap();
            let r = table_of(m.rt, ELEMSIZE).unwrap();
            same_val(&format!("e38 #{i} slot_count"), c.slot_count, r.slot_count);
            same_val(&format!("e38 #{i} used"), c.used_count, r.used_count);
            same_val(&format!("e38 #{i} tomb"), c.tombstone_count, r.tombstone_count);
            ladder.push(c.slot_count);
        }
    }
    assert!(ladder.contains(&8usize), "must shrink back to 8: {ladder:?}");
    m.free();
}

#[test]
fn e39_hmdel_no_shrink_at_8() {
    let p = fresh_pair(0x39a);
    let mut rng = Rng::new(0x39a);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    for _ in 0..4 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        keys.push(k);
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    unsafe {
        let t = table_of(m.ct, ELEMSIZE).unwrap();
        same_val("e39 slot_count", t.slot_count, 8usize);
        same_val("e39 shrink_thr forced to 0", t.used_count_shrink_threshold, 0usize);
        same_val(
            "e39 shrink_thr parity",
            t.used_count_shrink_threshold,
            table_of(m.rt, ELEMSIZE).unwrap().used_count_shrink_threshold,
        );
    }
    for (i, k) in keys.iter().enumerate() {
        m.del(*k, 0);
        m.check(&format!("e39 del#{i}"));
        unsafe {
            same_val(
                &format!("e39 #{i} still 8 slots"),
                table_of(m.ct, ELEMSIZE).unwrap().slot_count,
                8usize,
            );
        }
    }
    m.free();
}

#[test]
fn e40_hmdel_rebuild_tombstones() {
    let p = fresh_pair(0x40a);
    let mut rng = Rng::new(0x40a);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut keys = Vec::new();
    // 30 keys -> slot_count 64, tomb_thr = 8+4 = 12, shrink_thr = 16
    for _ in 0..30 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        keys.push(k);
        m.put(k, &rng.bytes(ELEMSIZE));
    }
    unsafe {
        let t = table_of(m.ct, ELEMSIZE).unwrap();
        same_val("e40 slot_count", t.slot_count, 64usize);
        same_val("e40 tomb_thr", t.tombstone_count_threshold, 12usize);
    }
    let mut saw_rebuild = false;
    for (i, k) in keys.iter().enumerate() {
        let before = unsafe { table_of(m.ct, ELEMSIZE).unwrap() };
        m.del(*k, 0);
        m.check(&format!("e40 del#{i}"));
        let after = unsafe { table_of(m.ct, ELEMSIZE).unwrap() };
        let after_r = unsafe { table_of(m.rt, ELEMSIZE).unwrap() };
        same_val(&format!("e40 #{i} tomb"), after.tombstone_count, after_r.tombstone_count);
        same_val(&format!("e40 #{i} slots"), after.slot_count, after_r.slot_count);
        if before.slot_count == after.slot_count && after.tombstone_count == 0 && i >= 12 {
            saw_rebuild = true;
        }
    }
    assert!(saw_rebuild, "tombstone rebuild at the same slot_count must occur");
    m.free();
}

#[test]
fn e41_hmdel_strdup_frees_key() {
    let p = fresh_pair(0x41a);
    let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, SH_STRDUP, KeyRepr::CharPtr);
    let mut ka = KeyArena::new();
    let mut rng = Rng::new(0x41a);
    let mut keys = Vec::new();
    for i in 0..40 {
        let kb = rng.cstring_range(3, 24, ASCII);
        let k = ka.add(&kb);
        keys.push(k);
        m.put(k, &[i as u8; 8]);
    }
    m.check("e41 filled");
    while !keys.is_empty() {
        let i = rng.below(keys.len());
        let k = keys.remove(i);
        let (a, b) = m.del(k, 0);
        same_val("e41 del temp", a, b);
        m.check(&format!("e41 after del, {} left", keys.len()));
    }
    m.free();
}

#[test]
fn e42_hmdel_mode_two() {
    let p = fresh_pair(0x42);
    // mode 2 is out of the STBDS_HM_* range: `mode >= STBDS_HM_STRING` is true
    // (string hashing/compare) but `mode == STBDS_HM_STRING` is false, so the
    // strdup'd key copy is NOT freed. Both libraries must behave identically.
    let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, 2, SH_STRDUP, KeyRepr::CharPtr);
    let mut ka = KeyArena::new();
    let mut rng = Rng::new(0x42);
    let mut keys = Vec::new();
    for i in 0..30 {
        let kb = uniq_cstring(&mut rng, i);
        let k = ka.add(&kb);
        keys.push(k);
        m.put(k, &[i as u8; 8]);
        m.check(&format!("e42 put#{i}"));
    }
    while let Some(k) = keys.pop() {
        let (a, b) = m.del(k, 0);
        same_val("e42 del temp", a, b);
        m.check(&format!("e42 after del, {} left", keys.len()));
    }
    m.free();
    // also mode = INT_MAX / 3 / 1000 on a strdup table
    for mode in [3, 1000, c_int::MAX] {
        let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, mode, SH_STRDUP, KeyRepr::CharPtr);
        let mut ka2 = KeyArena::new();
        let mut ks = Vec::new();
        for i in 0..12 {
            let kb = uniq_cstring(&mut rng, i);
            let k = ka2.add(&kb);
            ks.push(k);
            m.put(k, &[i as u8; 8]);
            m.check(&format!("e42 mode={mode} put#{i}"));
        }
        for k in ks.into_iter().rev() {
            let (a, b) = m.del(k, 0);
            same_val(&format!("e42 mode={mode} del temp"), a, b);
            m.check(&format!("e42 mode={mode} del"));
        }
        m.free();
    }
}

// ===========================================================================
// E44 / E45 : shmode_func out of range
// ===========================================================================

#[test]
fn e44_shmode_out_of_range() {
    let p = fresh_pair(0x44);
    let modes: Vec<c_int> = vec![
        -1,
        -2,
        -256,
        -255,
        4,
        5,
        17,
        200,
        254,
        255,
        256,
        257,
        258,
        259,
        1000,
        c_int::MAX,
        c_int::MIN,
    ];
    for &mode in &modes {
        unsafe {
            let ct = (p.c.shmode_func)(ELEMSIZE, mode);
            let rt = (p.r.shmode_func)(ELEMSIZE, mode);
            let ctx = format!("e44 shmode_func(_,{mode})");
            let cm = table_of(ct, ELEMSIZE).unwrap().string.mode;
            let rm = table_of(rt, ELEMSIZE).unwrap().string.mode;
            same_val(&format!("{ctx} string.mode"), cm, rm);
            same_val(
                &format!("{ctx} string.mode == (unsigned char) mode"),
                cm,
                (mode as u32 & 0xff) as u8,
            );
            same(
                &ctx,
                &snap_map(ct, ELEMSIZE, KeyRepr::Auto),
                &snap_map(rt, ELEMSIZE, KeyRepr::Auto),
            );
            (p.c.hmfree_func)((ct as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((rt as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
    // and drive a full pipeline for each truncated mode (self-referential keys
    // keep every storage variant dereferenceable)
    for &mode in &modes {
        let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, HM_STRING, mode, KeyRepr::Auto);
        let sk = SelfKeys::new(10);
        for (i, &k) in sk.keys.iter().enumerate() {
            let (a, b) = m.put(k, &[i as u8; 8]);
            same_val(&format!("e44 pipeline mode={mode} put#{i} temp"), a, b);
            m.check(&format!("e44 pipeline mode={mode} put#{i}"));
        }
        for (i, &k) in sk.keys.iter().enumerate() {
            let (a, b) = m.get(k);
            same_val(&format!("e44 pipeline mode={mode} get#{i} temp"), a, b);
        }
        m.check(&format!("e44 pipeline mode={mode} gets"));
        for (i, &k) in sk.keys.iter().enumerate() {
            let (a, b) = m.del(k, 0);
            same_val(&format!("e44 pipeline mode={mode} del#{i} temp"), a, b);
            m.check(&format!("e44 pipeline mode={mode} del#{i}"));
        }
        m.free();
    }
}

// ===========================================================================
// E59 : hash < 2 is bumped by +2
// ===========================================================================

#[test]
fn e59_hash_lt_2_bumped() {
    let p = pair();
    unsafe {
        // hash_string("", seed) == K + seed for a constant K, so a seed can be
        // chosen that drives the raw hash to exactly 0 and to exactly 1 — the two
        // values the `if (hash < 2) hash += 2;` guard exists for.
        let mut empty = vec![0u8];
        let k = (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, 0);
        same_val(
            "e59 K parity",
            k,
            (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, 0),
        );
        for want in [0usize, 1] {
            let seed = want.wrapping_sub(k);
            let hc = (p.c.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            let hr = (p.r.hash_string)(empty.as_mut_ptr() as *mut c_char, seed);
            same_val(&format!("e59 raw hash for want={want}"), hc, hr);
            same_val(&format!("e59 raw hash == {want}"), hc, want);

            // drive a real string map with that seed: the "" key's raw hash is
            // `want`, so the library must store `want + 2` in the bucket.
            (p.c.rand_seed)(seed);
            (p.r.rand_seed)(seed);
            let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_STRING, KeyRepr::CharPtr);
            let mut ka = KeyArena::new();
            let ek = ka.add(&[0u8]);
            m.put(ek, &[0xAB; 8]);
            m.check(&format!("e59 put empty key, want={want}"));
            let bk = bucket(m.ct, ELEMSIZE, 0);
            let bkr = bucket(m.rt, ELEMSIZE, 0);
            same_val(&format!("e59 bucket hashes want={want}"), bk.hash, bkr.hash);
            same_val(
                &format!("e59 bumped hash present want={want}"),
                bk.hash[(want + 2) & 7],
                want + 2,
            );
            // lookup and delete must apply the same bump
            let (g1, g2) = m.get(ek);
            same_val("e59 get temp", g1, g2);
            same_val("e59 get hit", g1, 0isize);
            let (d1, d2) = m.del(ek, 0);
            same_val("e59 del temp", d1, d2);
            m.check(&format!("e59 after del want={want}"));
            m.free();
            std::mem::forget(ka);
        }
    }
}

// ===========================================================================
// E63 : the make_hash_index invariant holds for every reachable slot_count
// ===========================================================================

#[test]
fn e63_make_hash_index_assert_holds() {
    let p = fresh_pair(0x63);
    let mut rng = Rng::new(0x63);
    let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, HM_BINARY, KeyRepr::Inline);
    let mut ka = KeyArena::new();
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..1200 {
        let k = ka.add(&rng.bytes(KEYSIZE));
        m.put(k, &rng.bytes(ELEMSIZE));
        unsafe {
            for (tag, t) in [("C", table_of(m.ct, ELEMSIZE)), ("Rust", table_of(m.rt, ELEMSIZE))] {
                let t = t.unwrap();
                assert!(
                    t.used_count_threshold + t.tombstone_count_threshold < t.slot_count,
                    "{tag}: STBDS_ASSERT would fire for slot_count={}",
                    t.slot_count
                );
                if tag == "C" {
                    seen.insert(t.slot_count);
                }
            }
            let c = table_of(m.ct, ELEMSIZE).unwrap();
            let r = table_of(m.rt, ELEMSIZE).unwrap();
            same_val(
                "e63 thresholds parity",
                (
                    c.slot_count,
                    c.used_count_threshold,
                    c.tombstone_count_threshold,
                    c.used_count_shrink_threshold,
                    c.slot_count_log2,
                ),
                (
                    r.slot_count,
                    r.used_count_threshold,
                    r.tombstone_count_threshold,
                    r.used_count_shrink_threshold,
                    r.slot_count_log2,
                ),
            );
        }
    }
    assert!(
        seen.contains(&2048usize),
        "should have grown far: {seen:?}"
    );
    m.free();
}

// ===========================================================================
// Generic FFI boundaries : B01..B05
// ===========================================================================

#[test]
fn b01_null_pointers() {
    let p = fresh_pair(0xB01);
    unsafe {
        let mut key = *b"nul\0\0\0\0\0";
        let kp = key.as_mut_ptr() as *mut c_void;
        // hash_bytes(NULL, 0, seed)
        for seed in [0usize, 1, usize::MAX] {
            same_val(
                &format!("b01 hash_bytes(NULL,0,{seed})"),
                (p.c.hash_bytes)(std::ptr::null_mut(), 0, seed),
                (p.r.hash_bytes)(std::ptr::null_mut(), 0, seed),
            );
        }
        // hmfree_func(NULL, _)
        (p.c.hmfree_func)(std::ptr::null_mut(), ELEMSIZE);
        (p.r.hmfree_func)(std::ptr::null_mut(), ELEMSIZE);
        // hmdel_key(NULL, ...)
        same_val(
            "b01 hmdel_key(NULL)",
            (p.c.hmdel_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, 0, HM_BINARY) as usize,
            (p.r.hmdel_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, 0, HM_BINARY) as usize,
        );
        // arrgrowf(NULL, ...)
        for (e, a, m) in [(8usize, 0usize, 0usize), (8, 0, 1), (8, 1, 0)] {
            let c = (p.c.arrgrowf)(std::ptr::null_mut(), e, a, m);
            let r = (p.r.arrgrowf)(std::ptr::null_mut(), e, a, m);
            same_val(&format!("b01 arrgrowf(NULL,{e},{a},{m})"), c.is_null(), r.is_null());
            if !c.is_null() {
                same_val(
                    "b01 arrgrowf header",
                    (header_of(c).length, header_of(c).capacity, header_of(c).temp),
                    (header_of(r).length, header_of(r).capacity, header_of(r).temp),
                );
                (p.c.arrfreef)(c);
                (p.r.arrfreef)(r);
            }
        }
        // hmput_key / hmget_key / hmget_key_ts / hmput_default on NULL
        let mut tc: isize = 3;
        let mut tr: isize = 3;
        let a1 = (p.c.hmget_key_ts)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, &mut tc, HM_BINARY);
        let a2 = (p.r.hmget_key_ts)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, &mut tr, HM_BINARY);
        same_val("b01 hmget_key_ts(NULL) temp", tc, tr);
        same(
            "b01 hmget_key_ts(NULL)",
            &snap_map(a1, ELEMSIZE, KeyRepr::Inline),
            &snap_map(a2, ELEMSIZE, KeyRepr::Inline),
        );
        let b1 = (p.c.hmget_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, HM_BINARY);
        let b2 = (p.r.hmget_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, HM_BINARY);
        same(
            "b01 hmget_key(NULL)",
            &snap_map(b1, ELEMSIZE, KeyRepr::Inline),
            &snap_map(b2, ELEMSIZE, KeyRepr::Inline),
        );
        let c1 = (p.c.hmput_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, HM_BINARY);
        let c2 = (p.r.hmput_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, HM_BINARY);
        same(
            "b01 hmput_key(NULL)",
            &snap_map(c1, ELEMSIZE, KeyRepr::InlineKeyOnly(KEYSIZE)),
            &snap_map(c2, ELEMSIZE, KeyRepr::InlineKeyOnly(KEYSIZE)),
        );
        let d1 = (p.c.hmput_default)(std::ptr::null_mut(), ELEMSIZE);
        let d2 = (p.r.hmput_default)(std::ptr::null_mut(), ELEMSIZE);
        same(
            "b01 hmput_default(NULL)",
            &snap_map(d1, ELEMSIZE, KeyRepr::Inline),
            &snap_map(d2, ELEMSIZE, KeyRepr::Inline),
        );
        for (l, t) in [(&p.c, a1), (&p.r, a2), (&p.c, b1), (&p.r, b2), (&p.c, c1), (&p.r, c2), (&p.c, d1), (&p.r, d2)] {
            (l.hmfree_func)((t as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
}

#[test]
fn b02_zero_lengths() {
    let p = fresh_pair(0xB02);
    unsafe {
        // hash_bytes with len 0
        let mut buf = [0xffu8; 8];
        same_val(
            "b02 hash_bytes len 0",
            (p.c.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, 42),
            (p.r.hash_bytes)(buf.as_mut_ptr() as *mut c_void, 0, 42),
        );
        // hash_string of ""
        let mut e = vec![0u8];
        same_val(
            "b02 hash_string \"\"",
            (p.c.hash_string)(e.as_mut_ptr() as *mut c_char, 42),
            (p.r.hash_string)(e.as_mut_ptr() as *mut c_char, 42),
        );
        // arrgrowf with all-zero arguments
        for e2 in [0usize, 1, 8] {
            let c = (p.c.arrgrowf)(std::ptr::null_mut(), e2, 0, 0);
            let r = (p.r.arrgrowf)(std::ptr::null_mut(), e2, 0, 0);
            same_val(&format!("b02 arrgrowf(NULL,{e2},0,0)"), c.is_null(), r.is_null());
        }
        // shmode_func / hmput_default / hmput_key with elemsize 0
        for shmode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let ct = (p.c.shmode_func)(0, shmode);
            let rt = (p.r.shmode_func)(0, shmode);
            same(
                &format!("b02 shmode_func(0,{shmode})"),
                &snap_map(ct, 0, KeyRepr::Inline),
                &snap_map(rt, 0, KeyRepr::Inline),
            );
            (p.c.hmfree_func)(ct, 0);
            (p.r.hmfree_func)(rt, 0);
        }
        // keysize 0
        let mut m = DiffMap::lazy(&p, 8, 0, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        for i in 0..6u8 {
            let k = ka.add(&[i; 4]);
            m.put(k, &[i; 8]);
            m.check(&format!("b02 keysize 0 put#{i}"));
        }
        m.free();
    }
}

#[test]
fn b03_oversized_lengths() {
    let p = fresh_pair(0xB03);
    unsafe {
        // addlen / min_cap at the top of the range, with elemsize 0 so that the
        // allocation stays at sizeof(header)
        for (a, mc) in [
            (usize::MAX, 0usize),
            (0, usize::MAX / 64),
            (usize::MAX / 2, 0),
            (0, usize::MAX),
        ] {
            let c = (p.c.arrgrowf)(std::ptr::null_mut(), 0, a, mc);
            let r = (p.r.arrgrowf)(std::ptr::null_mut(), 0, a, mc);
            same_val(&format!("b03 arrgrowf(0,{a:#x},{mc:#x}) null"), c.is_null(), r.is_null());
            if !c.is_null() {
                same_val(
                    &format!("b03 arrgrowf(0,{a:#x},{mc:#x}) header"),
                    (header_of(c).length, header_of(c).capacity),
                    (header_of(r).length, header_of(r).capacity),
                );
                (p.c.arrfreef)(c);
                (p.r.arrfreef)(r);
            }
        }
        // keysize larger than the element: the C copies `keysize` bytes into a
        // (bigger) element, so use elemsize >= keysize but keysize > "real" key
        let mut m = DiffMap::lazy(&p, 32, 24, HM_BINARY, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let mut rng = Rng::new(0xB03);
        for i in 0..20 {
            let k = ka.add(&rng.bytes(24));
            m.put(k, &rng.bytes(32));
            m.check(&format!("b03 big keysize put#{i}"));
        }
        m.free();
        // hmfree_func with an absurd elemsize on a NULL map (must early-return)
        (p.c.hmfree_func)(std::ptr::null_mut(), usize::MAX);
        (p.r.hmfree_func)(std::ptr::null_mut(), usize::MAX);
    }
}

#[test]
fn b04_one_past_range() {
    let p = fresh_pair(0xB04);
    // STBDS_HM_* has only {0,1}; test -1, 0, 1, 2
    for mode in [-1, 0, 1, 2] {
        let keyrepr = if mode >= 1 { KeyRepr::Auto } else { KeyRepr::Inline };
        let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, mode, keyrepr);
        let mut ka = KeyArena::new();
        let mut rng = Rng::new(0xB04 ^ mode as i64 as u64);
        let mut keys = Vec::new();
        for i in 0..20 {
            let kb = if mode >= 1 {
                uniq_cstring(&mut rng, i)
            } else {
                uniq_bytes(&mut rng, i, KEYSIZE)
            };
            let k = ka.add(&kb);
            keys.push(k);
            let (a, b) = m.put(k, &rng.bytes(ELEMSIZE));
            same_val(&format!("b04 mode={mode} put temp"), a, b);
            m.check(&format!("b04 mode={mode} put"));
        }
        for k in keys.into_iter().rev() {
            let (a, b) = m.get(k);
            same_val(&format!("b04 mode={mode} get temp"), a, b);
            let (c, d) = m.del(k, 0);
            same_val(&format!("b04 mode={mode} del temp"), c, d);
            m.check(&format!("b04 mode={mode} del"));
        }
        m.free();
        std::mem::forget(ka);
    }
    // STBDS_SH_* has only {0..3}; test -1 .. 4 (and the truncation wrap)
    for shmode in [-1, 0, 1, 2, 3, 4, 256, 259, 260] {
        unsafe {
            let ct = (p.c.shmode_func)(ELEMSIZE, shmode);
            let rt = (p.r.shmode_func)(ELEMSIZE, shmode);
            same_val(
                &format!("b04 shmode={shmode} string.mode"),
                table_of(ct, ELEMSIZE).unwrap().string.mode,
                table_of(rt, ELEMSIZE).unwrap().string.mode,
            );
            same(
                &format!("b04 shmode={shmode}"),
                &snap_map(ct, ELEMSIZE, KeyRepr::Auto),
                &snap_map(rt, ELEMSIZE, KeyRepr::Auto),
            );
            (p.c.hmfree_func)((ct as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((rt as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
}

#[test]
fn b05_enum_out_of_range() {
    let p = fresh_pair(0xB05);
    let wild: &[c_int] = &[
        c_int::MIN,
        c_int::MIN + 1,
        -1000,
        -2,
        -1,
        2,
        3,
        0x100,
        1000,
        0x7fff_fffe,
        c_int::MAX,
    ];
    // 1. every `mode`-taking entry point
    unsafe {
        let mut key = *b"wild123\0";
        let kp = key.as_mut_ptr() as *mut c_void;
        for &mode in wild {
            // hmget_key_ts / hmget_key on a NULL map
            let mut tc: isize = 9;
            let mut tr: isize = 9;
            let a1 = (p.c.hmget_key_ts)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, &mut tc, mode);
            let a2 = (p.r.hmget_key_ts)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, &mut tr, mode);
            same_val(&format!("b05 hmget_key_ts mode={mode} temp"), tc, tr);
            same(
                &format!("b05 hmget_key_ts mode={mode}"),
                &snap_map(a1, ELEMSIZE, KeyRepr::Inline),
                &snap_map(a2, ELEMSIZE, KeyRepr::Inline),
            );
            (p.c.hmfree_func)((a1 as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((a2 as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);

            let b1 = (p.c.hmget_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, mode);
            let b2 = (p.r.hmget_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, mode);
            same(
                &format!("b05 hmget_key mode={mode}"),
                &snap_map(b1, ELEMSIZE, KeyRepr::Inline),
                &snap_map(b2, ELEMSIZE, KeyRepr::Inline),
            );
            (p.c.hmfree_func)((b1 as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((b2 as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);

            // hmdel_key on a NULL map
            same_val(
                &format!("b05 hmdel_key(NULL) mode={mode}"),
                (p.c.hmdel_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, 0, mode) as usize,
                (p.r.hmdel_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, 0, mode) as usize,
            );

            // hmput_key on a NULL map (creates the table with the derived
            // string.mode) then a full round trip
            let c1 = (p.c.hmput_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, mode);
            let c2 = (p.r.hmput_key)(std::ptr::null_mut(), ELEMSIZE, kp, KEYSIZE, mode);
            same_val(
                &format!("b05 hmput_key mode={mode} string.mode"),
                table_of(c1, ELEMSIZE).unwrap().string.mode,
                table_of(c2, ELEMSIZE).unwrap().string.mode,
            );
            same(
                &format!("b05 hmput_key mode={mode}"),
                &snap_map(c1, ELEMSIZE, KeyRepr::InlineKeyOnly(8)),
                &snap_map(c2, ELEMSIZE, KeyRepr::InlineKeyOnly(8)),
            );
            (p.c.hmfree_func)((c1 as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
            (p.r.hmfree_func)((c2 as usize - ELEMSIZE) as *mut c_void, ELEMSIZE);
        }
    }
    // 2. multi-op pipelines with a wild mode
    for &mode in wild {
        let keyrepr = if mode >= 1 { KeyRepr::Auto } else { KeyRepr::Inline };
        let mut m = DiffMap::lazy(&p, ELEMSIZE, KEYSIZE, mode, keyrepr);
        let mut ka = KeyArena::new();
        let mut rng = Rng::new(0xB05 ^ mode as i64 as u64);
        let mut keys = Vec::new();
        for i in 0..25 {
            let kb = if mode >= 1 {
                uniq_cstring(&mut rng, i)
            } else {
                uniq_bytes(&mut rng, i, KEYSIZE)
            };
            let k = ka.add(&kb);
            keys.push(k);
            let (a, b) = m.put(k, &rng.bytes(ELEMSIZE));
            same_val(&format!("b05 pipeline mode={mode} put temp"), a, b);
            m.check(&format!("b05 pipeline mode={mode} put"));
        }
        for k in &keys {
            let (a, b) = m.get(*k);
            same_val(&format!("b05 pipeline mode={mode} get temp"), a, b);
            let (c, d) = m.get_ts(*k);
            same_val(&format!("b05 pipeline mode={mode} get_ts temp"), c, d);
        }
        m.check(&format!("b05 pipeline mode={mode} gets"));
        for k in keys.iter().rev() {
            let (a, b) = m.del(*k, 0);
            same_val(&format!("b05 pipeline mode={mode} del temp"), a, b);
            m.check(&format!("b05 pipeline mode={mode} del"));
        }
        m.free();
        std::mem::forget(ka);
    }
    // 3. wild shmode combined with a wild mode (self-referential keys keep the
    //    `default:` memcpy storage dereferenceable for is_key_equal)
    for &shmode in wild {
        for &mode in &[HM_STRING, 2, c_int::MAX] {
            let mut m = DiffMap::shmode(&p, ELEMSIZE, KEYSIZE, mode, shmode, KeyRepr::Auto);
            let sk = SelfKeys::new(8);
            for (i, &k) in sk.keys.iter().enumerate() {
                let (a, b) = m.put(k, &[i as u8; 8]);
                same_val(&format!("b05 shmode={shmode} mode={mode} put temp"), a, b);
                m.check(&format!("b05 shmode={shmode} mode={mode} put#{i}"));
            }
            for (i, &k) in sk.keys.iter().enumerate().rev() {
                let (a, b) = m.del(k, 0);
                same_val(&format!("b05 shmode={shmode} mode={mode} del temp"), a, b);
                m.check(&format!("b05 shmode={shmode} mode={mode} del#{i}"));
            }
            m.free();
        }
    }
}

// ===========================================================================
// E23 vs E24, precisely: only the FIRST in-bucket duplicate scan updates
// `stbds_temp_key`; the wrap-around scan does not (lib.c:732-733 vs 747-751).
// ===========================================================================

#[test]
fn e23_e24_temp_key_asymmetry() {
    let p = pair();
    let elemsize = 16usize;
    let seed: usize = 0xC0FFEE_1234;
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
        // string keys whose probe positions are exactly 5,6,7 and a second one
        // at 5. The 4th key finds 5..7 occupied and wraps to slot 0.
        let mut counter = 1u32;
        let k5a = key_at_slot_str(&p.c, seed, 8, 5, &mut counter);
        let k6 = key_at_slot_str(&p.c, seed, 8, 6, &mut counter);
        let k7 = key_at_slot_str(&p.c, seed, 8, 7, &mut counter);
        let k5b = key_at_slot_str(&p.c, seed, 8, 5, &mut counter);
        for (k, want) in [(&k5a, 5usize), (&k6, 6), (&k7, 7), (&k5b, 5)] {
            let mut kk = k.clone();
            same_val(
                "e23/e24 rust probe slot agrees",
                effective_hash_string(&p.r, &mut kk, seed) & 7,
                want,
            );
        }

        let mut m = DiffMap::lazy(&p, elemsize, 8, HM_STRING, KeyRepr::Inline);
        let mut ka = KeyArena::new();
        let p5a = ka.add(&k5a);
        let p6 = ka.add(&k6);
        let p7 = ka.add(&k7);
        let p5b = ka.add(&k5b);

        for (i, k) in [p5a, p6, p7, p5b].into_iter().enumerate() {
            let (a, b) = m.put(k, &[i as u8; 8]);
            same_val(&format!("e23/e24 insert#{i} temp"), a, b);
            m.check(&format!("e23/e24 insert#{i}"));
        }
        // SH_DEFAULT was chosen implicitly -> element keys are caller pointers
        same_val(
            "e23/e24 string.mode",
            table_of(m.ct, elemsize).unwrap().string.mode,
            1u8,
        );
        // the 4th key must have wrapped into slot 0
        let bk = bucket(m.ct, elemsize, 0);
        same_val("e23/e24 bucket index parity", bk.index, bucket(m.rt, elemsize, 0).index);
        assert!(
            bk.index[0] >= 0,
            "the 4th key must have wrapped to slot 0: {:?}",
            bk.index
        );
        // after the last insert temp_key == p5b in both
        same_val(
            "e23/e24 temp_key after insert of k5b",
            (
                temp_key_ptr(m.ct, elemsize) as usize,
                temp_key_ptr(m.rt, elemsize) as usize,
            ),
            (p5b as usize, p5b as usize),
        );

        // E23: re-put k5a -> found in the FIRST scan -> temp_key updated to p5a
        let (a, b) = m.put(p5a, &[0xA0; 8]);
        same_val("e23 dup temp", a, b);
        m.check("e23 first-scan dup");
        same_val(
            "e23 first-scan dup updates temp_key",
            (
                temp_key_ptr(m.ct, elemsize) as usize,
                temp_key_ptr(m.rt, elemsize) as usize,
            ),
            (p5a as usize, p5a as usize),
        );

        // E24: re-put k5b -> found in the WRAP-AROUND scan -> temp_key NOT
        // updated, so it must still be p5a in BOTH libraries.
        let (c, d) = m.put(p5b, &[0xB0; 8]);
        same_val("e24 dup temp", c, d);
        m.check("e24 wrap-scan dup");
        same_val(
            "e24 wrap-scan dup leaves temp_key stale",
            (
                temp_key_ptr(m.ct, elemsize) as usize,
                temp_key_ptr(m.rt, elemsize) as usize,
            ),
            (p5a as usize, p5a as usize),
        );

        m.free();
        std::mem::forget(ka);
    }
}
