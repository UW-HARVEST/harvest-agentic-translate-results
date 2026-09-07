//! Phase B rows 28-36, 38, 48-51, 53-54, 66: string-mode (`strcmp`) hash maps,
//! all four `string.mode` storage strategies, and out-of-range enum values.
mod common;
use common::*;
use std::ffi::c_void;

fn put_str(m: &mut MapPair, key: &[u8], mode: i32, tag: u64) {
    let kp = m.own(key);
    m.put(kp, 8, mode);
    unsafe { m.fill_tail(8, tag) };
}

/// SH_STRDUP / SH_ARENA copy the key, so the pointer the caller passed must not
/// be the one stored.
unsafe fn stored_key_ptr(m: &MapPair, which: u8, idx: isize) -> *const u8 {
    let base = if which == 0 { m.c } else { m.r };
    *((base as *mut u8).offset(idx * m.elemsize as isize) as *mut *const u8)
}

// ---------------------------------------------------------------------------
// rows 28-31: hmput_key with string / out-of-range / negative modes
// ---------------------------------------------------------------------------

// row 28: mode = 1 on a NULL map => string.mode becomes SH_DEFAULT
#[test]
fn row28_put_string_default_mode() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(28);
    for &n in &[1usize, 2, 6, 7, 8, 9, 40, 200] {
        seed_both(0x31415926);
        let mut m = MapPair::null(16, "put/str-default");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            let k = { let n = 1 + rng.below(20) as usize; rng.cstring(n) };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            put_str(&mut m, k, HM_STRING, i as u64);
            m.assert_same(&format!("n={n} put {i}"));
        }
        assert_eq!(m.snap_c().table.unwrap().arena_mode, SH_DEFAULT as u8);
        m.free();
    }
}

// row 29: duplicate string keys => update path; temp_key must be the *stored*
// pointer, not the caller's.
#[test]
fn row29_put_string_duplicates() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(29);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, shmode, "put/str-dupes");
        let keys: Vec<Vec<u8>> = (0..15).map(|_| { let n = 1 + rng.below(12) as usize; rng.cstring(n) }).collect();
        for round in 0..12u64 {
            for (i, k) in keys.iter().enumerate() {
                put_str(&mut m, k, HM_STRING, round * 100 + i as u64);
                m.assert_same(&format!("shmode={shmode} round={round} key={i}"));
            }
        }
        m.free();
    }
}

// row 30: out-of-range enum modes 2 / 3 / 99 / INT_MAX still take the string path
#[test]
fn row30_put_out_of_range_string_modes() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(30);
    for &mode in &[2i32, 3, 4, 99, 1000, i32::MAX] {
        seed_both(0x31415926);
        let mut m = MapPair::null(16, "put/oor-mode");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 40 {
            let k = { let n = 1 + rng.below(16) as usize; rng.cstring(n) };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            put_str(&mut m, k, mode, i as u64);
            m.assert_same(&format!("mode={mode} put {i}"));
        }
        // string path was taken => string.mode == SH_DEFAULT
        assert_eq!(m.snap_c().table.unwrap().arena_mode, SH_DEFAULT as u8);
        // lookups with the same out-of-range mode must agree
        for k in &keys {
            let kp = m.own(k);
            let (a, b) = m.get(kp, 8, mode);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.assert_same("after gets");
        m.free();
    }
}

// row 31: negative modes take the binary path
#[test]
fn row31_put_negative_modes_are_binary() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(31);
    for &mode in &[-1i32, -2, -1000, i32::MIN] {
        seed_both(0x31415926);
        let mut m = MapPair::null(16, "put/neg-mode");
        for i in 0..40u64 {
            let k = rng.bytes(8);
            let kp = m.own(&k);
            m.put(kp, 8, mode);
            unsafe { m.fill_tail(8, i) };
            m.assert_same(&format!("mode={mode} put {i}"));
        }
        // binary path => string.mode stays 0
        assert_eq!(m.snap_c().table.unwrap().arena_mode, 0);
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 32-34: shmode_func storage strategies
// ---------------------------------------------------------------------------

// row 32: SH_STRDUP -- keys are heap copies
#[test]
fn row32_shmode_strdup() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(32);
    for &n in &[1usize, 7, 8, 9, 60, 200] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, SH_STRDUP, "shmode/strdup");
        m.assert_same("fresh SH_STRDUP map");
        assert_eq!(m.snap_c().table.unwrap().arena_mode, SH_STRDUP as u8);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            let k = { let n = 1 + rng.below(30) as usize; rng.cstring(n) };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, HM_STRING);
            unsafe {
                m.fill_tail(8, i as u64);
                let t = m.temp_c();
                assert_ne!(
                    stored_key_ptr(&m, 0, t) as usize,
                    kp as usize,
                    "C SH_STRDUP must store a copy"
                );
                assert_ne!(
                    stored_key_ptr(&m, 1, m.temp_r()) as usize,
                    kp as usize,
                    "Rust SH_STRDUP must store a copy"
                );
            }
            m.assert_same(&format!("strdup n={n} put {i}"));
        }
        for k in &keys {
            let kp = m.own(k);
            let (a, b) = m.get(kp, 8, HM_STRING);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.assert_same("strdup after gets");
        m.free();
    }
}

// row 33: SH_ARENA -- keys live in the table's own string arena
#[test]
fn row33_shmode_arena() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(33);
    for &n in &[1usize, 8, 9, 60, 150] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, SH_ARENA, "shmode/arena");
        assert_eq!(m.snap_c().table.unwrap().arena_mode, SH_ARENA as u8);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            // mix short strings with ones that exceed the 512-byte first block
            let len = match rng.below(6) {
                0 => { let n = rng.below(3000) as usize; 600 + n },
                1 => { let n = rng.below(30) as usize; 500 + n },
                _ => { let n = rng.below(40) as usize; 1 + n },
            };
            let k = rng.cstring(len);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i as u64) };
            m.assert_same(&format!("arena n={n} put {i}"));
        }
        for k in &keys {
            let kp = m.own(k);
            let (a, b) = m.get(kp, 8, HM_STRING);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.assert_same("arena after gets");
        m.free();
    }
}

// row 34: SH_DEFAULT via shmode_func -- keys stored by pointer
#[test]
fn row34_shmode_default() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(34);
    seed_both(0x31415926);
    let mut m = MapPair::shmode(24, SH_DEFAULT, "shmode/default");
    assert_eq!(m.snap_c().table.unwrap().arena_mode, SH_DEFAULT as u8);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    while keys.len() < 120 {
        let k = { let n = 1 + rng.below(25) as usize; rng.cstring(n) };
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    for (i, k) in keys.iter().enumerate() {
        let kp = m.own(k);
        m.put(kp, 8, HM_STRING);
        unsafe {
            m.fill_tail(8, i as u64);
            assert_eq!(stored_key_ptr(&m, 0, m.temp_c()) as usize, kp as usize);
            assert_eq!(stored_key_ptr(&m, 1, m.temp_r()) as usize, kp as usize);
        }
        m.assert_same(&format!("default put {i}"));
    }
    m.free();
}

// row 35: SH_NONE table with mode=1 puts => the `default:` memcpy branch.
// Keys are stored as raw bytes even though the hash is computed with strcmp
// semantics, so only a handful of entries are used (a hash collision would make
// the C `strcmp` read the copied bytes as a pointer, which is UB in the C too).
#[test]
fn row35_shmode_none_with_string_puts() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(35);
    for trial in 0..40u64 {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, SH_NONE, "shmode/none+string");
        assert_eq!(m.snap_c().table.unwrap().arena_mode, 0);
        for i in 0..4u64 {
            let k = { let n = 1 + rng.below(30) as usize; rng.cstring(n) };
            let kp = m.own(&k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, trial * 10 + i) };
            m.assert_same(&format!("trial {trial} put {i}"));
        }
        m.free();
    }
}

// row 36: out-of-range shmode_func modes -- `(unsigned char) mode` truncation
// decides which switch arm `hmput_key` takes.
#[test]
fn row36_shmode_out_of_range() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(36);
    for &mode in &[4i32, 5, 99, 255, 256, 257, 258, 259, -1, -256, i32::MAX, i32::MIN] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, mode, "shmode/oor");
        let cs = m.snap_c();
        let rs = m.snap_r();
        assert_eq!(cs, rs, "fresh out-of-range shmode map (mode={mode})");
        assert_eq!(
            cs.table.unwrap().arena_mode,
            (mode as u32 & 0xff) as u8,
            "mode must be truncated to unsigned char (mode={mode})"
        );
        let truncated = (mode as u32 & 0xff) as i32;
        // Only the pointer-storing arms are safe to fill with several entries;
        // the memcpy arm stores raw bytes (see row 35).
        let n = if matches!(truncated, 1 | 2 | 3) { 30 } else { 4 };
        for i in 0..n {
            let k = { let n = 1 + rng.below(25) as usize; rng.cstring(n) };
            let kp = m.own(&k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i as u64) };
            m.assert_same(&format!("mode={mode} put {i}"));
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 38: hmget_key on string maps
// ---------------------------------------------------------------------------

#[test]
fn row38_get_string_modes() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(38);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, shmode, "get/string");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 90 {
            let k = { let n = 1 + rng.below(20) as usize; rng.cstring(n) };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i as u64) };
        }
        m.assert_same("populated");
        for k in &keys {
            let kp = m.own(k);
            let (a, b) = m.get(kp, 8, HM_STRING);
            assert_eq!(a, b, "present");
            assert!(a >= 0);
        }
        for _ in 0..300 {
            let mut k = { let n = 1 + rng.below(20) as usize; rng.cstring(n) };
            k.insert(0, b'Z'); // uppercase prefix never generated above
            let kp = m.own(&k);
            let (a, b) = m.get(kp, 8, HM_STRING);
            assert_eq!(a, b, "absent");
            assert_eq!(a, -1);
        }
        m.assert_same("after gets");
        // get_ts variant
        for k in &keys {
            let kp = m.own(k);
            let (a, b) = m.get_ts(kp, 8, HM_STRING);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.assert_same("after get_ts");
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 48-51, 53: hmdel_key on string maps
// ---------------------------------------------------------------------------

// row 48 / 49 / 51: delete head, middle, tail and all, for each storage mode
#[test]
fn row48_51_del_string_positions() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(48);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[1usize, 2, 3, 9, 40, 120] {
            seed_both(0x31415926);
            let mut m = MapPair::shmode(16, shmode, "del/string");
            let mut keys: Vec<Vec<u8>> = Vec::new();
            while keys.len() < n {
                let k = { let n = 1 + rng.below(24) as usize; rng.cstring(n) };
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            for (i, k) in keys.iter().enumerate() {
                let kp = m.own(k);
                m.put(kp, 8, HM_STRING);
                unsafe { m.fill_tail(8, i as u64) };
            }
            m.assert_same("populated");
            // delete in an order that hits head / middle / tail
            let mut order: Vec<usize> = (0..keys.len()).collect();
            for i in (1..order.len()).rev() {
                let j = rng.below(i as u64 + 1) as usize;
                order.swap(i, j);
            }
            for (step, &oi) in order.iter().enumerate() {
                let kp = m.own(&keys[oi]);
                m.del(kp, 8, 0, HM_STRING);
                unsafe {
                    assert_eq!(m.temp_c(), 1, "delete must report success");
                    assert_eq!(m.temp_r(), 1);
                }
                m.assert_same(&format!("shmode={shmode} n={n} step {step}"));
            }
            assert_eq!(m.snap_c().length, 1);
            m.free();
        }
    }
}

// row 48b: deleting an absent string key
#[test]
fn row48b_del_string_absent() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(481);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, shmode, "del/string-absent");
        for i in 0..30u64 {
            let k = { let n = 1 + rng.below(15) as usize; rng.cstring(n) };
            let kp = m.own(&k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i) };
        }
        m.assert_same("populated");
        for _ in 0..200 {
            let mut k = { let n = 1 + rng.below(15) as usize; rng.cstring(n) };
            k.insert(0, b'Q');
            let kp = m.own(&k);
            m.del(kp, 8, 0, HM_STRING);
            unsafe {
                assert_eq!(m.temp_c(), 0);
                assert_eq!(m.temp_r(), 0);
            }
            m.assert_same("absent delete");
        }
        m.free();
    }
}

// row 50: SH_STRDUP map deleted with mode = 2 (not exactly STBDS_HM_STRING)
// => the stored key is NOT freed. Only tail deletes are exercised: with a
// non-tail delete the C's relocation re-lookup hashes the raw pointer bytes and
// then trips `STBDS_ASSERT(slot >= 0)`, i.e. it aborts (see ERRORS.md row 25).
#[test]
fn row50_del_strdup_with_mode2_tail_only() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(50);
    for &n in &[1usize, 5, 9, 40] {
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, SH_STRDUP, "del/strdup-mode2");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < n {
            let k = { let n = 1 + rng.below(20) as usize; rng.cstring(n) };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i as u64) };
        }
        m.assert_same("populated");
        for i in (0..keys.len()).rev() {
            let kp = m.own(&keys[i]);
            m.del(kp, 8, 0, 2);
            unsafe {
                assert_eq!(m.temp_c(), 1);
                assert_eq!(m.temp_r(), 1);
            }
            m.assert_same(&format!("n={n} tail delete {i} with mode=2"));
        }
        m.free();
    }
}

// row 53: full random churn on a SH_STRDUP string map
#[test]
fn row53_random_churn_strdup() {
    let _s = session(0x31415926);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut rng = Rng::with(530 + shmode as u64);
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, shmode, "churn/string");
        let mut live: Vec<Vec<u8>> = Vec::new();
        for step in 0..500u64 {
            let op = rng.below(10);
            if op < 5 || live.is_empty() {
                // small key space so updates and collisions happen
                let k = { let n = 1 + rng.below(4) as usize; rng.cstring(n) };
                let kp = m.own(&k);
                m.put(kp, 8, HM_STRING);
                unsafe { m.fill_tail(8, step) };
                if !live.contains(&k) {
                    live.push(k);
                }
            } else if op < 8 {
                let i = rng.below(live.len() as u64) as usize;
                let kp = m.own(&live[i].clone());
                let (a, b) = m.get(kp, 8, HM_STRING);
                assert_eq!(a, b, "step {step}");
                assert!(a >= 0);
            } else {
                let i = rng.below(live.len() as u64) as usize;
                let k = live.remove(i);
                let kp = m.own(&k);
                m.del(kp, 8, 0, HM_STRING);
            }
            m.assert_same(&format!("shmode={shmode} step {step}"));
        }
        m.free();
    }
}

// row 54: hmfree_func across every storage mode, empty and populated
#[test]
fn row54_hmfree_all_modes() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(54);
    for &shmode in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[0usize, 1, 3, 40] {
            seed_both(0x31415926);
            let mut m = MapPair::shmode(16, shmode, "hmfree");
            let put_mode = if shmode == SH_NONE { HM_BINARY } else { HM_STRING };
            for i in 0..n {
                if put_mode == HM_BINARY {
                    let k = rng.bytes(8);
                    let kp = m.own(&k);
                    m.put(kp, 8, HM_BINARY);
                } else {
                    let k = { let n = 1 + rng.below(700) as usize; rng.cstring(n) };
                    let kp = m.own(&k);
                    m.put(kp, 8, HM_STRING);
                }
                unsafe { m.fill_tail(8, i as u64) };
            }
            m.assert_same(&format!("shmode={shmode} n={n} before free"));
            m.free();
        }
    }
    // maps that never got a hash table
    seed_both(0x31415926);
    let mut m = MapPair::null(16, "hmfree/tableless");
    m.put_default();
    m.assert_same("tableless");
    m.free();
}

// row 66: composed pipeline through the low-level entry points, SH_ARENA
#[test]
fn row66_pipeline_arena() {
    let _s = session(0x31415926);
    let mut rng = Rng::with(66);
    for &shmode in &[SH_ARENA, SH_STRDUP, SH_DEFAULT] {
        seed_both(rng.next_u64() as usize);
        let mut m = MapPair::shmode(16, shmode, "pipeline/string");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 200 {
            let k = { let n = 1 + rng.below(60) as usize; rng.cstring(n) };
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, i as u64) };
            m.assert_same("pipeline put");
        }
        for k in &keys {
            let kp = m.own(k);
            let (a, b) = m.get(kp, 8, HM_STRING);
            assert_eq!(a, b);
            assert!(a >= 0);
        }
        m.assert_same("pipeline gets");
        for k in &keys {
            let kp = m.own(k);
            m.del(kp, 8, 0, HM_STRING);
            m.assert_same("pipeline del");
        }
        m.free();
    }
}

// ERRORS.md row 48: `table->temp_key`.
//
// The C sets `temp_key` in the found-empty-slot path (every genuine insert) and
// in the *first* scan loop of the update path, but NOT in the wrap-around
// (second) scan loop -- there it keeps whatever was there before. Because a
// table rebuild (`stbds_make_hash_index`) allocates a table with an
// *uninitialised* `temp_key`, the stale value is raw heap garbage, so only the
// insert path has an observable, well-defined value. That is what is asserted
// here: after every insert of a brand-new key, both libraries must report the
// key text, for all three pointer-storing modes.
#[test]
fn temp_key_after_every_insert() {
    let _s = session(0x31415926);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut rng = Rng::with(7000 + shmode as u64);
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, shmode, "temp_key");
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 400 {
            let n = 1 + rng.below(40) as usize;
            let k = rng.cstring(n);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        for (step, k) in keys.iter().enumerate() {
            let kp = m.own(k);
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, step as u64) };
            m.assert_temp_key(k, &format!("shmode={shmode} insert {step}"));
            m.assert_same(&format!("shmode={shmode} insert {step}"));
        }
        m.free();
    }
}

// The same, but for the update path: re-putting an existing key must leave the
// map byte-identical between the two libraries (whether or not `temp_key` was
// refreshed, both must behave the same way).
#[test]
fn update_path_state_matches() {
    let _s = session(0x31415926);
    for &shmode in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let mut rng = Rng::with(7100 + shmode as u64);
        seed_both(0x31415926);
        let mut m = MapPair::shmode(16, shmode, "update-path");
        // short keys from a 3-letter alphabet => heavy bucket pressure, so the
        // wrap-around scan loop really is reached
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for a in b'a'..=b'c' {
            for b in b'a'..=b'c' {
                for c in b'a'..=b'c' {
                    keys.push(vec![a, b, c, 0]);
                }
            }
        }
        for step in 0..2000u64 {
            let i = rng.below(keys.len() as u64) as usize;
            let kp = m.own(&keys[i].clone());
            m.put(kp, 8, HM_STRING);
            unsafe { m.fill_tail(8, step) };
            m.assert_same(&format!("shmode={shmode} step={step}"));
            if step % 7 == 0 {
                let j = rng.below(keys.len() as u64) as usize;
                let kp2 = m.own(&keys[j].clone());
                m.del(kp2, 8, 0, HM_STRING);
                m.assert_same("after churn delete");
            }
        }
        m.free();
    }
}
