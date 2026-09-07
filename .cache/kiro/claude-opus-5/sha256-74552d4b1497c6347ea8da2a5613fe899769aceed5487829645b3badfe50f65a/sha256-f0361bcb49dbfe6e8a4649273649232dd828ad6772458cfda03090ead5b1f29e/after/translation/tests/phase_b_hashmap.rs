//! Phase B — valid-path differential tests for the hash-map entry points,
//! driven exactly the way the `stbds_hmput`/`shput`/`hmget`/`hmdel` macros in
//! `c_src/src/lib.c` drive them (low-level exports, not a convenience wrapper).
//!
//! CONFIGS.md rows 14-44, 53-54.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// A faithful re-implementation of the C macros on top of the exported symbols
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Cfg {
    elemsize: usize,
    keysize: usize,
    /// byte offset inside the element where the macro writes `.value`
    valoff: usize,
    mode: c_int,
    /// `keyoffset` argument of `stbds_hmdel_key` (`STBDS_OFFSETOF(t,key)`)
    keyoffset: usize,
    kind: KeyKind,
    /// mirror `stbds_hmput(t,k,v)`, which also assigns `.key = k`
    write_key_on_put: bool,
    /// how many leading bytes of a freshly inserted element the *library*
    /// defines (`keysize` for the `default:` memcpy branch, 8 for the three
    /// pointer-storing arena modes).  Everything past this is indeterminate
    /// realloc content in both builds, so the consumer must initialise it
    /// before it can be compared.
    lib_writes: usize,
}

struct Map<'a> {
    api: &'a Api,
    t: *mut c_void,
    cfg: Cfg,
    /// last observed `hash_table` pointer — a change means the index was
    /// rebuilt and `temp_key` is uninitialised again
    last_table: *mut c_void,
    /// `table->temp_key` is only written by the `stbds_temp_key` macro; reading
    /// it before that is a wild pointer dereference
    tk_valid: bool,
}

impl<'a> Map<'a> {
    fn new(api: &'a Api, cfg: Cfg) -> Map<'a> {
        Map {
            api,
            t: std::ptr::null_mut(),
            cfg,
            last_table: std::ptr::null_mut(),
            tk_valid: false,
        }
    }

    fn from_shmode(api: &'a Api, cfg: Cfg, sh_mode: c_int) -> Map<'a> {
        let t = unsafe { (api.shmode_func)(cfg.elemsize, sh_mode) };
        let lt = unsafe { (*header_of_hash(t, cfg.elemsize)).hash_table };
        Map {
            api,
            t,
            cfg,
            last_table: lt,
            tk_valid: false,
        }
    }

    unsafe fn table(&self) -> *mut c_void {
        if self.t.is_null() {
            std::ptr::null_mut()
        } else {
            (*header_of_hash(self.t, self.cfg.elemsize)).hash_table
        }
    }

    unsafe fn arr_len(&self) -> usize {
        if self.t.is_null() {
            0
        } else {
            (*header_of_hash(self.t, self.cfg.elemsize)).length
        }
    }

    /// Invalidate `temp_key` tracking if the hash index object was replaced.
    unsafe fn note_table(&mut self) {
        let cur = self.table();
        if cur != self.last_table {
            self.last_table = cur;
            self.tk_valid = false;
        }
    }

    unsafe fn put(&mut self, key: *mut c_void, val: u8) -> isize {
        let c = self.cfg;
        let before = self.arr_len();
        self.t = (self.api.hmput_key)(self.t, c.elemsize, key, c.keysize, c.mode);
        self.note_table();
        let after = self.arr_len();
        let idx = (*header_of_hash(self.t, c.elemsize)).temp;
        let e = (self.t as *mut u8).offset(idx * c.elemsize as isize);
        // the library leaves [lib_writes, elemsize) indeterminate
        if c.elemsize > c.lib_writes {
            std::ptr::write_bytes(e.add(c.lib_writes), 0, c.elemsize - c.lib_writes);
        }
        if c.write_key_on_put {
            std::ptr::copy_nonoverlapping(key as *const u8, e, c.keysize);
        }
        *e.add(c.valoff) = val;
        if after > before {
            // a fresh insert always runs the `switch (table->string.mode)`,
            // whose 1/2/3 arms all assign `stbds_temp_key`
            let sm = self.string_mode();
            if (1..=3).contains(&sm) {
                self.tk_valid = true;
            }
        }
        idx
    }

    unsafe fn string_mode(&self) -> i32 {
        let t = self.table() as *mut HashIndex;
        if t.is_null() {
            -1
        } else {
            (*t).string.mode as i32
        }
    }

    unsafe fn get(&mut self, key: *mut c_void) -> isize {
        let c = self.cfg;
        self.t = (self.api.hmget_key)(self.t, c.elemsize, key, c.keysize, c.mode);
        self.note_table();
        (*header_of_hash(self.t, c.elemsize)).temp
    }

    unsafe fn get_ts(&mut self, key: *mut c_void) -> (isize, isize) {
        let c = self.cfg;
        let mut temp: isize = 0x5A5A_5A5A;
        self.t = (self.api.hmget_key_ts)(self.t, c.elemsize, key, c.keysize, &mut temp, c.mode);
        self.note_table();
        (temp, (*header_of_hash(self.t, c.elemsize)).temp)
    }

    unsafe fn del(&mut self, key: *mut c_void) -> isize {
        let c = self.cfg;
        self.t = (self.api.hmdel_key)(self.t, c.elemsize, key, c.keysize, c.keyoffset, c.mode);
        if self.t.is_null() {
            self.last_table = std::ptr::null_mut();
            self.tk_valid = false;
            0
        } else {
            self.note_table();
            (*header_of_hash(self.t, c.elemsize)).temp
        }
    }

    unsafe fn put_default(&mut self) {
        let c = self.cfg;
        self.t = (self.api.hmput_default)(self.t, c.elemsize);
        self.note_table();
    }

    /// `stbds_hmlen(t)`
    unsafe fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            (*header_of_hash(self.t, self.cfg.elemsize)).length as isize - 1
        }
    }

    unsafe fn dump(&self) -> String {
        dump_hash(self.t, self.cfg.elemsize, self.cfg.kind)
    }

    /// `table->temp_key`, or `None` while it is still indeterminate.
    unsafe fn temp_key(&self) -> Option<String> {
        if self.tk_valid {
            Some(temp_key_str(self.t, self.cfg.elemsize))
        } else {
            None
        }
    }

    unsafe fn free(&mut self) {
        if !self.t.is_null() {
            (self.api.hmfree_func)(
                (self.t as *mut u8).sub(self.cfg.elemsize) as *mut c_void,
                self.cfg.elemsize,
            );
            self.t = std::ptr::null_mut();
            self.last_table = std::ptr::null_mut();
            self.tk_valid = false;
        }
    }
}

/// Runs the same op on both maps and asserts their dumps agree.
macro_rules! both {
    ($ctx:expr, $mc:expr, $mr:expr, |$m:ident| $body:expr) => {{
        let rc = unsafe {
            let $m = &mut $mc;
            $body
        };
        let rr = unsafe {
            let $m = &mut $mr;
            $body
        };
        assert_eq!(rc, rr, "return value mismatch in {}", $ctx);
        let dc = unsafe { $mc.dump() };
        let dr = unsafe { $mr.dump() };
        diff($ctx, &dc, &dr);
        rc
    }};
}

fn seeded(p: &Pair, seed: usize) {
    unsafe {
        (p.c.rand_seed)(seed);
        (p.r.rand_seed)(seed);
    }
}

fn cfg_bin(elemsize: usize, keysize: usize) -> Cfg {
    Cfg {
        elemsize,
        keysize,
        valoff: elemsize - 1,
        mode: STBDS_HM_BINARY,
        keyoffset: 0,
        kind: KeyKind::Binary,
        write_key_on_put: true,
        lib_writes: keysize,
    }
}

fn cfg_str(elemsize: usize, mode: c_int) -> Cfg {
    Cfg {
        elemsize,
        keysize: 8,
        valoff: 8,
        mode,
        keyoffset: 0,
        kind: KeyKind::StringPtr,
        write_key_on_put: false,
        lib_writes: 8,
    }
}

// ---------------------------------------------------------------------------
// Rows 14-17: BINARY mode
// ---------------------------------------------------------------------------

#[test]
fn row14_binary_int_key_growth_stages() {
    let p = load_pair();
    for &seed in &[0x31415926usize, 0, 1, usize::MAX] {
        seeded(&p, seed);
        let cfg = cfg_bin(8, 4);
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        for n in 0..120u32 {
            let mut k = n.to_le_bytes();
            let kp = k.as_mut_ptr() as *mut c_void;
            both!(
                &format!("row14 seed={:#x} put {}", seed, n),
                mc,
                mr,
                |m| m.put(kp, (n & 0xff) as u8)
            );
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row15_binary_8byte_key_1000_inserts() {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = cfg_bin(16, 8);
    let mut mc = Map::new(&p.c, cfg);
    let mut mr = Map::new(&p.r, cfg);
    let mut rng = Rng::new(0x1500);
    let mut keys: Vec<[u8; 8]> = Vec::new();
    for i in 0..1000usize {
        let mut k = [0u8; 8];
        k.copy_from_slice(&rng.next_u64().to_le_bytes());
        keys.push(k);
        let kp = keys[i].as_ptr() as *mut c_void;
        let rc = unsafe { mc.put(kp, i as u8) };
        let rr = unsafe { mr.put(kp, i as u8) };
        assert_eq!(rc, rr, "row15 put {} index", i);
        // full structural compare every 50 steps + around every growth point
        if i % 50 == 0 || i < 20 || [5, 6, 11, 12, 23, 24, 47, 48, 95, 96].contains(&i) {
            let dc = unsafe { mc.dump() };
            let dr = unsafe { mr.dump() };
            diff(&format!("row15 after put {}", i), &dc, &dr);
        }
    }
    let dc = unsafe { mc.dump() };
    let dr = unsafe { mr.dump() };
    diff("row15 final", &dc, &dr);
    unsafe {
        mc.free();
        mr.free();
    }
}

#[test]
fn row16_binary_duplicate_keys() {
    let p = load_pair();
    for &seed in &[0x31415926usize, 12345] {
        seeded(&p, seed);
        let cfg = cfg_bin(8, 8);
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        let mut rng = Rng::new(0x1600u64.wrapping_add(seed as u64));
        // small key domain -> lots of duplicate puts, exercising both
        // duplicate-found loops in stbds_hmput_key
        let mut keys: Vec<[u8; 8]> = Vec::new();
        for _ in 0..40 {
            let mut k = [0u8; 8];
            k.copy_from_slice(&rng.next_u64().to_le_bytes());
            keys.push(k);
        }
        for i in 0..600usize {
            let ki = rng.below(keys.len() as u64) as usize;
            let kp = keys[ki].as_ptr() as *mut c_void;
            both!(
                &format!("row16 seed={:#x} op{} key{}", seed, i, ki),
                mc,
                mr,
                |m| m.put(kp, i as u8)
            );
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row17_binary_elemsize24_collisions() {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = cfg_bin(24, 8);
    let mut mc = Map::new(&p.c, cfg);
    let mut mr = Map::new(&p.r, cfg);
    // 16-value key domain: every probe path (dup found, tombstone-free
    // wrap-around of the two inner loops) gets hit repeatedly
    let keys: Vec<[u8; 8]> = (0..16u64).map(|v| v.to_le_bytes()).collect();
    let mut rng = Rng::new(0x1700);
    for i in 0..400usize {
        let ki = rng.below(16) as usize;
        let kp = keys[ki].as_ptr() as *mut c_void;
        both!(&format!("row17 op{} key{}", i, ki), mc, mr, |m| m
            .put(kp, i as u8));
    }
    unsafe {
        mc.free();
        mr.free();
    }
}

// ---------------------------------------------------------------------------
// Rows 18-23: STRING mode / arena modes
// ---------------------------------------------------------------------------

/// Shared key pool. Inner `Vec` buffers stay put when the outer `Vec` grows, so
/// the pointers handed to a `SH_DEFAULT` table remain valid.
fn key_pool(seed: u64, n: usize, maxlen: usize) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(seed);
    (0..n)
        .map(|_| {
            let l = rng.below(maxlen as u64 + 1) as usize;
            rng.ascii_cstring(l)
        })
        .collect()
}

#[test]
fn row18_string_implicit_sh_default() {
    let p = load_pair();
    for &seed in &[0x31415926usize, 0, usize::MAX] {
        seeded(&p, seed);
        let cfg = cfg_str(16, STBDS_HM_STRING);
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        let keys = key_pool(0x1800u64.wrapping_add(seed as u64), 200, 24);
        for (i, k) in keys.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            both!(
                &format!("row18 seed={:#x} put {}", seed, i),
                mc,
                mr,
                |m| m.put(kp, i as u8)
            );
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row19_string_duplicates_and_temp_key() {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = cfg_str(16, STBDS_HM_STRING);
    let mut mc = Map::new(&p.c, cfg);
    let mut mr = Map::new(&p.r, cfg);
    // two *distinct* buffers holding the same bytes: re-putting the alias must
    // leave the stored key pointer at the original buffer (SH_DEFAULT keeps the
    // caller's pointer only on first insert)
    let base = key_pool(0x1900, 60, 12);
    let aliases: Vec<Vec<u8>> = base.iter().cloned().collect();
    let mut rng = Rng::new(0x19AA);
    for i in 0..500usize {
        let ki = rng.below(base.len() as u64) as usize;
        let use_alias = rng.below(2) == 1;
        let kp = if use_alias {
            aliases[ki].as_ptr() as *mut c_void
        } else {
            base[ki].as_ptr() as *mut c_void
        };
        both!(
            &format!("row19 op{} key{} alias={}", i, ki, use_alias),
            mc,
            mr,
            |m| (m.put(kp, i as u8), m.temp_key())
        );
    }
    unsafe {
        mc.free();
        mr.free();
    }
}

fn string_arena_scenario(sh_mode: c_int, maxlen: usize, tag: &str) {
    let p = load_pair();
    for &seed in &[0x31415926usize, 7] {
        seeded(&p, seed);
        let cfg = cfg_str(16, STBDS_HM_STRING);
        let mut mc = Map::from_shmode(&p.c, cfg, sh_mode);
        let mut mr = Map::from_shmode(&p.r, cfg, sh_mode);
        let keys = key_pool(0x2000u64.wrapping_add(sh_mode as u32 as u64).wrapping_add(seed as u64), 200, maxlen);
        let mut rng = Rng::new(0x2100u64.wrapping_add(sh_mode as u32 as u64));
        for i in 0..400usize {
            let ki = rng.below(keys.len() as u64) as usize;
            let kp = keys[ki].as_ptr() as *mut c_void;
            both!(
                &format!("{} seed={:#x} op{} key{}", tag, seed, i, ki),
                mc,
                mr,
                |m| (m.put(kp, i as u8), m.temp_key())
            );
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row20_sh_strdup() {
    string_arena_scenario(STBDS_SH_STRDUP, 24, "row20");
}

#[test]
fn row21_sh_arena_incl_long_keys() {
    string_arena_scenario(STBDS_SH_ARENA, 24, "row21a");
    // keys longer than STBDS_STRING_ARENA_BLOCKSIZE_MIN -> the oversized path
    string_arena_scenario(STBDS_SH_ARENA, 900, "row21b");
}

#[test]
fn row23_sh_default_explicit() {
    string_arena_scenario(STBDS_SH_DEFAULT, 24, "row23");
}

#[test]
fn row22_sh_none_with_string_mode_single_insert() {
    // `string.mode == SH_NONE` sends a STRING-mode put down the `default:`
    // memcpy branch, which copies `keysize` bytes *of the string itself* into
    // the element.  A second insert would make `stbds_is_key_equal` treat those
    // bytes as a `char *`, so exactly one insert is the whole observable
    // configuration.
    let p = load_pair();
    for &seed in &[0x31415926usize, 3] {
        seeded(&p, seed);
        let cfg = Cfg {
            kind: KeyKind::Binary, // elements hold raw bytes, NOT a pointer
            ..cfg_str(16, STBDS_HM_STRING)
        };
        let mut mc = Map::from_shmode(&p.c, cfg, STBDS_SH_NONE);
        let mut mr = Map::from_shmode(&p.r, cfg, STBDS_SH_NONE);
        let mut key = b"abcdefghijklmnop\0".to_vec();
        let kp = key.as_mut_ptr() as *mut c_void;
        both!(&format!("row22 seed={:#x}", seed), mc, mr, |m| m.put(kp, 0x5a));
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row24_shmode_func_out_of_range_modes() {
    let p = load_pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for &m in &[
        -1i32,
        0,
        1,
        2,
        3,
        4,
        5,
        255,
        256,
        257,
        258,
        259,
        1000,
        i32::MAX,
        i32::MIN,
        -256,
        -255,
    ] {
        for elemsize in [8usize, 16, 24] {
            for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
                unsafe {
                    (api.rand_seed)(0x31415926);
                    let t = (api.shmode_func)(elemsize, m);
                    out.push_str(&format!(
                        "mode={} e={} -> {}\n",
                        m,
                        elemsize,
                        dump_hash(t, elemsize, KeyKind::Binary)
                    ));
                    (api.hmfree_func)((t as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                }
            }
        }
    }
    diff("row24", &out_c, &out_r);
}

// ---------------------------------------------------------------------------
// Rows 25-29: hmget_key / hmget_key_ts
// ---------------------------------------------------------------------------

#[test]
fn row25_hmget_binary_present_and_absent() {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = cfg_bin(16, 8);
    let mut mc = Map::new(&p.c, cfg);
    let mut mr = Map::new(&p.r, cfg);
    let mut rng = Rng::new(0x2500);
    let present: Vec<[u8; 8]> = (0..200).map(|_| rng.next_u64().to_le_bytes()).collect();
    let absent: Vec<[u8; 8]> = (0..200).map(|_| rng.next_u64().to_le_bytes()).collect();
    for (i, k) in present.iter().enumerate() {
        let kp = k.as_ptr() as *mut c_void;
        both!(&format!("row25 put {}", i), mc, mr, |m| m.put(kp, i as u8));
    }
    for (i, k) in present.iter().chain(absent.iter()).enumerate() {
        let kp = k.as_ptr() as *mut c_void;
        both!(&format!("row25 get {}", i), mc, mr, |m| (
            m.get(kp),
            m.get_ts(kp)
        ));
    }
    unsafe {
        mc.free();
        mr.free();
    }
}

#[test]
fn row26_hmget_string_present_and_absent() {
    let p = load_pair();
    for sh in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        seeded(&p, 0x31415926);
        let cfg = cfg_str(16, STBDS_HM_STRING);
        let mut mc = Map::from_shmode(&p.c, cfg, sh);
        let mut mr = Map::from_shmode(&p.r, cfg, sh);
        let present = key_pool(0x2600, 150, 20);
        let absent = key_pool(0x2601, 150, 20);
        for (i, k) in present.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            both!(&format!("row26 sh={} put {}", sh, i), mc, mr, |m| m
                .put(kp, i as u8));
        }
        for (i, k) in present.iter().chain(absent.iter()).enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            both!(&format!("row26 sh={} get {}", sh, i), mc, mr, |m| (
                m.get(kp),
                m.get_ts(kp)
            ));
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row27_hmget_bootstrap_from_null() {
    let p = load_pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for elemsize in [4usize, 8, 16, 24] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
            for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
                unsafe {
                    (api.rand_seed)(0x31415926);
                    let mut key = b"somekey\0".to_vec();
                    let kp = key.as_mut_ptr() as *mut c_void;
                    // _ts variant
                    let mut temp: isize = 0x7777;
                    let t = (api.hmget_key_ts)(
                        std::ptr::null_mut(),
                        elemsize,
                        kp,
                        elemsize.min(8),
                        &mut temp,
                        mode,
                    );
                    out.push_str(&format!(
                        "e={} mode={} ts temp={} {}\n",
                        elemsize,
                        mode,
                        temp,
                        dump_hash(t, elemsize, KeyKind::Binary)
                    ));
                    (api.hmfree_func)((t as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                    // non-_ts variant
                    let t2 = (api.hmget_key)(
                        std::ptr::null_mut(),
                        elemsize,
                        kp,
                        elemsize.min(8),
                        mode,
                    );
                    out.push_str(&format!(
                        "e={} mode={} nots {}\n",
                        elemsize,
                        mode,
                        dump_hash(t2, elemsize, KeyKind::Binary)
                    ));
                    (api.hmfree_func)((t2 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                }
            }
        }
    }
    diff("row27", &out_c, &out_r);
}

#[test]
fn row28_hmget_on_hmput_default_only_array() {
    let p = load_pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for elemsize in [8usize, 16, 24] {
        for mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
            for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
                unsafe {
                    (api.rand_seed)(0x31415926);
                    // hash_table stays 0 -> the `table == 0` branch of hmget
                    let t = (api.hmput_default)(std::ptr::null_mut(), elemsize);
                    let mut key = b"anykey12\0".to_vec();
                    let kp = key.as_mut_ptr() as *mut c_void;
                    let mut temp: isize = 0x3333;
                    let t2 = (api.hmget_key_ts)(t, elemsize, kp, 8, &mut temp, mode);
                    out.push_str(&format!(
                        "e={} mode={} same={} temp={} {}\n",
                        elemsize,
                        mode,
                        t2 == t,
                        temp,
                        dump_hash(t2, elemsize, KeyKind::Binary)
                    ));
                    let t3 = (api.hmget_key)(t2, elemsize, kp, 8, mode);
                    out.push_str(&format!(
                        "  nots same={} {}\n",
                        t3 == t2,
                        dump_hash(t3, elemsize, KeyKind::Binary)
                    ));
                    // and hmdel on a table-less array
                    let t4 = (api.hmdel_key)(t3, elemsize, kp, 8, 0, mode);
                    out.push_str(&format!(
                        "  del same={} {}\n",
                        t4 == t3,
                        dump_hash(t4, elemsize, KeyKind::Binary)
                    ));
                    (api.hmfree_func)((t4 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                }
            }
        }
    }
    diff("row28", &out_c, &out_r);
}

// ---------------------------------------------------------------------------
// Rows 30-31: hmput_default
// ---------------------------------------------------------------------------

#[test]
fn row30_hmput_default_shapes() {
    let p = load_pair();
    let mut out_c = String::new();
    let mut out_r = String::new();
    for elemsize in [4usize, 8, 16, 24] {
        for (api, out) in [(&p.c, &mut out_c), (&p.r, &mut out_r)] {
            unsafe {
                (api.rand_seed)(0x31415926);
                let t1 = (api.hmput_default)(std::ptr::null_mut(), elemsize);
                out.push_str(&format!(
                    "e={} first {}\n",
                    elemsize,
                    dump_hash(t1, elemsize, KeyKind::Binary)
                ));
                let t2 = (api.hmput_default)(t1, elemsize);
                out.push_str(&format!(
                    "e={} second same={} {}\n",
                    elemsize,
                    t2 == t1,
                    dump_hash(t2, elemsize, KeyKind::Binary)
                ));
                // force length back to 0 -> the second branch of the condition
                (*header_of_hash(t2, elemsize)).length = 0;
                let t3 = (api.hmput_default)(t2, elemsize);
                out.push_str(&format!(
                    "e={} len0 same={} {}\n",
                    elemsize,
                    t3 == t2,
                    dump_hash(t3, elemsize, KeyKind::Binary)
                ));
                (api.hmfree_func)((t3 as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
    diff("row30", &out_c, &out_r);
}

#[test]
fn row31_hmput_default_then_insert() {
    let p = load_pair();
    for mode in [STBDS_HM_BINARY, STBDS_HM_STRING] {
        seeded(&p, 0x31415926);
        let cfg = if mode == STBDS_HM_BINARY {
            cfg_bin(16, 8)
        } else {
            cfg_str(16, STBDS_HM_STRING)
        };
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        let skeys = key_pool(0x3100, 60, 16);
        let bkeys: Vec<[u8; 8]> = {
            let mut r = Rng::new(0x3101);
            (0..60).map(|_| r.next_u64().to_le_bytes()).collect()
        };
        both!("row31 default", mc, mr, |m| m.put_default());
        for i in 0..60usize {
            let kp = if mode == STBDS_HM_BINARY {
                bkeys[i].as_ptr() as *mut c_void
            } else {
                skeys[i].as_ptr() as *mut c_void
            };
            both!(&format!("row31 mode={} put {}", mode, i), mc, mr, |m| m
                .put(kp, i as u8));
            if i % 7 == 0 {
                both!(&format!("row31 mode={} default {}", mode, i), mc, mr, |m| m
                    .put_default());
            }
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 32-43: hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn row32_row33_row34_del_middle_last_absent() {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = cfg_bin(16, 8);
    let mut mc = Map::new(&p.c, cfg);
    let mut mr = Map::new(&p.r, cfg);
    let mut rng = Rng::new(0x3200);
    let keys: Vec<[u8; 8]> = (0..40).map(|_| rng.next_u64().to_le_bytes()).collect();
    for (i, k) in keys.iter().enumerate() {
        let kp = k.as_ptr() as *mut c_void;
        both!(&format!("row32 put {}", i), mc, mr, |m| m.put(kp, i as u8));
    }
    // absent key
    let mut ak = [0xEEu8; 8];
    let akp = ak.as_mut_ptr() as *mut c_void;
    both!("row34 del absent", mc, mr, |m| m.del(akp));
    // delete a middle key (swap-with-last + re-find)
    let kp = keys[5].as_ptr() as *mut c_void;
    both!("row32 del middle", mc, mr, |m| m.del(kp));
    // delete the current last element
    let kp = keys[39].as_ptr() as *mut c_void;
    both!("row33 del last", mc, mr, |m| m.del(kp));
    // deleting the same key again -> absent
    both!("row33 del again", mc, mr, |m| m.del(kp));
    unsafe {
        mc.free();
        mr.free();
    }
}

#[test]
fn row35_row36_del_shrink_and_tombstone_rebuild() {
    let p = load_pair();
    for &seed in &[0x31415926usize, 99] {
        seeded(&p, seed);
        let cfg = cfg_bin(16, 8);
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        let mut rng = Rng::new(0x3500u64.wrapping_add(seed as u64));
        let keys: Vec<[u8; 8]> = (0..300).map(|_| rng.next_u64().to_le_bytes()).collect();
        for (i, k) in keys.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            both!(&format!("row35 seed={:#x} put {}", seed, i), mc, mr, |m| m
                .put(kp, i as u8));
        }
        // delete everything: crosses used_count_shrink_threshold repeatedly and
        // trips tombstone_count_threshold rebuilds on the way
        for (i, k) in keys.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            both!(&format!("row35 seed={:#x} del {}", seed, i), mc, mr, |m| m
                .del(kp));
        }
        // interleaved delete/reinsert -> tombstone reuse
        for i in 0..300usize {
            let ki = rng.below(keys.len() as u64) as usize;
            let kp = keys[ki].as_ptr() as *mut c_void;
            if rng.below(2) == 0 {
                both!(
                    &format!("row36 seed={:#x} op{} put {}", seed, i, ki),
                    mc,
                    mr,
                    |m| m.put(kp, i as u8)
                );
            } else {
                both!(
                    &format!("row36 seed={:#x} op{} del {}", seed, i, ki),
                    mc,
                    mr,
                    |m| m.del(kp)
                );
            }
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

fn del_string_scenario(sh: c_int, mode: c_int, tag: &str) {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = Cfg {
        mode,
        ..cfg_str(16, mode)
    };
    let mut mc = Map::from_shmode(&p.c, cfg, sh);
    let mut mr = Map::from_shmode(&p.r, cfg, sh);
    let keys = key_pool(0x3700u64.wrapping_add(sh as u32 as u64), 120, 20);
    for (i, k) in keys.iter().enumerate() {
        let kp = k.as_ptr() as *mut c_void;
        both!(&format!("{} put {}", tag, i), mc, mr, |m| m.put(kp, i as u8));
    }
    let mut rng = Rng::new(0x3701u64.wrapping_add(sh as u32 as u64));
    let mut order: Vec<usize> = (0..keys.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        order.swap(i, j);
    }
    for (n, &ki) in order.iter().enumerate() {
        let kp = keys[ki].as_ptr() as *mut c_void;
        both!(&format!("{} del {} (key {})", tag, n, ki), mc, mr, |m| m
            .del(kp));
    }
    unsafe {
        mc.free();
        mr.free();
    }
}

#[test]
fn row37_del_string_sh_default() {
    del_string_scenario(STBDS_SH_DEFAULT, STBDS_HM_STRING, "row37");
}

#[test]
fn row38_del_string_sh_strdup() {
    del_string_scenario(STBDS_SH_STRDUP, STBDS_HM_STRING, "row38");
}

#[test]
fn row39_del_string_sh_arena() {
    del_string_scenario(STBDS_SH_ARENA, STBDS_HM_STRING, "row39");
}

#[test]
fn row40_del_mode_two_string_table() {
    // `mode == 2`: `>= STBDS_HM_STRING` (so hashing/compare are string-based)
    // but `!= STBDS_HM_STRING`, so the strdup-free is skipped and the
    // swap-with-last re-find uses the *raw bytes* branch.
    let p = load_pair();
    for sh in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        seeded(&p, 0x31415926);
        let cfg = Cfg {
            mode: 2,
            ..cfg_str(16, 2)
        };
        let mut mc = Map::from_shmode(&p.c, cfg, sh);
        let mut mr = Map::from_shmode(&p.r, cfg, sh);
        // Distinct keys only: with `mode == 2` the swap-with-last re-find hashes
        // the *element bytes* (a `char *` value) as a string, which the C's own
        // `STBDS_ASSERT(slot >= 0)` then rejects with SIGABRT.  That abort is
        // covered as an error-surface row in phase_c_errors.rs; here we stay on
        // the valid path by only ever deleting the current last element.
        let keys: Vec<Vec<u8>> = (0..40)
            .map(|i| {
                let mut v = format!("row40key{:04}", i).into_bytes();
                v.push(0);
                v
            })
            .collect();
        for (i, k) in keys.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_void;
            both!(&format!("row40 sh={} put {}", sh, i), mc, mr, |m| m
                .put(kp, i as u8));
        }
        // Delete from the back so `old_index == final_index` every time; the
        // raw-bytes re-find branch is then never taken with a bogus key, which
        // would trip the C's own `STBDS_ASSERT(slot >= 0)`.
        for i in (0..keys.len()).rev() {
            let kp = keys[i].as_ptr() as *mut c_void;
            both!(&format!("row40 sh={} del {}", sh, i), mc, mr, |m| m.del(kp));
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row41_del_nonzero_keyoffset() {
    // The C hard-codes `keyoffset = 0` inside hmput_key/hmget_key, so the only
    // entry point that honours a non-zero keyoffset is hmdel_key.  Elements are
    // built with the key duplicated at offset 8 so the alternate keyoffset is
    // self-consistent (anything else trips the C's internal asserts).
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg = Cfg {
        keyoffset: 8,
        ..cfg_bin(16, 8)
    };
    let mut mc = Map::new(&p.c, cfg);
    let mut mr = Map::new(&p.r, cfg);
    let mut rng = Rng::new(0x4100);
    let keys: Vec<[u8; 16]> = (0..60)
        .map(|_| {
            let v = rng.next_u64().to_le_bytes();
            let mut e = [0u8; 16];
            e[0..8].copy_from_slice(&v);
            e[8..16].copy_from_slice(&v);
            e
        })
        .collect();
    for (i, k) in keys.iter().enumerate() {
        let kp = k.as_ptr() as *mut c_void;
        both!(&format!("row41 put {}", i), mc, mr, |m| {
            let idx = m.put(kp, 0);
            // mirror `.key` *and* the duplicate at offset 8
            let e = (m.t as *mut u8).offset(idx * 16);
            std::ptr::copy_nonoverlapping(kp as *const u8, e, 16);
            idx
        });
    }
    let mut order: Vec<usize> = (0..keys.len()).collect();
    for i in (1..order.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        order.swap(i, j);
    }
    for (n, &ki) in order.iter().enumerate() {
        let kp = keys[ki].as_ptr() as *mut c_void;
        both!(&format!("row41 del {} (key {})", n, ki), mc, mr, |m| m.del(kp));
    }
    unsafe {
        mc.free();
        mr.free();
    }
}

#[test]
fn row42_random_lifecycle_binary() {
    let p = load_pair();
    for &seed in &[0x31415926usize, 0, 424242] {
        seeded(&p, seed);
        let cfg = cfg_bin(16, 8);
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        let mut rng = Rng::new(0x4200u64.wrapping_add(seed as u64));
        let keys: Vec<[u8; 8]> = (0..80).map(|_| rng.next_u64().to_le_bytes()).collect();
        for i in 0..500usize {
            let ki = rng.below(keys.len() as u64) as usize;
            let kp = keys[ki].as_ptr() as *mut c_void;
            let ctx = format!("row42 seed={:#x} op{} key{}", seed, i, ki);
            match rng.below(10) {
                0..=4 => {
                    both!(&ctx, mc, mr, |m| m.put(kp, i as u8));
                }
                5..=6 => {
                    both!(&ctx, mc, mr, |m| (m.get(kp), m.get_ts(kp), m.len()));
                }
                _ => {
                    both!(&ctx, mc, mr, |m| m.del(kp));
                }
            };
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

#[test]
fn row43_random_lifecycle_string_strdup() {
    let p = load_pair();
    for sh in [STBDS_SH_DEFAULT, STBDS_SH_STRDUP, STBDS_SH_ARENA] {
        for &seed in &[0x31415926usize, 5] {
            seeded(&p, seed);
            let cfg = cfg_str(16, STBDS_HM_STRING);
            let mut mc = Map::from_shmode(&p.c, cfg, sh);
            let mut mr = Map::from_shmode(&p.r, cfg, sh);
            let keys = key_pool(0x4300u64.wrapping_add(sh as u32 as u64).wrapping_add(seed as u64), 80, 20);
            let mut rng = Rng::new(0x4301u64.wrapping_add(sh as u32 as u64).wrapping_add(seed as u64));
            for i in 0..500usize {
                let ki = rng.below(keys.len() as u64) as usize;
                let kp = keys[ki].as_ptr() as *mut c_void;
                let ctx = format!("row43 sh={} seed={:#x} op{} key{}", sh, seed, i, ki);
                match rng.below(10) {
                    0..=4 => {
                        both!(&ctx, mc, mr, |m| m.put(kp, i as u8));
                    }
                    5..=6 => {
                        both!(&ctx, mc, mr, |m| (m.get(kp), m.get_ts(kp), m.len()));
                    }
                    _ => {
                        both!(&ctx, mc, mr, |m| m.del(kp));
                    }
                };
            }
            unsafe {
                mc.free();
                mr.free();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 44: hmfree_func across every arena mode
// ---------------------------------------------------------------------------

#[test]
fn row44_hmfree_all_arena_modes() {
    let p = load_pair();
    for sh in [
        STBDS_SH_NONE,
        STBDS_SH_DEFAULT,
        STBDS_SH_STRDUP,
        STBDS_SH_ARENA,
    ] {
        seeded(&p, 0x31415926);
        let cfg = cfg_str(16, STBDS_HM_STRING);
        // SH_NONE cannot take more than one STRING put (see row22), so use
        // BINARY mode for it
        let (cfg, mode_keys_are_strings) = if sh == STBDS_SH_NONE {
            (
                Cfg {
                    mode: STBDS_HM_BINARY,
                    kind: KeyKind::Binary,
                    write_key_on_put: true,
                    ..cfg
                },
                false,
            )
        } else {
            (cfg, true)
        };
        let mut mc = Map::from_shmode(&p.c, cfg, sh);
        let mut mr = Map::from_shmode(&p.r, cfg, sh);
        let skeys = key_pool(0x4400u64.wrapping_add(sh as u32 as u64), 50, 700);
        let bkeys: Vec<[u8; 8]> = {
            let mut r = Rng::new(0x4401);
            (0..50).map(|_| r.next_u64().to_le_bytes()).collect()
        };
        for i in 0..50usize {
            let kp = if mode_keys_are_strings {
                skeys[i].as_ptr() as *mut c_void
            } else {
                bkeys[i].as_ptr() as *mut c_void
            };
            both!(&format!("row44 sh={} put {}", sh, i), mc, mr, |m| m
                .put(kp, i as u8));
        }
        unsafe {
            mc.free();
            mr.free();
        }
        // freeing an array with no hash table at all
        for (api, _) in [(&p.c, 0), (&p.r, 1)] {
            unsafe {
                let t = (api.hmput_default)(std::ptr::null_mut(), 16);
                (api.hmfree_func)((t as *mut u8).sub(16) as *mut c_void, 16);
                // and the NULL no-op
                (api.hmfree_func)(std::ptr::null_mut(), 16);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 53: the exact pipeline `helxo` runs, driven through the low-level exports
// ---------------------------------------------------------------------------

#[test]
fn row53_helxo_pipeline_via_low_level_exports() {
    let p = load_pair();
    for letter in [b'z', 0u8, 0xff, b'A'] {
        seeded(&p, 0x31415926);
        let cfg = cfg_str(16, STBDS_HM_STRING);
        let mut mc = Map::new(&p.c, cfg);
        let mut mr = Map::new(&p.r, cfg);
        let mut bob = b"bob\0".to_vec();
        let mut sally = b"sally\0".to_vec();
        let mut fred = b"fred\0".to_vec();
        let mut jen = b"jen\0".to_vec();
        let mut doug = b"doug\0".to_vec();
        let mut name = b"jen\0".to_vec(); // the `char name[4]` local
        let seq: Vec<(*mut c_void, u8)> = vec![
            (bob.as_mut_ptr() as *mut c_void, b'h'),
            (sally.as_mut_ptr() as *mut c_void, b'e'),
            (fred.as_mut_ptr() as *mut c_void, b'l'),
            (jen.as_mut_ptr() as *mut c_void, b'x'),
            (doug.as_mut_ptr() as *mut c_void, b'o'),
            (name.as_mut_ptr() as *mut c_void, letter),
        ];
        for (i, &(kp, v)) in seq.iter().enumerate() {
            both!(
                &format!("row53 letter={} put {}", letter, i),
                mc,
                mr,
                |m| (m.put(kp, v), m.temp_key(), m.len())
            );
        }
        unsafe {
            mc.free();
            mr.free();
        }
    }
}

// ---------------------------------------------------------------------------
// Row 54: two live tables sharing the (global) seed state
// ---------------------------------------------------------------------------

#[test]
fn row54_two_interleaved_tables() {
    let p = load_pair();
    seeded(&p, 0x31415926);
    let cfg_a = cfg_bin(16, 8);
    let cfg_b = cfg_str(16, STBDS_HM_STRING);
    let mut ac = Map::new(&p.c, cfg_a);
    let mut ar = Map::new(&p.r, cfg_a);
    let mut bc = Map::from_shmode(&p.c, cfg_b, STBDS_SH_STRDUP);
    let mut br = Map::from_shmode(&p.r, cfg_b, STBDS_SH_STRDUP);
    let mut rng = Rng::new(0x5400);
    let bin_keys: Vec<[u8; 8]> = (0..60).map(|_| rng.next_u64().to_le_bytes()).collect();
    let str_keys = key_pool(0x5401, 60, 18);
    for i in 0..600usize {
        let which = rng.below(2);
        let ki = rng.below(60) as usize;
        let op = rng.below(10);
        if which == 0 {
            let kp = bin_keys[ki].as_ptr() as *mut c_void;
            let ctx = format!("row54 A op{} key{}", i, ki);
            match op {
                0..=5 => {
                    both!(&ctx, ac, ar, |m| m.put(kp, i as u8));
                }
                6..=7 => {
                    both!(&ctx, ac, ar, |m| m.get(kp));
                }
                _ => {
                    both!(&ctx, ac, ar, |m| m.del(kp));
                }
            };
        } else {
            let kp = str_keys[ki].as_ptr() as *mut c_void;
            let ctx = format!("row54 B op{} key{}", i, ki);
            match op {
                0..=5 => {
                    both!(&ctx, bc, br, |m| m.put(kp, i as u8));
                }
                6..=7 => {
                    both!(&ctx, bc, br, |m| m.get(kp));
                }
                _ => {
                    both!(&ctx, bc, br, |m| m.del(kp));
                }
            };
        }
    }
    unsafe {
        ac.free();
        ar.free();
        bc.free();
        br.free();
    }
}

// keep the `c_char` import used
const _: Option<*const c_char> = None;
