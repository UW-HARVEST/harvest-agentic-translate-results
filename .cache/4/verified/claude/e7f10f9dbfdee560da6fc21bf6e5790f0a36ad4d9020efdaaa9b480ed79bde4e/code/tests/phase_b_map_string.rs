//! Phase B — CONFIGS.md rows 35-41, 46, 56-58, 61, 72, 73 (string half).
//! String-key hash maps: `STBDS_SH_DEFAULT`, `STBDS_SH_STRDUP`,
//! `STBDS_SH_ARENA` and the `default:`/`SH_NONE` fallback.
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Owns stable, never-moved key buffers. `STBDS_SH_DEFAULT` tables store the
/// raw pointer, so the buffers must outlive the maps.
struct Keys {
    bufs: Vec<Box<[u8]>>,
}

impl Keys {
    fn new() -> Keys {
        Keys { bufs: Vec::new() }
    }
    /// NUL-terminated buffer, padded to at least `min_len` bytes so that the
    /// `default:` `memcpy(dst, key, keysize)` branch never reads out of bounds.
    fn add(&mut self, text: &[u8], min_len: usize) -> *mut c_char {
        let mut v: Vec<u8> = text.to_vec();
        v.push(0);
        while v.len() < min_len {
            v.push(0);
        }
        let mut b = v.into_boxed_slice();
        let p = b.as_mut_ptr() as *mut c_char;
        self.bufs.push(b);
        p
    }
}

fn rand_text(rng: &mut Rng, len: usize) -> Vec<u8> {
    let mut v = rng.cstring(len);
    v.pop(); // drop the NUL, `Keys::add` re-adds it
    v
}

/// Distinct random strings.
fn distinct_texts(rng: &mut Rng, n: usize, max_len: usize) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    while out.len() < n {
        let len = rng.below(max_len + 1);
        let t = rand_text(rng, len);
        if !out.contains(&t) {
            out.push(t);
        }
    }
    out
}

struct SDual<'a> {
    s: &'a Session,
    lay: Layout,
    c: *mut c_void,
    r: *mut c_void,
    label: String,
    step: usize,
    opts: DumpOpts,
}

impl<'a> SDual<'a> {
    fn empty(s: &'a Session, lay: Layout, label: &str, opts: DumpOpts) -> SDual<'a> {
        SDual {
            s,
            lay,
            c: std::ptr::null_mut(),
            r: std::ptr::null_mut(),
            label: label.to_string(),
            step: 0,
            opts,
        }
    }
    fn from_shmode(
        s: &'a Session,
        lay: Layout,
        label: &str,
        sh_mode: c_int,
        opts: DumpOpts,
    ) -> SDual<'a> {
        unsafe {
            SDual {
                s,
                lay,
                c: (s.c.shmode_func)(lay.elemsize, sh_mode),
                r: (s.rust.shmode_func)(lay.elemsize, sh_mode),
                label: label.to_string(),
                step: 0,
                opts,
            }
        }
    }

    /// `stbds_make_hash_index` leaves `hash_index.temp_key` **uninitialised**
    /// (it is never written by the index constructor), so the field may only be
    /// compared while the most recent write to it is known: i.e. after a
    /// `stbds_hmput_key` into a string-mode table and before any operation that
    /// rebuilds/shrinks the index (every `stbds_hmdel_key` may do that).
    fn temp_key_check(&mut self, on: bool) {
        self.opts.check_temp_key = on;
    }

    #[track_caller]
    fn check(&mut self, what: &str) {
        unsafe {
            let c = dump_map(self.c, self.opts);
            let r = dump_map(self.r, self.opts);
            assert_same(
                &format!("{} [{} step {}] {}", self.label, self.lay.name, self.step, what),
                &c,
                &r,
            );
        }
        self.step += 1;
    }

    fn put(&mut self, key: *mut c_char, val: &[u8], mode: c_int) {
        unsafe {
            self.c = map_put_string(self.s.c, self.c, self.lay, key, val, mode);
            self.r = map_put_string(self.s.rust, self.r, self.lay, key, val, mode);
        }
        self.check("put");
    }

    fn put_quiet(&mut self, key: *mut c_char, val: &[u8], mode: c_int) {
        unsafe {
            self.c = map_put_string(self.s.c, self.c, self.lay, key, val, mode);
            self.r = map_put_string(self.s.rust, self.r, self.lay, key, val, mode);
        }
    }

    fn get(&mut self, key: *mut c_char, mode: c_int) -> isize {
        unsafe {
            let (c, ci) = map_geti(self.s.c, self.c, self.lay, key as *mut c_void, mode);
            let (r, ri) = map_geti(self.s.rust, self.r, self.lay, key as *mut c_void, mode);
            self.c = c;
            self.r = r;
            assert_eq!(ci, ri, "{} shgeti index differs (C={} RUST={})", self.label, ci, ri);
            self.check("get");
            ci
        }
    }

    fn get_ts(&mut self, key: *mut c_char, mode: c_int) -> isize {
        unsafe {
            let (c, ct, ch) = map_geti_ts(self.s.c, self.c, self.lay, key as *mut c_void, mode);
            let (r, rt, rh) = map_geti_ts(self.s.rust, self.r, self.lay, key as *mut c_void, mode);
            self.c = c;
            self.r = r;
            assert_eq!(ct, rt, "{} shgeti_ts *temp differs", self.label);
            assert_eq!(ch, rh, "{} shgeti_ts header temp differs", self.label);
            self.check("get_ts");
            ct
        }
    }

    fn del(&mut self, key: *mut c_char, mode: c_int) -> isize {
        unsafe {
            let (c, ct) = map_del(self.s.c, self.c, self.lay, key as *mut c_void, 0, mode);
            let (r, rt) = map_del(self.s.rust, self.r, self.lay, key as *mut c_void, 0, mode);
            assert_eq!(c.is_null(), r.is_null());
            self.c = c;
            self.r = r;
            assert_eq!(ct, rt, "{} shdel temp differs (C={} RUST={})", self.label, ct, rt);
            self.check("del");
            ct
        }
    }

    fn free(self) {
        unsafe {
            map_free(self.s.c, self.c, self.lay);
            map_free(self.s.rust, self.r, self.lay);
        }
    }
}

const STR_LAYOUTS: [Layout; 2] = [L_STR, L_STRB];

// --- rows 35/36: auto-created SH_DEFAULT table --------------------------
#[test]
fn cfg_35_36_string_default_table() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 35);
    for &lay in STR_LAYOUTS.iter() {
        for n in [1usize, 2, 6, 7, 8, 13, 40] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 20);
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
            let mut d = SDual::empty(
                &s,
                lay,
                &format!("sh_default n={}", n),
                DumpOpts::strptr(lay.elemsize)
                    .with_temp_key()
                    .with_ptr_identity(),
            );
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, HM_STRING);
            }
            for &p in ptrs.iter() {
                assert!(d.get(p, HM_STRING) >= 0);
            }
            d.free();
        }
        // include the empty string explicitly
        let mut keys = Keys::new();
        let e = keys.add(b"", 16);
        let a = keys.add(b"a", 16);
        let mut d = SDual::empty(
            &s,
            lay,
            "sh_default empty-string",
            DumpOpts::strptr(lay.elemsize)
                .with_temp_key()
                .with_ptr_identity(),
        );
        let v = vec![1u8; lay.elemsize - 8];
        d.put(e, &v, HM_STRING);
        d.put(a, &v, HM_STRING);
        assert!(d.get(e, HM_STRING) >= 0);
        assert!(d.get(a, HM_STRING) >= 0);
        d.free();
    }
}

// --- rows 37/52: duplicate keys, temp_key asymmetry --------------------
#[test]
fn cfg_37_52_string_duplicates_and_temp_key() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 37);
    let lay = L_STR;
    for trial in 0..12 {
        let mut keys = Keys::new();
        let texts = distinct_texts(&mut rng, 30, 12);
        let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
        // second set of buffers holding *equal* text at *different* addresses
        let alias: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();

        let mut d = SDual::empty(
            &s,
            lay,
            &format!("dup trial={}", trial),
            DumpOpts::strptr(lay.elemsize)
                .with_temp_key()
                .with_ptr_identity(),
        );
        for &p in ptrs.iter() {
            let v = rng.bytes(lay.elemsize - 8);
            d.put(p, &v, HM_STRING);
        }
        // re-put through the aliases: hmput_key must find the existing entry,
        // keep the ORIGINAL key pointer, and (in the upper scan only) refresh
        // temp_key from the stored pointer.
        for _ in 0..200 {
            let i = rng.below(alias.len());
            let v = rng.bytes(lay.elemsize - 8);
            d.put(alias[i], &v, HM_STRING);
        }
        d.free();
    }
}

// --- row 38: SH_STRDUP ------------------------------------------------
#[test]
fn cfg_38_string_strdup_table() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 38);
    for &lay in STR_LAYOUTS.iter() {
        for n in [1usize, 6, 7, 20, 100] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 24);
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
            let mut d = SDual::from_shmode(
                &s,
                lay,
                &format!("strdup n={}", n),
                SH_STRDUP,
                DumpOpts::strptr(lay.elemsize),
            );
            d.check("fresh strdup table");
            d.temp_key_check(true);
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, HM_STRING);
            }
            for &p in ptrs.iter() {
                assert!(d.get(p, HM_STRING) >= 0);
                assert!(d.get_ts(p, HM_STRING) >= 0);
            }
            // re-put existing keys: no new strdup, entry updated in place
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, HM_STRING);
            }
            d.free();
        }
    }
}

// --- row 39: SH_ARENA ------------------------------------------------
#[test]
fn cfg_39_string_arena_table() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 39);
    for &lay in STR_LAYOUTS.iter() {
        for n in [1usize, 6, 7, 30, 200] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 30);
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
            let mut d = SDual::from_shmode(
                &s,
                lay,
                &format!("arena n={}", n),
                SH_ARENA,
                DumpOpts::strptr(lay.elemsize),
            );
            d.check("fresh arena table");
            d.temp_key_check(true);
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, HM_STRING);
            }
            for &p in ptrs.iter() {
                assert!(d.get(p, HM_STRING) >= 0);
            }
            d.free();
        }
    }
    // strings that force the arena block ladder from inside the map
    let lay = L_STR;
    let mut keys = Keys::new();
    let mut d = SDual::from_shmode(
        &s,
        lay,
        "arena big strings",
        SH_ARENA,
        DumpOpts::strptr(lay.elemsize).with_temp_key(),
    );
    d.temp_key_check(false);
    d.check("fresh arena table");
    d.temp_key_check(true);
    for i in 0..40usize {
        let len = match i % 4 {
            0 => 400,
            1 => 600,   // > 512 -> dedicated block early on
            2 => 2000,  // > blocksize for small block counts
            _ => 30,
        };
        let mut t = rand_text(&mut rng, len);
        // make each key unique
        t.extend_from_slice(format!("#{}", i).as_bytes());
        let p = keys.add(&t, 16);
        let v = rng.bytes(lay.elemsize - 8);
        d.put(p, &v, HM_STRING);
    }
    d.free();
}

// --- row 40: explicit SH_DEFAULT via shmode_func ----------------------
#[test]
fn cfg_40_explicit_sh_default_table() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 40);
    for &lay in STR_LAYOUTS.iter() {
        let mut keys = Keys::new();
        let texts = distinct_texts(&mut rng, 25, 18);
        let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
        let mut d = SDual::from_shmode(
            &s,
            lay,
            "explicit sh_default",
            SH_DEFAULT,
            DumpOpts::strptr(lay.elemsize).with_ptr_identity(),
        );
        d.check("fresh table");
        d.temp_key_check(true);
        for &p in ptrs.iter() {
            let v = rng.bytes(lay.elemsize - 8);
            d.put(p, &v, HM_STRING);
        }
        for &p in ptrs.iter() {
            assert!(d.get(p, HM_STRING) >= 0);
            assert!(d.get_ts(p, HM_STRING) >= 0);
        }
        d.free();
    }
}

// --- row 41: SH_NONE table + mode=STRING -> `default:` memcpy branch ---
// Only inserts of *distinct* keys are well defined here: the element holds the
// first `keysize` bytes of the key *text* (not a pointer), so any subsequent
// `strcmp` through it would dereference text-as-a-pointer. Insert-only is what
// the C code does deterministically, and the resulting element bytes are
// directly comparable.
#[test]
fn cfg_41_sh_none_table_with_string_mode() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 41);
    for &lay in STR_LAYOUTS.iter() {
        for n in [1usize, 2, 5, 6, 7] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 24);
            // pad to 32 bytes so `memcpy(dst, key, keysize)` is always in bounds
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 32)).collect();
            let mut d = SDual::from_shmode(
                &s,
                lay,
                &format!("sh_none+string n={}", n),
                SH_NONE,
                DumpOpts::raw(lay.elemsize),
            );
            d.check("fresh SH_NONE table");
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, HM_STRING);
            }
            d.free();
        }
    }
}

// --- row 46: lookups on every string table kind -----------------------
#[test]
fn cfg_46_string_lookups_all_table_kinds() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 46);
    let lay = L_STR;
    for &shm in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        for n in [1usize, 7, 40] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 16);
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
            let absent_texts = distinct_texts(&mut rng, 40, 16);
            let absent: Vec<*mut c_char> = absent_texts
                .iter()
                .filter(|t| !texts.contains(t))
                .map(|t| keys.add(t, 16))
                .collect();

            let mut d = SDual::from_shmode(
                &s,
                lay,
                &format!("lookup shm={} n={}", shm, n),
                shm,
                DumpOpts::strptr(lay.elemsize),
            );
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, HM_STRING);
            }
            for &p in ptrs.iter() {
                assert!(d.get(p, HM_STRING) >= 0);
                assert!(d.get_ts(p, HM_STRING) >= 0);
            }
            for &p in absent.iter() {
                assert_eq!(d.get(p, HM_STRING), -1);
                assert_eq!(d.get_ts(p, HM_STRING), -1);
            }
            d.free();
        }
    }
    // hmget_key / hmget_key_ts on a NULL string map
    let mut keys = Keys::new();
    let p = keys.add(b"nope", 16);
    let mut d = SDual::empty(&s, lay, "get-null-string", DumpOpts::strptr(lay.elemsize));
    assert_eq!(d.get(p, HM_STRING), -1);
    d.free();
    let mut d = SDual::empty(&s, lay, "get_ts-null-string", DumpOpts::strptr(lay.elemsize));
    assert_eq!(d.get_ts(p, HM_STRING), -1);
    d.free();
}

// --- rows 56/57/58: deletes on every string table kind ----------------
#[test]
fn cfg_56_57_58_string_deletes() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 56);
    let lay = L_STR;
    for &shm in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        for n in [1usize, 2, 7, 13, 60] {
            for which in 0..4usize {
                let mut keys = Keys::new();
                let texts = distinct_texts(&mut rng, n, 16);
                let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
                let mut d = SDual::from_shmode(
                    &s,
                    lay,
                    &format!("del shm={} n={} which={}", shm, n, which),
                    shm,
                    DumpOpts::strptr(lay.elemsize),
                );
                for &p in ptrs.iter() {
                    let v = rng.bytes(lay.elemsize - 8);
                    d.put(p, &v, HM_STRING);
                }
                match which {
                    0 => {
                        d.del(ptrs[n - 1], HM_STRING);
                    }
                    1 => {
                        d.del(ptrs[0], HM_STRING);
                    }
                    2 => {
                        d.del(ptrs[n / 2], HM_STRING);
                    }
                    _ => {
                        // delete everything, driving shrink + tombstone rebuild
                        let mut idx: Vec<usize> = (0..n).collect();
                        for i in (1..n).rev() {
                            let j = rng.below(i + 1);
                            idx.swap(i, j);
                        }
                        for &i in idx.iter() {
                            assert_eq!(d.del(ptrs[i], HM_STRING), 1);
                        }
                    }
                }
                // absent delete
                let ap = keys.add(b"definitely-absent-key", 32);
                assert_eq!(d.del(ap, HM_STRING), 0);
                d.free();
            }
        }
    }
}

// --- row 61: hmfree_func over every string table kind ------------------
#[test]
fn cfg_61_hmfree_string_tables() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 61);
    for &lay in STR_LAYOUTS.iter() {
        for &shm in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
            for n in [0usize, 1, 7, 100] {
                let mut keys = Keys::new();
                let texts = distinct_texts(&mut rng, n, 30);
                let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 32)).collect();
                let mut d = SDual::from_shmode(
                    &s,
                    lay,
                    &format!("hmfree shm={} n={}", shm, n),
                    shm,
                    if shm == SH_NONE {
                        DumpOpts::raw(lay.elemsize)
                    } else {
                        DumpOpts::strptr(lay.elemsize)
                    },
                );
                for &p in ptrs.iter() {
                    let v = rng.bytes(lay.elemsize - 8);
                    d.put_quiet(p, &v, HM_STRING);
                }
                d.check("before free");
                d.free();
            }
        }
        // auto-created SH_DEFAULT table (no shmode_func)
        for n in [0usize, 1, 20] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 20);
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 32)).collect();
            let mut d = SDual::empty(&s, lay, "hmfree auto", DumpOpts::strptr(lay.elemsize));
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put_quiet(p, &v, HM_STRING);
            }
            if n > 0 {
                d.check("before free");
            }
            d.free();
        }
    }
}

// --- row 72: mixed randomized string workload -------------------------
#[test]
fn cfg_72_mixed_string_workload() {
    let s = session();
    let lay = L_STR;
    for &shm in [SH_DEFAULT, SH_STRDUP, SH_ARENA].iter() {
        let mut rng = Rng::new(TEST_SEED ^ 72 ^ (shm as u64));
        let mut keys = Keys::new();
        let texts = distinct_texts(&mut rng, 50, 14);
        let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 16)).collect();
        let mut d = SDual::from_shmode(
            &s,
            lay,
            &format!("mixed shm={}", shm),
            shm,
            DumpOpts::strptr(lay.elemsize),
        );
        for _ in 0..1500 {
            let p = ptrs[rng.below(ptrs.len())];
            match rng.below(5) {
                0 | 1 => {
                    let v = rng.bytes(lay.elemsize - 8);
                    d.put(p, &v, HM_STRING);
                }
                2 => {
                    d.get(p, HM_STRING);
                }
                3 => {
                    d.get_ts(p, HM_STRING);
                }
                _ => {
                    d.del(p, HM_STRING);
                }
            }
        }
        d.free();
    }
}

// --- row 73 (string half): out-of-range HM modes ----------------------
// Every mode >= 1 selects the string path. `hmdel_key`, however, tests
// `mode == STBDS_HM_STRING` exactly, so for mode != 1 only deletes of the
// *last live element* (old_index == final_index) stay well defined; those are
// reached by deleting in reverse insertion order.
#[test]
fn cfg_73_out_of_range_hm_modes_string() {
    let s = session();
    let mut rng = Rng::new(TEST_SEED ^ 173);
    let lay = L_STR;
    let modes: [c_int; 6] = [1, 2, 3, 7, 255, i32::MAX];
    for &m in modes.iter() {
        for n in [1usize, 6, 7, 20] {
            let mut keys = Keys::new();
            let texts = distinct_texts(&mut rng, n, 16);
            let ptrs: Vec<*mut c_char> = texts.iter().map(|t| keys.add(t, 32)).collect();
            let mut d = SDual::empty(
                &s,
                lay,
                &format!("hm-mode({}) n={}", m, n),
                DumpOpts::strptr(lay.elemsize)
                    .with_temp_key()
                    .with_ptr_identity(),
            );
            for &p in ptrs.iter() {
                let v = rng.bytes(lay.elemsize - 8);
                d.put(p, &v, m);
            }
            for &p in ptrs.iter() {
                assert!(d.get(p, m) >= 0, "mode {} lookup failed", m);
                assert!(d.get_ts(p, m) >= 0);
            }
            // absent lookups
            let ap = keys.add(b"absent~~~", 32);
            assert_eq!(d.get(ap, m), -1);
            assert_eq!(d.del(ap, m), 0);
            // deletes may rebuild/shrink the index, which leaves temp_key
            // uninitialised again -> stop comparing it from here on
            d.temp_key_check(false);
            // deletes in reverse insertion order => no compaction, no re-find
            for &p in ptrs.iter().rev() {
                assert_eq!(d.del(p, m), 1, "mode {} delete failed", m);
            }
            d.free();
        }
    }
}
