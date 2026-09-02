//! Phase B — CONFIGS.md rows 16..38 and 42: the hash-map pipeline
//! (`shmode_func`, `hmput_default`, `hmput_key`, `hmget_key`, `hmget_key_ts`,
//! `hmdel_key`, `hmfree_func`) driven through the low-level exports only.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Two maps (one per implementation) driven in lockstep.
struct Dual<'a> {
    c: Map<'a>,
    rs: Map<'a>,
    elemsize: usize,
    keysize: usize,
    mode: c_int,
    /// element layout has a `char *` at offset 0
    string_key: bool,
    /// dump element 0..8 as a C string (false when the element does not really
    /// hold a `char *`, e.g. the SH_NONE `default:` memcpy branch)
    dump_str: bool,
    /// where the "value" bytes live inside an element
    voff: usize,
}

impl<'a> Dual<'a> {
    fn empty(p: &'a Pair, elemsize: usize, keysize: usize, mode: c_int) -> Dual<'a> {
        let string_key = mode >= STBDS_HM_STRING;
        Dual {
            c: Map::empty(&p.c, elemsize),
            rs: Map::empty(&p.rs, elemsize),
            elemsize,
            keysize,
            mode,
            string_key,
            dump_str: string_key,
            voff: if string_key { 8 } else { keysize },
        }
    }

    fn shmode(p: &'a Pair, elemsize: usize, keysize: usize, mode: c_int, sh: c_int) -> Dual<'a> {
        let string_key = mode >= STBDS_HM_STRING;
        Dual {
            c: Map::shmode(&p.c, elemsize, sh),
            rs: Map::shmode(&p.rs, elemsize, sh),
            elemsize,
            keysize,
            mode,
            string_key,
            dump_str: string_key && sh >= STBDS_SH_DEFAULT,
            voff: if string_key { 8 } else { keysize },
        }
    }

    unsafe fn dump(&self) -> (String, String) {
        (self.c.dump(self.dump_str), self.rs.dump(self.dump_str))
    }

    unsafe fn check(&self, ctx: &str) {
        let (a, b) = self.dump();
        assert_eq!(a, b, "state divergence @ {ctx}");
    }

    /// `hmput`/`shput`: insert the key, then write `value` into the element.
    unsafe fn put(&mut self, key: *mut c_void, value: &[u8], ctx: &str) -> isize {
        let vlen = value.len().min(self.elemsize - self.voff);
        let len_before = self.arr_len();
        let tc = self.c.put_kv(key, self.keysize, self.mode, self.voff, &value[..vlen]);
        let tr = self.rs.put_kv(key, self.keysize, self.mode, self.voff, &value[..vlen]);
        assert_eq!(tc, tr, "hmput_key temp differs @ {ctx}");
        // `temp_key` is only written by the SH_DEFAULT/STRDUP/ARENA branches, and
        // only a *fresh* insert guarantees the pointer is still live (a re-put
        // taken through the wrap-around probe loop leaves it stale by design —
        // see the comment in the Rust translation of `hmput_key`).
        if self.string_key && self.sh_writes_temp_key() && self.arr_len() > len_before {
            assert_eq!(
                temp_key_str(hash_to_arr(self.c.p, self.elemsize)),
                temp_key_str(hash_to_arr(self.rs.p, self.elemsize)),
                "temp_key differs @ {ctx}"
            );
        }
        self.check(ctx);
        tc
    }

    unsafe fn arr_len(&self) -> usize {
        if self.c.p.is_null() {
            0
        } else {
            (*header(hash_to_arr(self.c.p, self.elemsize))).length
        }
    }

    /// True when the table's arena mode makes `hmput_key` write `temp_key`.
    unsafe fn sh_writes_temp_key(&self) -> bool {
        if self.c.p.is_null() {
            return false;
        }
        let h = &*header(hash_to_arr(self.c.p, self.elemsize));
        if h.hash_table.is_null() {
            return false;
        }
        let m = (*(h.hash_table as *const HashIndex)).string.mode as c_int;
        m == STBDS_SH_DEFAULT || m == STBDS_SH_STRDUP || m == STBDS_SH_ARENA
    }

    unsafe fn get(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let tc = self.c.get(key, self.keysize, self.mode);
        let tr = self.rs.get(key, self.keysize, self.mode);
        assert_eq!(tc, tr, "hmget_key temp differs @ {ctx}");
        self.check(ctx);
        tc
    }

    unsafe fn get_ts(&mut self, key: *mut c_void, ctx: &str) -> isize {
        let tc = self.c.get_ts(key, self.keysize, self.mode);
        let tr = self.rs.get_ts(key, self.keysize, self.mode);
        assert_eq!(tc, tr, "hmget_key_ts temp differs @ {ctx}");
        self.check(ctx);
        tc
    }

    unsafe fn del(&mut self, key: *mut c_void, keyoffset: usize, ctx: &str) -> isize {
        let tc = self.c.del(key, self.keysize, keyoffset, self.mode);
        let tr = self.rs.del(key, self.keysize, keyoffset, self.mode);
        assert_eq!(tc, tr, "hmdel_key temp differs @ {ctx}");
        assert_eq!(
            self.c.p.is_null(),
            self.rs.p.is_null(),
            "hmdel_key NULL-ness differs @ {ctx}"
        );
        self.check(ctx);
        tc
    }

    unsafe fn free(&mut self) {
        self.c.free();
        self.rs.free();
    }
}

/// Row 16 — `shmode_func` for every `SH_*` mode × several element sizes.
#[test]
fn cfg_16_shmode_func_modes() {
    for &sh in &[STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &e in &[8usize, 12, 16, 24, 40] {
            let (p, _g) = begin(DEFAULT_SEED);
            unsafe {
                let cp = (p.c.shmode_func)(e, sh);
                let rp = (p.rs.shmode_func)(e, sh);
                assert_eq!(
                    dump_map(cp, e, sh >= STBDS_SH_DEFAULT),
                    dump_map(rp, e, sh >= STBDS_SH_DEFAULT),
                    "shmode_func({e},{sh})"
                );
                (p.c.hmfree_func)(hash_to_arr(cp, e), e);
                (p.rs.hmfree_func)(hash_to_arr(rp, e), e);
            }
        }
    }
}

/// Row 17 — `hmput_default` in each of its three states.
#[test]
fn cfg_17_hmput_default_states() {
    for &e in &[8usize, 16, 24] {
        let (p, _g) = begin(DEFAULT_SEED);
        unsafe {
            // state 1: a == NULL
            let mut cp = (p.c.hmput_default)(std::ptr::null_mut(), e);
            let mut rp = (p.rs.hmput_default)(std::ptr::null_mut(), e);
            assert_eq!(dump_map(cp, e, false), dump_map(rp, e, false), "null e={e}");

            // state 3: length != 0 -> no-op (must be byte-identical *and* the
            // same pointer)
            let cp2 = (p.c.hmput_default)(cp, e);
            let rp2 = (p.rs.hmput_default)(rp, e);
            assert_eq!(cp2, cp, "C hmput_default must be a no-op");
            assert_eq!(rp2, rp, "Rust hmput_default must be a no-op");
            cp = cp2;
            rp = rp2;
            assert_eq!(dump_map(cp, e, false), dump_map(rp, e, false), "noop e={e}");

            // state 2: length == 0
            (*header(hash_to_arr(cp, e))).length = 0;
            (*header(hash_to_arr(rp, e))).length = 0;
            let cp3 = (p.c.hmput_default)(cp, e);
            let rp3 = (p.rs.hmput_default)(rp, e);
            assert_eq!(dump_map(cp3, e, false), dump_map(rp3, e, false), "len0 e={e}");

            (p.c.hmfree_func)(hash_to_arr(cp3, e), e);
            (p.rs.hmfree_func)(hash_to_arr(rp3, e), e);
        }
    }
}

/// Row 18 — binary mode, one insert, across the `keysize` axis.
#[test]
fn cfg_18_binary_keysizes() {
    let mut rng = Rng::new(SEED ^ 18);
    for &ks in &[1usize, 2, 3, 4, 7, 8, 9, 16, 32] {
        for _ in 0..24 {
            let (p, _g) = begin(DEFAULT_SEED);
            let e = ks + 8;
            let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
            let mut key = rng.bytes(ks);
            let val = rng.bytes(8);
            unsafe {
                d.put(key.as_mut_ptr() as *mut c_void, &val, &format!("ks={ks}"));
                let t = d.get(key.as_mut_ptr() as *mut c_void, "get-present");
                assert_eq!(t, 0, "single entry must live at index 0");
                let mut absent = rng.bytes(ks);
                if absent == key {
                    absent[0] ^= 0xff;
                }
                d.get(absent.as_mut_ptr() as *mut c_void, "get-absent");
                d.free();
            }
        }
    }
}

/// Row 19 — binary mode, growth across `used_count_threshold` several times.
#[test]
fn cfg_19_binary_growth() {
    let mut rng = Rng::new(SEED ^ 19);
    for &n in &[0usize, 1, 2, 5, 6, 7, 8, 9, 50, 300] {
        let (p, _g) = begin(DEFAULT_SEED);
        let (e, ks) = (16usize, 8usize);
        let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for i in 0..n {
                let mut k = rng.bytes(ks);
                while keys.iter().any(|x| *x == k) {
                    k = rng.bytes(ks);
                }
                let v = rng.bytes(8);
                d.put(k.as_mut_ptr() as *mut c_void, &v, &format!("n={n} i={i}"));
                keys.push(k);
            }
            // every key must still be findable at the same index in both
            for (i, k) in keys.iter_mut().enumerate() {
                let t = d.get(k.as_mut_ptr() as *mut c_void, &format!("n={n} lookup {i}"));
                assert!(t >= 0, "key {i} vanished (n={n})");
            }
            d.free();
        }
    }
}

/// Row 20 — duplicate keys re-put (the "update existing" probe path).
#[test]
fn cfg_20_binary_duplicate_puts() {
    let mut rng = Rng::new(SEED ^ 20);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for step in 0..600usize {
            if keys.is_empty() || rng.below(3) != 0 {
                let mut k = rng.bytes(ks);
                while keys.iter().any(|x| *x == k) {
                    k = rng.bytes(ks);
                }
                let v = rng.bytes(8);
                d.put(k.as_mut_ptr() as *mut c_void, &v, &format!("new step={step}"));
                keys.push(k);
            } else {
                let i = rng.below(keys.len());
                let v = rng.bytes(8);
                let mut k = keys[i].clone();
                d.put(k.as_mut_ptr() as *mut c_void, &v, &format!("dup step={step}"));
            }
        }
        d.free();
    }
}

fn keyptr(v: &mut Vec<u8>) -> *mut c_void {
    v.as_mut_ptr() as *mut c_void
}

/// Rows 21..24 — string modes. `sh` is the arena mode of the table:
/// `None` means "let `hmput_key` create the table itself" (→ `SH_DEFAULT`).
fn string_mode_body(sh: Option<c_int>, mode: c_int, n: usize, big: bool, tag: &str) {
    let mut rng = Rng::new(SEED ^ (n as u64) ^ (tag.len() as u64) << 8);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let mut d = match sh {
        None => Dual::empty(p, e, ks, mode),
        Some(sh) => Dual::shmode(p, e, ks, mode, sh),
    };
    // Keys must stay alive: SH_DEFAULT stores the caller's pointer verbatim.
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..n {
            let len = if big && i % 7 == 3 {
                600 + rng.below(1200)
            } else {
                1 + rng.below(20)
            };
            let mut k = rng.cstring(len);
            while keys.iter().any(|x| *x == k) {
                k = rng.cstring(len);
            }
            keys.push(k);
            let v = rng.bytes(8);
            let kp = keyptr(keys.last_mut().unwrap());
            d.put(kp, &v, &format!("{tag} put {i}"));
        }
        for i in 0..keys.len() {
            let mut k = keys[i].clone();
            let t = d.get(keyptr(&mut k), &format!("{tag} get {i}"));
            assert!(t >= 0, "{tag}: key {i} vanished");
            let t2 = d.get_ts(keyptr(&mut k), &format!("{tag} get_ts {i}"));
            assert_eq!(t, t2);
        }
        // absent keys
        for i in 0..8 {
            let mut k = rng.cstring_range(1, 30);
            k.insert(0, b'~');
            d.get(keyptr(&mut k), &format!("{tag} absent {i}"));
        }
        d.free();
    }
}

/// Row 21 — `SH_DEFAULT` (table auto-created by `hmput_key` with `mode >= 1`).
#[test]
fn cfg_21_string_sh_default() {
    for n in [0usize, 1, 2, 7, 8, 50, 300] {
        string_mode_body(None, STBDS_HM_STRING, n, false, "sh_default");
    }
}

/// Row 22 — `SH_STRDUP`.
#[test]
fn cfg_22_string_sh_strdup() {
    for n in [0usize, 1, 2, 7, 8, 50, 300] {
        string_mode_body(Some(STBDS_SH_STRDUP), STBDS_HM_STRING, n, false, "sh_strdup");
    }
}

/// Row 23 — `SH_ARENA`, including keys larger than the arena block size.
#[test]
fn cfg_23_string_sh_arena() {
    for n in [0usize, 1, 2, 7, 8, 50, 300] {
        string_mode_body(Some(STBDS_SH_ARENA), STBDS_HM_STRING, n, true, "sh_arena");
    }
}

/// Row 24 — `SH_NONE` table + string `mode`: `hash_string`/`strcmp` are used for
/// probing but the `default:` branch `memcpy`s `keysize` raw bytes *of the
/// string itself* into the element. Any subsequent lookup therefore reinterprets
/// string bytes as a `char *` and dereferences it — a wild read that the C
/// performs too. Only the insert path (where a full 64-bit hash collision is
/// required to reach `stbds_is_key_equal`) can be compared in-process; the
/// crashing lookup is compared as a crash-equivalence case in Phase C
/// (`err_24_sh_none_string_lookup_crash`).
#[test]
fn cfg_24_sh_none_with_string_mode() {
    let mut rng = Rng::new(SEED ^ 24);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let mut d = Dual::shmode(p, e, ks, STBDS_HM_STRING, STBDS_SH_NONE);
    assert!(!d.dump_str, "elements do not hold real char* in this mode");
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..40usize {
            let n = 1 + rng.below(30);
            let mut k = rng.cstring(n);
            while keys.iter().any(|x| *x == k) {
                k = rng.cstring(n);
            }
            keys.push(k);
            let v = rng.bytes(8);
            let kp = keyptr(keys.last_mut().unwrap());
            d.put(kp, &v, &format!("sh_none put {i}"));
        }
        // Frees only the array + index: SH_NONE never sweeps element keys.
        d.free();
    }
}

/// Row 25 — binary lookups across table sizes and every probe path.
#[test]
fn cfg_25_binary_lookups_all_sizes() {
    let mut rng = Rng::new(SEED ^ 25);
    for &n in &[1usize, 6, 7, 12, 24, 48, 96, 200] {
        let (p, _g) = begin(DEFAULT_SEED);
        let (e, ks) = (16usize, 8usize);
        let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for _ in 0..n {
                let mut k = rng.bytes(ks);
                while keys.iter().any(|x| *x == k) {
                    k = rng.bytes(ks);
                }
                let v = rng.bytes(8);
                d.put(keyptr(&mut k), &v, &format!("n={n}"));
                keys.push(k);
            }
            for i in 0..keys.len() {
                let mut k = keys[i].clone();
                assert!(d.get(keyptr(&mut k), &format!("n={n} present {i}")) >= 0);
            }
            for i in 0..64 {
                let mut k = rng.bytes(ks);
                d.get_ts(keyptr(&mut k), &format!("n={n} random {i}"));
            }
            d.free();
        }
    }
}

/// Row 26 — string lookups in every arena mode, present and absent.
#[test]
fn cfg_26_string_lookups_all_arena_modes() {
    for sh in [
        None,
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
    ] {
        for n in [1usize, 8, 40] {
            string_mode_body(sh, STBDS_HM_STRING, n, false, "lookups");
        }
    }
}

/// Row 27 — `hmget_key_ts` bootstrap from `NULL`, then on a table-less array.
#[test]
fn cfg_27_hmget_ts_bootstrap() {
    let mut rng = Rng::new(SEED ^ 27);
    for &e in &[8usize, 16, 24] {
        let (p, _g) = begin(DEFAULT_SEED);
        let ks = 8usize;
        let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
        unsafe {
            for i in 0..20usize {
                let mut k = rng.bytes(ks);
                let t = d.get_ts(keyptr(&mut k), &format!("e={e} boot {i}"));
                assert_eq!(t, -1, "table-less lookups must report -1");
            }
            for i in 0..20usize {
                let mut k = rng.bytes(ks);
                let t = d.get(keyptr(&mut k), &format!("e={e} boot-get {i}"));
                assert_eq!(t, -1);
            }
            d.free();
        }
    }
}

/// Rows 28..33 — deletion. `del_last` picks the most recently inserted key
/// (`old_index == final_index`, no memmove), otherwise a random middle one.
fn delete_body(sh: Option<c_int>, mode: c_int, n: usize, del_last: bool, tag: &str) {
    let mut rng = Rng::new(SEED ^ 0x5a ^ (n as u64) << 3);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let string = mode >= STBDS_HM_STRING;
    let mut d = match sh {
        None => Dual::empty(p, e, ks, mode),
        Some(sh) => Dual::shmode(p, e, ks, mode, sh),
    };
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..n {
            let mut k = if string {
                rng.cstring_range(1, 16)
            } else {
                rng.bytes(ks)
            };
            while keys.iter().any(|x| *x == k) {
                k = if string {
                    rng.cstring_range(1, 16)
                } else {
                    rng.bytes(ks)
                };
            }
            let v = rng.bytes(8);
            keys.push(k);
            let kp = keyptr(keys.last_mut().unwrap());
            d.put(kp, &v, &format!("{tag} put {i}"));
        }
        let mut order: Vec<usize> = (0..keys.len()).collect();
        if del_last {
            order.reverse();
        } else {
            // Fisher-Yates with the fixed-seed PRNG.
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
        }
        for (step, &i) in order.iter().enumerate() {
            let mut k = keys[i].clone();
            let t = d.del(keyptr(&mut k), 0, &format!("{tag} del {step} (key {i})"));
            assert_eq!(t, 1, "{tag}: delete of a present key must report 1");
            // deleting again must be a no-op
            let t2 = d.del(keyptr(&mut k), 0, &format!("{tag} redel {step}"));
            assert_eq!(t2, 0, "{tag}: re-delete must report 0");
        }
        d.free();
    }
}

/// Row 28 — binary, delete the last live element each time.
#[test]
fn cfg_28_del_binary_last() {
    for n in [1usize, 2, 8, 40, 200] {
        delete_body(None, STBDS_HM_BINARY, n, true, "bin-last");
    }
}

/// Row 29 — binary, delete middle elements (memmove + index patch).
#[test]
fn cfg_29_del_binary_middle() {
    for n in [2usize, 3, 9, 40, 200] {
        delete_body(None, STBDS_HM_BINARY, n, false, "bin-mid");
    }
}

/// Row 30 — shrink path: fill big, then delete until the table shrinks.
#[test]
fn cfg_30_del_shrink() {
    let mut rng = Rng::new(SEED ^ 30);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for _ in 0..400usize {
            let mut k = rng.bytes(ks);
            while keys.iter().any(|x| *x == k) {
                k = rng.bytes(ks);
            }
            let v = rng.bytes(8);
            d.put(keyptr(&mut k), &v, "shrink fill");
            keys.push(k);
        }
        let slots_before = {
            let t = (*header(hash_to_arr(d.c.p, e))).hash_table as *const HashIndex;
            (*t).slot_count
        };
        for (i, k) in keys.iter_mut().enumerate() {
            d.del(keyptr(k), 0, &format!("shrink del {i}"));
        }
        let slots_after = {
            let t = (*header(hash_to_arr(d.c.p, e))).hash_table as *const HashIndex;
            (*t).slot_count
        };
        assert!(
            slots_after < slots_before,
            "expected the table to shrink ({slots_before} -> {slots_after})"
        );
        d.free();
    }
}

/// Row 31 — tombstone rebuild: put/delete churn on a size-stable table.
#[test]
fn cfg_31_del_tombstone_rebuild() {
    let mut rng = Rng::new(SEED ^ 31);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
    let mut live: Vec<Vec<u8>> = Vec::new();
    let mut rebuilds = 0usize;
    unsafe {
        // Keep the population near constant so slot_count stays put while
        // tombstones accumulate past tombstone_count_threshold.
        for _ in 0..60usize {
            let mut k = rng.bytes(ks);
            while live.iter().any(|x| *x == k) {
                k = rng.bytes(ks);
            }
            let v = rng.bytes(8);
            d.put(keyptr(&mut k), &v, "tomb fill");
            live.push(k);
        }
        for step in 0..1200usize {
            let before = {
                let t = (*header(hash_to_arr(d.c.p, e))).hash_table as *const HashIndex;
                ((*t).slot_count, (*t).tombstone_count)
            };
            let i = rng.below(live.len());
            let mut old = live.swap_remove(i);
            d.del(keyptr(&mut old), 0, &format!("tomb del {step}"));
            let after = {
                let t = (*header(hash_to_arr(d.c.p, e))).hash_table as *const HashIndex;
                ((*t).slot_count, (*t).tombstone_count)
            };
            // A same-size rebuild clears every tombstone.
            if after.0 == before.0 && after.1 == 0 && before.1 > 0 {
                rebuilds += 1;
            }
            let mut k = rng.bytes(ks);
            while live.iter().any(|x| *x == k) {
                k = rng.bytes(ks);
            }
            let v = rng.bytes(8);
            d.put(keyptr(&mut k), &v, &format!("tomb put {step}"));
            live.push(k);
        }
        assert!(
            rebuilds > 0,
            "the tombstone-rebuild branch (lib.c:860) was never reached"
        );
        d.free();
    }
}

/// Row 32 — string deletion with `SH_STRDUP` (frees the duplicated key).
#[test]
fn cfg_32_del_string_strdup() {
    for n in [1usize, 2, 9, 60] {
        delete_body(Some(STBDS_SH_STRDUP), STBDS_HM_STRING, n, true, "strdup-last");
        delete_body(Some(STBDS_SH_STRDUP), STBDS_HM_STRING, n, false, "strdup-mid");
    }
}

/// Row 33 — string deletion with `SH_DEFAULT` / `SH_ARENA` (no key free).
#[test]
fn cfg_33_del_string_default_arena() {
    for n in [1usize, 2, 9, 60] {
        delete_body(None, STBDS_HM_STRING, n, true, "def-last");
        delete_body(None, STBDS_HM_STRING, n, false, "def-mid");
        delete_body(Some(STBDS_SH_ARENA), STBDS_HM_STRING, n, true, "arena-last");
        delete_body(Some(STBDS_SH_ARENA), STBDS_HM_STRING, n, false, "arena-mid");
    }
}

/// Row 34 — non-zero `keyoffset` passed to `hmdel_key` (binary mode).
#[test]
fn cfg_34_del_nonzero_keyoffset() {
    let mut rng = Rng::new(SEED ^ 34);
    for &keyoffset in &[1usize, 4, 8] {
        let (p, _g) = begin(DEFAULT_SEED);
        let (e, ks) = (24usize, 8usize);
        let mut d = Dual::empty(p, e, ks, STBDS_HM_BINARY);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for _ in 0..40usize {
                let mut k = rng.bytes(ks);
                while keys.iter().any(|x| *x == k) {
                    k = rng.bytes(ks);
                }
                let v = rng.bytes(16);
                d.put(keyptr(&mut k), &v, "koff fill");
                keys.push(k);
            }
            for i in 0..keys.len() {
                let mut k = keys[i].clone();
                d.del(keyptr(&mut k), keyoffset, &format!("koff={keyoffset} del {i}"));
            }
            d.free();
        }
    }
}

/// Rows 35..37 — randomized put/get/del op mixture over the full pipeline.
fn fuzz_body(sh: Option<c_int>, mode: c_int, ops: usize, seed_mix: u64, tag: &str) {
    let mut rng = Rng::new(SEED ^ seed_mix);
    let (p, _g) = begin(DEFAULT_SEED);
    let (e, ks) = (16usize, 8usize);
    let string = mode >= STBDS_HM_STRING;
    let mut d = match sh {
        None => Dual::empty(p, e, ks, mode),
        Some(sh) => Dual::shmode(p, e, ks, mode, sh),
    };
    // SH_DEFAULT keeps the caller's pointer, so keys must outlive the map.
    let mut arena: Vec<Box<Vec<u8>>> = Vec::new();
    let mut live: Vec<usize> = Vec::new();
    unsafe {
        for step in 0..ops {
            match rng.below(10) {
                0..=4 => {
                    // insert a fresh key
                    let mut k = if string {
                        rng.cstring_range(1, 24)
                    } else {
                        rng.bytes(ks)
                    };
                    while arena.iter().any(|x| ***x == *k) {
                        k = if string {
                            rng.cstring_range(1, 24)
                        } else {
                            rng.bytes(ks)
                        };
                    }
                    arena.push(Box::new(k));
                    let idx = arena.len() - 1;
                    let v = rng.bytes(8);
                    let kp = arena[idx].as_mut_ptr() as *mut c_void;
                    d.put(kp, &v, &format!("{tag} step={step} insert"));
                    live.push(idx);
                }
                5 => {
                    // re-put an existing key
                    if !live.is_empty() {
                        let i = live[rng.below(live.len())];
                        let v = rng.bytes(8);
                        let kp = arena[i].as_mut_ptr() as *mut c_void;
                        d.put(kp, &v, &format!("{tag} step={step} reput"));
                    }
                }
                6 | 7 => {
                    // delete a live key
                    if !live.is_empty() {
                        let j = rng.below(live.len());
                        let i = live.swap_remove(j);
                        let kp = arena[i].as_mut_ptr() as *mut c_void;
                        let t = d.del(kp, 0, &format!("{tag} step={step} del"));
                        assert_eq!(t, 1, "{tag}: delete of live key must report 1");
                    }
                }
                8 => {
                    // lookup a live key
                    if !live.is_empty() {
                        let i = live[rng.below(live.len())];
                        let kp = arena[i].as_mut_ptr() as *mut c_void;
                        let t = d.get(kp, &format!("{tag} step={step} get-live"));
                        assert!(t >= 0, "{tag}: live key must be found");
                    }
                }
                _ => {
                    // lookup a random (usually absent) key
                    let mut k = if string {
                        rng.cstring_range(1, 24)
                    } else {
                        rng.bytes(ks)
                    };
                    d.get_ts(keyptr(&mut k), &format!("{tag} step={step} get-rand"));
                }
            }
        }
        d.free();
    }
}

/// Row 35 — binary-mode fuzz.
#[test]
fn cfg_35_fuzz_binary() {
    fuzz_body(None, STBDS_HM_BINARY, 1500, 35, "fuzz-bin");
}

/// Row 36 — string-mode fuzz with `SH_STRDUP`.
#[test]
fn cfg_36_fuzz_string_strdup() {
    fuzz_body(Some(STBDS_SH_STRDUP), STBDS_HM_STRING, 1500, 36, "fuzz-strdup");
}

/// Row 37 — string-mode fuzz with `SH_ARENA`.
#[test]
fn cfg_37_fuzz_string_arena() {
    fuzz_body(Some(STBDS_SH_ARENA), STBDS_HM_STRING, 1500, 37, "fuzz-arena");
}

/// Row 38 — `hmfree_func` after 0 / 1 / many inserts, in every arena mode.
#[test]
fn cfg_38_hmfree_all_modes() {
    let mut rng = Rng::new(SEED ^ 38);
    for &sh in &[STBDS_SH_NONE, STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &n in &[0usize, 1, 2, 30] {
            let (p, _g) = begin(DEFAULT_SEED);
            let (e, ks) = (16usize, 8usize);
            let mode = if sh >= STBDS_SH_DEFAULT {
                STBDS_HM_STRING
            } else {
                STBDS_HM_BINARY
            };
            let mut d = Dual::shmode(p, e, ks, mode, sh);
            let mut keys: Vec<Vec<u8>> = Vec::new();
            unsafe {
                for _ in 0..n {
                    let mut k = if mode >= STBDS_HM_STRING {
                        rng.cstring_range(1, 20)
                    } else {
                        rng.bytes(ks)
                    };
                    while keys.iter().any(|x| *x == k) {
                        k = if mode >= STBDS_HM_STRING {
                            rng.cstring_range(1, 20)
                        } else {
                            rng.bytes(ks)
                        };
                    }
                    keys.push(k);
                    let v = rng.bytes(8);
                    let kp = keyptr(keys.last_mut().unwrap());
                    d.put(kp, &v, &format!("free sh={sh} n={n}"));
                }
                d.check(&format!("pre-free sh={sh} n={n}"));
                d.free();
                assert!(d.c.p.is_null() && d.rs.p.is_null());
            }
        }
    }
}

/// Row 42 — struct sizes/offsets must agree between the two implementations.
/// Verified indirectly but decisively: the whole test-suite reads C-produced
/// state through the Rust-side `#[repr(C)]` mirrors and vice versa, so a layout
/// mismatch would surface everywhere. This test pins the absolute numbers.
#[test]
fn cfg_42_abi_layout() {
    use std::mem::{align_of, size_of};
    assert_eq!(size_of::<ArrayHeader>(), 32);
    assert_eq!(align_of::<ArrayHeader>(), 8);
    assert_eq!(size_of::<StringArena>(), 24);
    assert_eq!(size_of::<HashBucket>(), 128);
    assert_eq!(size_of::<HashIndex>(), 104);
    assert_eq!(size_of::<StringBlock>(), 16);

    // `hmput_key`'s bootstrap writes `length = 1`; if the header layout differed
    // between the libraries this cross-read would not agree.
    let (p, _g) = begin(DEFAULT_SEED);
    unsafe {
        let e = 16usize;
        let cp = (p.c.shmode_func)(e, STBDS_SH_ARENA);
        let rp = (p.rs.shmode_func)(e, STBDS_SH_ARENA);
        let ch = &*header(hash_to_arr(cp, e));
        let rh = &*header(hash_to_arr(rp, e));
        assert_eq!(ch.length, 1);
        assert_eq!(rh.length, 1);
        assert_eq!(ch.capacity, rh.capacity);
        let ct = &*(ch.hash_table as *const HashIndex);
        let rt = &*(rh.hash_table as *const HashIndex);
        assert_eq!(ct.slot_count, 8);
        assert_eq!(rt.slot_count, 8);
        assert_eq!(ct.string.mode, STBDS_SH_ARENA as u8);
        assert_eq!(rt.string.mode, STBDS_SH_ARENA as u8);
        // storage must be 64-byte aligned and inside the allocation
        assert_eq!(ct.storage as usize % 64, 0);
        assert_eq!(rt.storage as usize % 64, 0);
        (p.c.hmfree_func)(hash_to_arr(cp, e), e);
        (p.rs.hmfree_func)(hash_to_arr(rp, e), e);
    }
}
