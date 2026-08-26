//! Phase B, CONFIGS.md rows 40-56: string-mode hash map across all four
//! `stbds_string_arena::mode` values and across `mode = 1` and `mode = 2`.

mod common;
use common::map::*;
use common::*;
use std::ffi::c_void;

/// Key buffers must stay alive for the whole test: `STBDS_SH_DEFAULT` stores
/// the caller's pointer verbatim.
struct Keys(Vec<Box<[u8]>>);
impl Keys {
    fn new() -> Self {
        Keys(Vec::new())
    }
    /// NUL-terminated key; `pad_to` zero-pads the allocation so that a
    /// `memcpy(elem, key, keysize)` (the `SH_NONE` path) reads only initialised
    /// bytes.
    fn add(&mut self, s: &[u8], pad_to: usize) -> *mut c_void {
        let mut v = s.to_vec();
        v.push(0);
        while v.len() < pad_to {
            v.push(0);
        }
        self.0.push(v.into_boxed_slice());
        self.0.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }
}

fn cfg_str(mode: i32) -> MapCfg {
    MapCfg {
        elemsize: 16,
        keysize: 8,
        keyoffset: 0,
        mode,
        valoffset: 8,
        valsize: 8,
        force_raw_snap: false,
    }
}

fn keyname(i: usize) -> Vec<u8> {
    format!("key_{i:05}").into_bytes()
}

// ---------------------------------------------------------------------------
// rows 40-42 — implicit table (string.mode becomes SH_DEFAULT)
// ---------------------------------------------------------------------------
#[test]
fn cfg40_41_42_string_default_mode() {
    for n in [1usize, 2, 5, 6, 7, 8, 11, 12, 13, 24, 25, 50] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = cfg_str(STBDS_HM_STRING);
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        unsafe {
            for i in 0..n {
                let k = keys.add(&keyname(i), 0);
                let before = m.hmlen(&format!("n={n} before {i}"));
                let idx = m.put(k, &(i as u64).to_le_bytes(), &format!("n={n} put {i}"));
                // fresh insert => string.mode == SH_DEFAULT wrote temp_key
                assert_eq!(m.hmlen(&format!("n={n} after {i}")), before + 1);
                m.check_temp_key(idx, &format!("n={n} put {i}"));
                // SH_DEFAULT stores the caller's pointer verbatim, identically
                // in both implementations
                assert_eq!(
                    rd_ptr(m.c.elem(idx), 0) as *mut c_void, k,
                    "C SH_DEFAULT must store the caller's key pointer"
                );
                assert_eq!(
                    rd_ptr(m.rs.elem(idx), 0) as *mut c_void, k,
                    "Rust SH_DEFAULT must store the caller's key pointer"
                );
                // string.mode really is SH_DEFAULT on both sides
                assert_eq_ctx(
                    string_mode(&m.c, cfg.elemsize),
                    string_mode(&m.rs, cfg.elemsize),
                    "string.mode",
                );
                assert_eq!(string_mode(&m.c, cfg.elemsize), Some(STBDS_SH_DEFAULT));
            }
            for i in 0..n {
                let k = keys.add(&keyname(i), 0);
                let idx = m.geti(k, &format!("n={n} get {i}"));
                assert!(idx >= 0, "n={n}: key {i} lost");
                m.check_val(idx, &format!("n={n} val {i}"));
            }
            for i in n..n + 8 {
                let k = keys.add(&keyname(i), 0);
                assert_eq!(m.geti(k, &format!("n={n} miss {i}")), -1);
            }
            m.free();
        }
    }

    // row 42 — 300 randomized keys with duplicates
    let (p, _g) = session(0x4242);
    let cfg = cfg_str(STBDS_HM_STRING);
    let mut m = MapPair::empty(p, cfg);
    let mut keys = Keys::new();
    let mut r = Rng::new(0x420042);
    unsafe {
        for i in 0..300usize {
            let n = r.range(0, 24);
            let mut s = r.cstring(n);
            s.pop();
            let k = keys.add(&s, 0);
            m.put(k, &r.u64().to_le_bytes(), &format!("rnd put {i}"));
        }
        for i in 0..100usize {
            let n = r.range(0, 24);
            let mut s = r.cstring(n);
            s.pop();
            let k = keys.add(&s, 0);
            m.geti(k, &format!("rnd get {i}"));
        }
        m.free();
    }
}

unsafe fn string_mode(m: &Map, elemsize: usize) -> Option<u8> {
    if m.t.is_null() {
        return None;
    }
    let h = (m.t as *mut u8).sub(elemsize).sub(HDR_SIZE);
    let tbl = rd_ptr(h, HDR_HASH_TABLE);
    if tbl.is_null() {
        None
    } else {
        Some(rd_u8(tbl.add(HI_STRING), ARENA_MODE))
    }
}

// ---------------------------------------------------------------------------
// row 43 — SH_STRDUP
// ---------------------------------------------------------------------------
#[test]
fn cfg43_string_strdup() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = cfg_str(STBDS_HM_STRING);
    let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_STRDUP as i32);
    let mut keys = Keys::new();
    let mut r = Rng::new(0x430043);
    unsafe {
        assert_eq!(string_mode(&m.c, cfg.elemsize), Some(STBDS_SH_STRDUP));
        assert_eq!(string_mode(&m.rs, cfg.elemsize), Some(STBDS_SH_STRDUP));
        m.check("shmode_func(SH_STRDUP) fresh");
        let mut live: Vec<Vec<u8>> = Vec::new();
        for i in 0..200usize {
            let n = r.range(1, 30);
            let mut s = r.cstring(n);
            s.pop();
            let k = keys.add(&s, 0);
            let before = m.hmlen(&format!("before {i}"));
            let idx = m.put(k, &r.u64().to_le_bytes(), &format!("strdup put {i}"));
            if m.hmlen(&format!("after {i}")) == before + 1 {
                live.push(s.clone());
                m.check_temp_key(idx, &format!("strdup put {i}"));
                // the stored key must be a COPY, not the caller's pointer
                assert_ne!(
                    rd_ptr(m.c.elem(idx), 0) as *mut c_void, k,
                    "C SH_STRDUP must store a copy"
                );
                assert_ne!(
                    rd_ptr(m.rs.elem(idx), 0) as *mut c_void, k,
                    "Rust SH_STRDUP must store a copy"
                );
                assert_eq!(cstr_bytes(rd_ptr(m.c.elem(idx), 0)), s);
                assert_eq!(cstr_bytes(rd_ptr(m.rs.elem(idx), 0)), s);
            }
        }
        for (i, s) in live.iter().enumerate() {
            let k = keys.add(s, 0);
            assert!(m.geti(k, &format!("strdup get {i}")) >= 0, "key {i} lost");
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// rows 44-45 — SH_ARENA, including keys larger than the current blocksize
// ---------------------------------------------------------------------------
#[test]
fn cfg44_45_string_arena() {
    for (tag, max_len) in [("short", 30usize), ("mixed", 900), ("huge", 2500)] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = cfg_str(STBDS_HM_STRING);
        let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_ARENA as i32);
        let mut keys = Keys::new();
        let mut r = Rng::new(0x440044 + max_len as u64);
        unsafe {
            assert_eq!(string_mode(&m.c, cfg.elemsize), Some(STBDS_SH_ARENA));
            m.check(&format!("{tag}: shmode_func(SH_ARENA) fresh"));
            let mut live: Vec<Vec<u8>> = Vec::new();
            for i in 0..200usize {
                let n = r.range(1, max_len);
                let mut s = r.cstring(n);
                s.pop();
                let k = keys.add(&s, 0);
                let before = m.hmlen(&format!("{tag} before {i}"));
                let idx = m.put(k, &r.u64().to_le_bytes(), &format!("{tag} arena put {i}"));
                if m.hmlen(&format!("{tag} after {i}")) == before + 1 {
                    live.push(s.clone());
                    m.check_temp_key(idx, &format!("{tag} arena put {i}"));
                    assert_eq!(cstr_bytes(rd_ptr(m.c.elem(idx), 0)), s);
                    assert_eq!(cstr_bytes(rd_ptr(m.rs.elem(idx), 0)), s);
                }
            }
            for (i, s) in live.iter().enumerate() {
                let k = keys.add(s, 0);
                assert!(m.geti(k, &format!("{tag} arena get {i}")) >= 0, "key {i} lost");
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 46 — SH_NONE with mode = STBDS_HM_STRING => `default:` memcpy branch
// ---------------------------------------------------------------------------
#[test]
fn cfg46_string_mode_none_memcpy() {
    let (p, _g) = session(INITIAL_HASH_SEED);
    // The element's "key field" holds raw *string bytes* (memcpy'd), NOT a
    // pointer, so it must never be dereferenced => force_raw_snap.
    let mut cfg = cfg_str(STBDS_HM_STRING);
    cfg.force_raw_snap = true;
    let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_NONE as i32);
    let mut keys = Keys::new();
    unsafe {
        assert_eq!(string_mode(&m.c, cfg.elemsize), Some(STBDS_SH_NONE));
        assert_eq!(string_mode(&m.rs, cfg.elemsize), Some(STBDS_SH_NONE));
        m.check("shmode_func(SH_NONE) fresh");
        // Only DISTINCT keys, and no lookups: a hash match would make the C
        // strcmp the raw string bytes *as a pointer*.  Distinct keys never
        // collide on a 64-bit hash in practice, so the compare never runs.
        for i in 0..30usize {
            let k = keys.add(&keyname(i), cfg.keysize + 1);
            m.put(k, &(i as u64).to_le_bytes(), &format!("SH_NONE put {i}"));
            // the element's first keysize bytes are the string's bytes
            let want = &keys.0.last().unwrap()[..cfg.keysize];
            let idx = m.c.temp();
            assert_eq!(
                std::slice::from_raw_parts(m.c.elem(idx), cfg.keysize),
                want,
                "C SH_NONE must memcpy the string bytes into the element"
            );
            assert_eq!(
                std::slice::from_raw_parts(m.rs.elem(idx), cfg.keysize),
                want,
                "Rust SH_NONE must memcpy the string bytes into the element"
            );
        }
        m.free();
    }
}

// ---------------------------------------------------------------------------
// row 47 — SH_STRDUP deletes (key freed; swap + string re-lookup)
// ---------------------------------------------------------------------------
#[test]
fn cfg47_string_strdup_delete() {
    for n in [1usize, 2, 3, 6, 7, 13, 20] {
        for del_pos in 0..n {
            let (p, _g) = session(INITIAL_HASH_SEED);
            let cfg = cfg_str(STBDS_HM_STRING);
            let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_STRDUP as i32);
            let mut keys = Keys::new();
            unsafe {
                for i in 0..n {
                    let k = keys.add(&keyname(i), 0);
                    m.put(k, &(i as u64).to_le_bytes(), &format!("n={n} put {i}"));
                }
                let k = keys.add(&keyname(del_pos), 0);
                let rc = m.del(k, &format!("n={n} del {del_pos}"));
                assert_eq!(rc, 1);
                assert_eq!(m.hmlen("after del"), n as isize - 1);
                let k = keys.add(&keyname(del_pos), 0);
                assert_eq!(m.geti(k, "get deleted"), -1);
                for i in 0..n {
                    if i == del_pos {
                        continue;
                    }
                    let k = keys.add(&keyname(i), 0);
                    assert!(
                        m.geti(k, &format!("n={n} survivor {i}")) >= 0,
                        "n={n}: survivor {i} lost after deleting {del_pos}"
                    );
                }
                m.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 48 — SH_ARENA deletes (no per-key free, arena retained)
// ---------------------------------------------------------------------------
#[test]
fn cfg48_string_arena_delete() {
    for n in [1usize, 3, 7, 13, 30] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = cfg_str(STBDS_HM_STRING);
        let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_ARENA as i32);
        let mut keys = Keys::new();
        unsafe {
            for i in 0..n {
                let k = keys.add(&keyname(i), 0);
                m.put(k, &(i as u64).to_le_bytes(), &format!("n={n} put {i}"));
            }
            for i in 0..n {
                let k = keys.add(&keyname(i), 0);
                let rc = m.del(k, &format!("n={n} del {i}"));
                assert_eq!(rc, 1);
                for j in (i + 1)..n {
                    let k = keys.add(&keyname(j), 0);
                    assert!(
                        m.geti(k, &format!("n={n} after del {i} survivor {j}")) >= 0,
                        "n={n}: key {j} lost"
                    );
                }
            }
            assert_eq!(m.hmlen("drained"), 0);
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 49-50 — long randomized churn in SH_DEFAULT and SH_STRDUP
// ---------------------------------------------------------------------------
fn churn(sh_mode: Option<u8>, seed: u64, ops: usize, keyspace: usize) {
    let (p, _g) = session(INITIAL_HASH_SEED);
    let cfg = cfg_str(STBDS_HM_STRING);
    let mut m = match sh_mode {
        None => MapPair::empty(p, cfg),
        Some(sm) => MapPair::with_shmode(p, cfg, sm as i32),
    };
    let mut keys = Keys::new();
    let mut r = Rng::new(seed);
    let mut model: std::collections::HashSet<usize> = std::collections::HashSet::new();
    unsafe {
        for op in 0..ops {
            let kn = r.below(keyspace);
            let k = keys.add(&keyname(kn), 0);
            match r.below(10) {
                0..=4 => {
                    m.put(k, &r.u64().to_le_bytes(), &format!("op {op} put {kn}"));
                    model.insert(kn);
                }
                5..=7 => {
                    let idx = m.geti(k, &format!("op {op} get {kn}"));
                    assert_eq!(idx >= 0, model.contains(&kn), "op {op}: presence of {kn}");
                }
                _ => {
                    let rc = m.del(k, &format!("op {op} del {kn}"));
                    assert_eq!(rc, model.contains(&kn) as isize, "op {op}: del({kn})");
                    model.remove(&kn);
                }
            }
            assert_eq!(
                m.hmlen(&format!("op {op}")),
                model.len() as isize,
                "op {op}: hmlen vs model"
            );
        }
        for kn in 0..keyspace {
            let k = keys.add(&keyname(kn), 0);
            assert_eq!(
                m.geti(k, &format!("final get {kn}")) >= 0,
                model.contains(&kn),
                "final: {kn}"
            );
        }
        m.free();
    }
}

#[test]
fn cfg49_string_default_churn() {
    churn(None, 0x490049, 1500, 40);
    churn(None, 0x49004a, 1500, 250);
}

#[test]
fn cfg50_string_strdup_churn() {
    churn(Some(STBDS_SH_STRDUP), 0x500050, 1500, 40);
    churn(Some(STBDS_SH_STRDUP), 0x500051, 1500, 250);
}

#[test]
fn cfg50b_string_arena_churn() {
    churn(Some(STBDS_SH_ARENA), 0x500052, 1500, 40);
    churn(Some(STBDS_SH_ARENA), 0x500053, 1500, 250);
}

#[test]
fn cfg50c_string_default_explicit_churn() {
    churn(Some(STBDS_SH_DEFAULT), 0x500054, 1500, 60);
}

// ---------------------------------------------------------------------------
// row 51 — mode = 2 (STBDS_HM_PTR_TO_STRING): `>= STBDS_HM_STRING` is true
// ---------------------------------------------------------------------------
#[test]
fn cfg51_mode2_is_string_mode() {
    for &mode in &[2i32, 3, 1000, i32::MAX] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = cfg_str(mode);
        let mut m = MapPair::empty(p, cfg);
        let mut keys = Keys::new();
        unsafe {
            for i in 0..20usize {
                let k = keys.add(&keyname(i), 0);
                let idx = m.put(k, &(i as u64).to_le_bytes(), &format!("mode={mode} put {i}"));
                m.check_temp_key(idx, &format!("mode={mode} put {i}"));
            }
            // an implicit table with mode >= 1 gets string.mode = SH_DEFAULT
            assert_eq_ctx(
                string_mode(&m.c, cfg.elemsize),
                string_mode(&m.rs, cfg.elemsize),
                &format!("mode={mode}: string.mode"),
            );
            assert_eq!(string_mode(&m.c, cfg.elemsize), Some(STBDS_SH_DEFAULT));
            for i in 0..25usize {
                let k = keys.add(&keyname(i), 0);
                let idx = m.geti(k, &format!("mode={mode} get {i}"));
                assert_eq!(idx >= 0, i < 20);
            }
            // Deletes must go in REVERSE insertion order so that
            // `old_index == final_index` and the swap-with-last block is
            // skipped.  For `mode >= 2` that block aborts in the C (its
            // re-lookup tests `mode == STBDS_HM_STRING` while `find_slot` tests
            // `mode >= STBDS_HM_STRING`, so it hashes the *address* of the key
            // pointer as a string and never finds the slot) -- that abort is
            // covered as its own row in tests/errors_fatal.rs.
            for i in (0..20usize).rev() {
                let k = keys.add(&keyname(i), 0);
                assert_eq!(m.del(k, &format!("mode={mode} del {i}")), 1);
            }
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 52 — mode = 2 with SH_STRDUP: hmdel_key does NOT free (mode != 1)
// ---------------------------------------------------------------------------
#[test]
fn cfg52_mode2_strdup_delete_does_not_free() {
    for &mode in &[2i32, 5, 1000] {
        let (p, _g) = session(INITIAL_HASH_SEED);
        let cfg = cfg_str(mode);
        let mut m = MapPair::with_shmode(p, cfg, STBDS_SH_STRDUP as i32);
        let mut keys = Keys::new();
        unsafe {
            for i in 0..15usize {
                let k = keys.add(&keyname(i), 0);
                m.put(k, &(i as u64).to_le_bytes(), &format!("mode={mode} put {i}"));
            }
            // deleting must leak the strdup'd key rather than free it, and both
            // implementations must agree on the resulting structure.
            // Reverse order => old_index == final_index => the swap-with-last
            // block (which aborts for mode >= 2) is skipped.
            for i in (0..15usize).rev() {
                let k = keys.add(&keyname(i), 0);
                assert_eq!(m.del(k, &format!("mode={mode} del {i}")), 1);
            }
            assert_eq!(m.hmlen("drained"), 0);
            // hmfree_func's strdup loop covers 1..length, and length is now 1,
            // so nothing is freed there either -- identical on both sides.
            m.free();
        }
    }
}

// ---------------------------------------------------------------------------
// row 53 — the wrap-around-half duplicate put where the C forgets temp_key
// ---------------------------------------------------------------------------

/// Exact replay of the `stbds_hmput_key` / `stbds_hm_find_slot` probe loop.
///
/// Returns `Some(true)` if slot `target` is reached by the SECOND (wrap-around,
/// `i < limit`) inner loop, `Some(false)` if by the FIRST (forward) inner loop,
/// and `None` if the probe hits an empty slot first.
///
/// `bhash[i]` is the stored hash of global slot `i`.  Distinct string keys never
/// share a 64-bit hash in practice, so `bhash[slot] == hash` implies "our key".
fn probe_half(hash: usize, sc: usize, bhash: &[usize], target: usize) -> Option<bool> {
    let mut pos = hash & (sc - 1);
    let mut step = BUCKET_LENGTH;
    for _ in 0..(4 * sc + 64) {
        let b = pos >> 3;
        for i in (pos & 7)..BUCKET_LENGTH {
            let slot = b * 8 + i;
            if bhash[slot] == hash {
                if slot == target {
                    return Some(false);
                }
            } else if bhash[slot] == 0 {
                return None;
            }
        }
        let limit = pos & 7;
        for i in 0..limit {
            let slot = b * 8 + i;
            if bhash[slot] == hash {
                if slot == target {
                    return Some(true);
                }
            } else if bhash[slot] == 0 {
                return None;
            }
        }
        pos = pos.wrapping_add(step);
        step = step.wrapping_add(BUCKET_LENGTH);
        pos &= sc - 1;
    }
    None
}

/// `stbds_hmput_key`'s FORWARD probe half sets `stbds_temp_key` when it finds an
/// existing key (lib.c:732-733) but the WRAP-AROUND half (lib.c:746-751) does
/// NOT.  Reproducing that omission is required.
///
/// To observe it deterministically the test uses `SH_DEFAULT`, where `temp_key`
/// holds a *caller-owned* pointer that is identical for both implementations,
/// and it keeps `used_count` strictly below `used_count_threshold` so the
/// duplicate put cannot trigger a rehash (which would leave `temp_key`
/// indeterminate).
#[test]
fn cfg53_wraparound_duplicate_put_temp_key() {
    let mut covered_wrap = 0usize;
    let mut covered_forward = 0usize;
    // (#entries, resulting slot_count, used_count_threshold)
    //   grows at used_count >= sc - (sc>>2): 6 @8, 12 @16, 24 @32, 48 @64
    for &n in &[5usize, 11, 23, 47] {
        for trial in 0..40usize {
            let (p, _g) = session(INITIAL_HASH_SEED.wrapping_mul(trial + 1).wrapping_add(n));
            let cfg = cfg_str(STBDS_HM_STRING);
            let mut m = MapPair::empty(p, cfg);
            let mut keys = Keys::new();
            unsafe {
                let base = trial * 100_000 + n * 1_000;
                let mut kptrs = Vec::new();
                for i in 0..n {
                    let k = keys.add(&keyname(base + i), 0);
                    kptrs.push((base + i, k));
                    m.put(k, &(i as u64).to_le_bytes(), &format!("n={n} put {i}"));
                }
                let tbl = rd_ptr(
                    (m.c.t as *mut u8).sub(cfg.elemsize).sub(HDR_SIZE),
                    HDR_HASH_TABLE,
                );
                let sc = rd_usize(tbl, HI_SLOT_COUNT);
                let used = rd_usize(tbl, HI_USED_COUNT);
                let uct = rd_usize(tbl, HI_USED_COUNT_THRESHOLD);
                assert!(
                    used < uct,
                    "n={n}: used_count {used} must stay below the threshold {uct} \
                     so a duplicate put cannot rehash"
                );
                let storage = rd_ptr(tbl, HI_STORAGE);
                let mut bhash = vec![0usize; sc];
                let mut bindex = vec![-1isize; sc];
                for b in 0..(sc >> 3) {
                    let bp = storage.add(b * BUCKET_SIZE);
                    for j in 0..BUCKET_LENGTH {
                        bhash[b * 8 + j] = rd_usize(bp, j * 8);
                        bindex[b * 8 + j] = rd_isize(bp, 64 + j * 8);
                    }
                }
                for slot in 0..sc {
                    if bindex[slot] < 0 || bhash[slot] < 2 {
                        continue;
                    }
                    let Some(via_wrap) = probe_half(bhash[slot], sc, &bhash, slot) else {
                        continue;
                    };
                    let kp = rd_ptr(m.c.elem(bindex[slot]), 0) as *mut c_void;
                    let Some(&(kn, _)) = kptrs.iter().find(|(_, q)| *q == kp) else {
                        continue;
                    };
                    // duplicate put of an EXISTING key
                    let dup = keys.add(&keyname(kn), 0);
                    let before = m.hmlen("before dup");
                    // `temp_key` BEFORE the put: in SH_DEFAULT mode this is a
                    // caller-owned pointer, so it is directly comparable across
                    // the two implementations.
                    let tk_before_c = m.c.temp_key_raw();
                    let tk_before_rs = m.rs.temp_key_raw();
                    assert_eq_ctx(
                        tk_before_c,
                        tk_before_rs,
                        &format!("n={n} trial {trial}: temp_key before dup put"),
                    );
                    let ci = m.put(
                        dup,
                        &0xabcdu64.to_le_bytes(),
                        &format!("n={n} trial {trial} dup put via_wrap={via_wrap}"),
                    );
                    assert_eq!(m.hmlen("after dup"), before, "dup put must not grow");
                    assert_eq!(ci, bindex[slot], "dup put must resolve to the same index");

                    let tk_after_c = m.c.temp_key_raw();
                    let tk_after_rs = m.rs.temp_key_raw();
                    // the differential property
                    assert_eq_ctx(
                        tk_after_c,
                        tk_after_rs,
                        &format!("n={n} trial {trial} via_wrap={via_wrap}: temp_key after dup put"),
                    );
                    assert_eq_ctx(
                        m.c.temp_key(),
                        m.rs.temp_key(),
                        &format!("n={n} trial {trial} via_wrap={via_wrap}: temp_key string"),
                    );
                    // THE QUIRK: the forward half refreshes temp_key to the
                    // element's stored key pointer; the wrap-around half leaves
                    // it completely untouched.
                    let elem_key = Some(m.c.elem_key_ptr(ci));
                    if via_wrap {
                        covered_wrap += 1;
                        if covered_wrap <= 5 {
                            eprintln!("WRAP #{covered_wrap}: ci={ci} tk_before_c={:?} tk_after_c={:?} tk_before_rs={:?} tk_after_rs={:?} elem_key_c={:?} elem_key_rs={:?}",
                                tk_before_c, tk_after_c, tk_before_rs, tk_after_rs,
                                m.c.elem_key_ptr(ci), m.rs.elem_key_ptr(ci));
                        }
                        assert_eq!(
                            tk_after_c, tk_before_c,
                            "n={n}: the C wrap-around half must leave temp_key untouched"
                        );
                        assert_eq!(
                            tk_after_rs, tk_before_rs,
                            "n={n}: the Rust wrap-around half must leave temp_key untouched"
                        );
                    } else {
                        covered_forward += 1;
                        assert_eq!(
                            tk_after_c, elem_key,
                            "n={n}: the C forward half MUST refresh temp_key"
                        );
                        assert_eq!(
                            tk_after_rs,
                            Some(m.rs.elem_key_ptr(ci)),
                            "n={n}: the Rust forward half MUST refresh temp_key"
                        );
                    }
                }
                m.free();
            }
        }
    }
    eprintln!("row53 coverage: wrap={covered_wrap} forward={covered_forward}");
    assert!(
        covered_wrap > 0,
        "never exercised the wrap-around duplicate-put half"
    );
    assert!(covered_forward > 0, "never exercised the forward half");
}

// ---------------------------------------------------------------------------
// rows 54-56 — hmfree_func for each string.mode
// ---------------------------------------------------------------------------
#[test]
fn cfg54_55_56_string_hmfree() {
    for sh in [
        None,
        Some(STBDS_SH_NONE),
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
    ] {
        for round in 0..6u64 {
            let (p, _g) = session(INITIAL_HASH_SEED);
            let mut cfg = cfg_str(STBDS_HM_STRING);
            if sh == Some(STBDS_SH_NONE) {
                cfg.force_raw_snap = true;
            }
            let mut m = match sh {
                None => MapPair::empty(p, cfg),
                Some(sm) => MapPair::with_shmode(p, cfg, sm as i32),
            };
            let mut keys = Keys::new();
            let mut r = Rng::new(0x540054 + round);
            unsafe {
                for i in 0..60usize {
                    // SH_NONE must only see distinct keys and no lookups
                    let kn = if sh == Some(STBDS_SH_NONE) {
                        i
                    } else {
                        r.below(30)
                    };
                    let k = keys.add(&keyname(kn), cfg.keysize + 1);
                    m.put(k, &r.u64().to_le_bytes(), &format!("{sh:?} put {i}"));
                    if sh != Some(STBDS_SH_NONE) && r.below(3) == 0 {
                        let k2 = keys.add(&keyname(r.below(30)), 0);
                        m.del(k2, &format!("{sh:?} del {i}"));
                    }
                }
                m.free();
                assert!(m.c.t.is_null() && m.rs.t.is_null());
            }
        }
    }
}
