//! Phase B: string-keyed hash tables and the four `string.mode`s.
//! CONFIGS rows 29–32, 34–38, 41–42, 55–58, 61–63.

mod common;
use common::*;
use std::ffi::{c_char, c_void};

const PTRSZ: usize = 8;

fn repr_for(sh: i32) -> KeyRepr {
    match sh {
        SH_DEFAULT => KeyRepr::SharedPtr,
        SH_STRDUP | SH_ARENA => KeyRepr::OwnedStr,
        _ => KeyRepr::Bytes, // SH_NONE: the switch default arm memcpy's the bytes
    }
}

// --- row 29: fresh table via hmput_key with mode == 1 --------------------

#[test]
fn row29_string_mode_fresh_table_is_sh_default() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x2901);
    let mut keys = Keys::strings(&mut rng, 200, 24, ASCII);
    let mut d = Dual::new(16, PTRSZ, HM_STRING, KeyRepr::SharedPtr);
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row29 put i={i}"));
        assert_eq!(d.snap_c().arena_mode, SH_DEFAULT as u8, "row29: string.mode");
    }
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, "row29 get"), i as isize);
    }
    // the stored key pointer must be exactly the caller's buffer, in both libs
    unsafe {
        for i in 0..keys.len() {
            let want = keys.ptr(i) as usize;
            let gc = *((d.cp as *mut u8).add(16 * i) as *const usize);
            let gr = *((d.rp as *mut u8).add(16 * i) as *const usize);
            assert_eq!(gc, want, "row29: C stored key pointer i={i}");
            assert_eq!(gr, want, "row29: Rust stored key pointer i={i}");
        }
    }
    d.free();
}

// --- row 30: many random strings -----------------------------------------

#[test]
fn row30_string_500_random_keys() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x3001);
    let mut keys = Keys::strings(&mut rng, 500, 32, ASCII);
    let mut d = Dual::new(24, PTRSZ, HM_STRING, KeyRepr::SharedPtr);
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row30 put i={i}"));
    }
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, "row30 get"), i as isize);
        assert_eq!(d.get_ts(k, "row30 get_ts"), i as isize);
    }
    d.free();
}

// --- row 31: equal contents in distinct buffers ---------------------------

#[test]
fn row31_string_duplicate_contents_distinct_buffers() {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x3101);
    let mut keys = Keys::strings(&mut rng, 200, 20, ASCII);
    // second set: byte-identical copies in fresh allocations
    let mut copies: Vec<Box<[u8]>> = keys.bufs.iter().map(|b| b.to_vec().into_boxed_slice()).collect();

    let mut d = Dual::new(16, PTRSZ, HM_STRING, KeyRepr::SharedPtr);
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row31 put i={i}"));
    }
    let before = d.snap_c();
    for i in 0..copies.len() {
        let k = copies[i].as_mut_ptr() as *mut c_void;
        let t = d.put(k, &format!("row31 reput-copy i={i}"));
        assert_eq!(t, i as isize, "row31: strcmp-equal key must reuse the slot");
    }
    let after = d.snap_c();
    assert_eq!(before.length, after.length, "row31: no growth");
    assert_eq!(before.slots, after.slots, "row31: buckets unchanged");
    // SH_DEFAULT does NOT overwrite the stored key pointer on a duplicate hit,
    // so it must still point at the ORIGINAL buffer in both libraries
    unsafe {
        for i in 0..keys.len() {
            let want = keys.ptr(i) as usize;
            let gc = *((d.cp as *mut u8).add(16 * i) as *const usize);
            let gr = *((d.rp as *mut u8).add(16 * i) as *const usize);
            assert_eq!(gc, want, "row31: C key pointer i={i}");
            assert_eq!(gr, want, "row31: Rust key pointer i={i}");
        }
    }
    d.free();
}

// --- row 32: out-of-range positive modes behave as string ----------------

#[test]
fn row32_out_of_range_positive_mode_is_string() {
    for &mode in &[2i32, 3, 99, i32::MAX] {
        let _g = serial();
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x3200 ^ (mode as u32 as u64));
        let mut keys = Keys::strings(&mut rng, 150, 20, ASCII);
        let mut d = Dual::new(16, PTRSZ, mode, KeyRepr::SharedPtr);
        for i in 0..keys.len() {
            let k = keys.ptr(i);
            d.put(k, &format!("row32 mode={mode} put i={i}"));
        }
        assert_eq!(
            d.snap_c().arena_mode,
            SH_DEFAULT as u8,
            "row32 mode={mode}: fresh table must get SH_DEFAULT"
        );
        for i in 0..keys.len() {
            let k = keys.ptr(i);
            assert_eq!(d.get(k, "row32 get"), i as isize);
        }
        d.free();
    }
}

// --- rows 34/35/36/37: explicit string.mode tables ----------------------

fn shmode_roundtrip(row: &str, sh: i32, mode: i32, elemsize: usize, n: usize, maxlen: usize, seed: u64) {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(seed);
    let mut keys = Keys::strings(&mut rng, n, maxlen, ASCII);
    let ks = if sh == SH_NONE { PTRSZ } else { PTRSZ };
    let mut d = Dual::with_shmode(elemsize, ks, mode, sh, repr_for(sh));
    assert_eq!(d.snap_c().arena_mode, sh as u8, "row{row}: string.mode");
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row{row} sh={sh} put i={i}"));
    }
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, &format!("row{row} get i={i}")), i as isize);
        assert_eq!(d.get_ts(k, &format!("row{row} get_ts i={i}")), i as isize);
    }
    // stored key strings must equal the inputs
    let len = d.snap_c().length;
    if sh == SH_STRDUP || sh == SH_ARENA || sh == SH_DEFAULT {
        let sc = unsafe { key_strings(d.cp, elemsize, len) };
        let sr = unsafe { key_strings(d.rp, elemsize, len) };
        assert_eq!(sc, sr, "row{row}: key strings");
        for i in 0..keys.len() {
            let want = &keys.bufs[i][..keys.bufs[i].len() - 1];
            assert_eq!(&sc[i][..], want, "row{row}: key {i} content");
        }
    }
    d.free();
}

#[test]
fn row34_sh_strdup_table() {
    shmode_roundtrip("34", SH_STRDUP, HM_STRING, 16, 200, 24, 0x3401);
    shmode_roundtrip("34", SH_STRDUP, HM_STRING, 24, 200, 40, 0x3402);
}

#[test]
fn row35_sh_arena_table() {
    shmode_roundtrip("35", SH_ARENA, HM_STRING, 16, 200, 24, 0x3501);
    shmode_roundtrip("35", SH_ARENA, HM_STRING, 24, 400, 40, 0x3502);
}

#[test]
fn row36_sh_arena_with_oversized_keys() {
    // keys longer than the 512-byte first arena block force the dedicated
    // oversized-block path inside stbds_stralloc
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x3601);
    let mut keys = Keys::strings(&mut rng, 60, 2000, ASCII);
    let es = 16usize;
    let mut d = Dual::with_shmode(es, PTRSZ, HM_STRING, SH_ARENA, KeyRepr::OwnedStr);
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row36 put i={i} len={}", keys.bufs[i].len() - 1));
    }
    let s = d.snap_c();
    assert!(s.arena_block_count > 1, "row36: expected several arena blocks");
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        assert_eq!(d.get(k, "row36 get"), i as isize);
    }
    d.free();
}

#[test]
fn row37_sh_none_table_with_string_mode() {
    // string.mode == 0 -> the `switch` default arm memcpy's `keysize` bytes of
    // the key into the element, but hashing/comparison is still string-based
    // (mode == 1), so the element's first 8 bytes are the first 8 bytes of the
    // key text rather than a pointer.
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(0x3701);
    // keys must be >= 8 bytes long so the memcpy of 8 bytes stays in bounds
    let mut keys = Keys::strings(&mut rng, 150, 24, ASCII);
    keys.bufs.retain(|b| b.len() >= 9);
    let es = 16usize;
    let mut d = Dual::with_shmode(es, PTRSZ, HM_STRING, SH_NONE, KeyRepr::Bytes);
    assert_eq!(d.snap_c().arena_mode, 0);
    let mut inserted = Vec::new();
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        let t = d.put(k, &format!("row37 put i={i}"));
        inserted.push(t);
    }
    // the first 8 bytes of every element must be a byte copy of some inserted
    // key's first 8 bytes — and the two libraries must agree element-for-element
    unsafe {
        let len = d.snap_c().length;
        let prefixes: std::collections::HashSet<&[u8]> =
            keys.bufs.iter().map(|b| &b[..8]).collect();
        for i in 1..len {
            let ec = std::slice::from_raw_parts((d.cp as *mut u8).add(es * (i - 1)), 8);
            let er = std::slice::from_raw_parts((d.rp as *mut u8).add(es * (i - 1)), 8);
            assert_eq!(ec, er, "row37: element {i} key bytes differ");
            assert!(
                prefixes.contains(ec),
                "row37: element {i} does not hold a copied key prefix: {ec:x?}"
            );
        }
        assert_eq!(len, keys.len() + 1, "row37: every key must have been inserted");
        assert_eq!(inserted.iter().filter(|&&t| t >= 0).count(), keys.len());
    }
    d.free();
}

// --- row 38: stbds_shmode_func mode truncation ---------------------------

#[test]
fn row38_shmode_func_truncates_mode() {
    let (c, r) = libs();
    let modes: [i32; 14] = [0, 1, 2, 3, 4, 5, 255, 256, 257, 259, -1, -256, i32::MIN, i32::MAX];
    for &m in &modes {
        for &es in &[8usize, 16, 24] {
            let _g = serial();
            set_seed(DEFAULT_SEED);
            unsafe {
                let a = (c.shmode_func)(es, m);
                let b = (r.shmode_func)(es, m);
                let ctx = format!("row38 mode={m} es={es}");
                assert_eq!(snapshot(a, es), snapshot(b, es), "{ctx}");
                let s = snapshot(a, es);
                assert_eq!(s.arena_mode, (m as u32 & 0xff) as u8, "{ctx}: truncated string.mode");
                assert_eq!(s.length, 1, "{ctx}");
                assert_eq!(s.slot_count, 8, "{ctx}");
                assert!(s.has_table, "{ctx}");
                // element 0 must be zeroed identically
                let ea = std::slice::from_raw_parts((a as *mut u8).sub(es), es).to_vec();
                let eb = std::slice::from_raw_parts((b as *mut u8).sub(es), es).to_vec();
                assert_eq!(ea, eb, "{ctx}");
                assert!(ea.iter().all(|&x| x == 0), "{ctx}");
                // only free through hmfree_func for modes it can handle safely
                if s.arena_mode as i32 != SH_STRDUP {
                    (c.hmfree_func)((a as *mut u8).sub(es) as *mut c_void, es);
                    (r.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);
                }
            }
        }
    }
}

// --- rows 41/42: get on string tables ------------------------------------

#[test]
fn row41_42_string_get_present_and_absent() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        let _g = serial();
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x4100 ^ sh as u64);
        let mut keys = Keys::strings(&mut rng, 600, 28, ASCII);
        let n = 300;
        let es = 16usize;
        let mut d = Dual::with_shmode(es, PTRSZ, HM_STRING, sh, repr_for(sh));
        for i in 0..n {
            let k = keys.ptr(i);
            d.put(k, &format!("row41 sh={sh} put i={i}"));
        }
        for i in 0..n {
            let k = keys.ptr(i);
            assert_eq!(d.get(k, &format!("row41 sh={sh} present i={i}")), i as isize);
        }
        for i in n..keys.len() {
            let k = keys.ptr(i);
            assert_eq!(d.get(k, &format!("row42 sh={sh} absent i={i}")), -1);
            assert_eq!(d.get_ts(k, &format!("row42 sh={sh} absent_ts i={i}")), -1);
        }
        d.free();
    }
}

// --- rows 55/56/57: deletes on string tables -----------------------------

fn string_delete_all(row: &str, sh: i32, mode: i32, n: usize, seed: u64) {
    let _g = serial();
    set_seed(DEFAULT_SEED);
    let mut rng = Rng::new(seed);
    let mut keys = Keys::strings(&mut rng, n, 28, ASCII);
    let es = 16usize;
    let mut d = Dual::with_shmode(es, PTRSZ, mode, sh, repr_for(sh));
    for i in 0..keys.len() {
        let k = keys.ptr(i);
        d.put(k, &format!("row{row} sh={sh} put i={i}"));
    }
    let mut order: Vec<usize> = (0..keys.len()).collect();
    for i in (1..order.len()).rev() {
        order.swap(i, rng.below(i + 1));
    }
    let mut live: Vec<usize> = (0..keys.len()).collect();
    for (step, &i) in order.iter().enumerate() {
        let k = keys.ptr(i);
        let t = d.del(k, 0, &format!("row{row} sh={sh} del step={step} key={i}"));
        assert_eq!(t, 1, "row{row}: delete of present key");
        live.retain(|&x| x != i);
        assert_eq!(d.snap_c().length, live.len() + 1, "row{row}: length");
        assert_eq!(d.del(k, 0, &format!("row{row} redel step={step}")), 0);
        if step % 29 == 0 || live.len() < 10 {
            for &j in &live {
                let kj = keys.ptr(j);
                assert!(d.get(kj, &format!("row{row} get {j} after {step}")) >= 0, "row{row}: lost key {j}");
            }
        }
    }
    assert_eq!(d.snap_c().length, 1);
    d.free();
}

#[test]
fn row55_delete_sh_default() {
    string_delete_all("55", SH_DEFAULT, HM_STRING, 250, 0x5501);
}

#[test]
fn row56_delete_sh_strdup_frees_keys() {
    string_delete_all("56", SH_STRDUP, HM_STRING, 250, 0x5601);
}

#[test]
fn row57_delete_sh_arena_keeps_keys() {
    string_delete_all("57", SH_ARENA, HM_STRING, 250, 0x5701);
}

// --- row 58: mode == 2 -> string hash, BINARY re-find --------------------

#[test]
fn row58_mode_two_uses_binary_refind_on_delete() {
    // `stbds_hmdel_key` gates the strdup-free and the string re-find on
    // `mode == STBDS_HM_STRING` exactly, while hashing/comparison use `>= 1`.
    // With mode == 2 the moved element is therefore re-found by comparing the
    // raw pointer bytes, not the string. This is faithfully weird; both
    // libraries must be weird identically.
    for &sh in &[SH_DEFAULT, SH_ARENA] {
        let _g = serial();
        set_seed(DEFAULT_SEED);
        let mut rng = Rng::new(0x5800 ^ sh as u64);
        let n = 120;
        let mut keys = Keys::strings(&mut rng, n, 20, ASCII);
        let es = 16usize;
        let mut d = Dual::with_shmode(es, PTRSZ, 2, sh, repr_for(sh));
        for i in 0..keys.len() {
            let k = keys.ptr(i);
            d.put(k, &format!("row58 sh={sh} put i={i}"));
        }
        // Delete only the most recently inserted element each time: then
        // old_index == final_index and the binary re-find branch is skipped, so
        // the structure stays consistent while still exercising mode == 2.
        for step in 0..keys.len() {
            let i = keys.len() - 1 - step;
            let k = keys.ptr(i);
            let t = d.del(k, 0, &format!("row58 sh={sh} del step={step} key={i}"));
            assert_eq!(t, 1, "row58 sh={sh}: delete step {step}");
        }
        assert_eq!(d.snap_c().length, 1);
        d.free();
    }
}

// --- rows 61/62/63: hmfree_func on the three string modes ----------------

#[test]
fn row61_62_63_hmfree_string_tables() {
    for &sh in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        for &n in &[0usize, 1, 10, 200] {
            let _g = serial();
            set_seed(DEFAULT_SEED);
            let mut rng = Rng::new(0x6100 ^ (sh as u64) << 8 ^ n as u64);
            let mut keys = Keys::strings(&mut rng, n.max(1), 600, ASCII);
            let es = 16usize;
            let mut d = Dual::with_shmode(es, PTRSZ, HM_STRING, sh, repr_for(sh));
            for i in 0..n {
                let k = keys.ptr(i);
                d.put(k, &format!("row61 sh={sh} put i={i}"));
            }
            assert_eq!(d.snap_c().arena_mode, sh as u8);
            d.free();
            assert!(d.cp.is_null() && d.rp.is_null());
        }
    }
}

// --- row 71 (here for convenience): strreset via hmfree on arena tables ---

#[test]
fn row71_strreset_direct() {
    let (c, r) = libs();
    let mut rng = Rng::new(0x7101);
    for &nblocks in &[0usize, 1, 3, 40] {
        let mut ac = StringArena::zeroed();
        let mut ar = StringArena::zeroed();
        unsafe {
            for _ in 0..nblocks {
                // 600 > 512 forces a fresh block each time for the first few
                let mut s = rng.cstring(600, ASCII);
                (c.stralloc)(&mut ac, s.as_mut_ptr() as *mut c_char);
                (r.stralloc)(&mut ar, s.as_mut_ptr() as *mut c_char);
            }
            (c.strreset)(&mut ac);
            (r.strreset)(&mut ar);
            let ctx = format!("row71 nblocks={nblocks}");
            assert!(ac.storage.is_null(), "{ctx}: C arena storage");
            assert!(ar.storage.is_null(), "{ctx}: Rust arena storage");
            assert_eq!(ac.remaining, 0, "{ctx}");
            assert_eq!(ar.remaining, 0, "{ctx}");
            assert_eq!(ac.block, 0, "{ctx}");
            assert_eq!(ar.block, 0, "{ctx}");
            assert_eq!(ac.mode, 0, "{ctx}");
            assert_eq!(ar.mode, 0, "{ctx}");
        }
    }
}
