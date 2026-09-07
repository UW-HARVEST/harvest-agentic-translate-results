//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.

#![allow(non_snake_case)]

#[path = "common/mod.rs"]
mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const SEED: usize = 0x31415926;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

unsafe fn hdr_of(map: *mut c_void, es: usize) -> *mut Header {
    ((map as *mut u8).sub(es + HDR)) as *mut Header
}

unsafe fn temp_of(map: *mut c_void, es: usize) -> isize {
    (*hdr_of(map, es)).temp
}

unsafe fn table_of(map: *mut c_void, es: usize) -> *mut HashIndex {
    (*hdr_of(map, es)).hash_table as *mut HashIndex
}

unsafe fn elem(map: *mut c_void, es: usize, raw_index: usize) -> *mut u8 {
    (map as *mut u8).sub(es).add(es * raw_index)
}

/// fill bytes [from..es) of the element that `hmput_key` just selected so no
/// uninitialised realloc bytes are ever compared
unsafe fn fill_tail(map: *mut c_void, es: usize, idx: isize, from: usize, val: u8) {
    let e = elem(map, es, (idx + 1) as usize);
    for i in from..es {
        *e.add(i) = val.wrapping_add(i as u8);
    }
}

struct M<'a> {
    l: &'a Lib,
    p: *mut c_void,
    es: usize,
}

impl<'a> M<'a> {
    fn none(l: &'a Lib, es: usize) -> M<'a> {
        M {
            l,
            p: std::ptr::null_mut(),
            es,
        }
    }
    unsafe fn shmode(l: &'a Lib, es: usize, mode: c_int) -> M<'a> {
        M {
            l,
            p: (l.shmode_func)(es, mode),
            es,
        }
    }
    unsafe fn put_bin(&mut self, key: &[u8], keysize: usize) -> isize {
        self.p = (self.l.hmput_key)(
            self.p,
            self.es,
            key.as_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        let t = temp_of(self.p, self.es);
        fill_tail(self.p, self.es, t, keysize, (t as u8).wrapping_mul(7).wrapping_add(3));
        t
    }
    #[allow(dead_code)]
    unsafe fn put_bin_mode(&mut self, key: &[u8], keysize: usize, mode: c_int) -> isize {
        self.p = (self.l.hmput_key)(
            self.p,
            self.es,
            key.as_ptr() as *mut c_void,
            keysize,
            mode,
        );
        let t = temp_of(self.p, self.es);
        fill_tail(self.p, self.es, t, keysize, 0x5a);
        t
    }
    /// emulates `shput(t,k,v)`: the library owns the key slot, we only fill the
    /// value bytes.  (`shputs` additionally copies the possibly-stale
    /// `temp_key` over the key, which can store a dangling pointer under
    /// `SH_STRDUP`; that is exercised by `str_dups` itself.)
    unsafe fn put_str(&mut self, key: *const c_char, mode: c_int) -> isize {
        self.p = (self.l.hmput_key)(self.p, self.es, key as *mut c_void, 8, mode);
        let t = temp_of(self.p, self.es);
        fill_tail(self.p, self.es, t, 8, 0x11);
        t
    }
    unsafe fn get_ts(&mut self, key: *const c_void, keysize: usize, mode: c_int) -> isize {
        let mut tmp: isize = 0x7f7f;
        self.p = (self.l.hmget_key_ts)(
            self.p,
            self.es,
            key as *mut c_void,
            keysize,
            &mut tmp,
            mode,
        );
        tmp
    }
    unsafe fn get(&mut self, key: *const c_void, keysize: usize, mode: c_int) -> isize {
        self.p = (self.l.hmget_key)(self.p, self.es, key as *mut c_void, keysize, mode);
        temp_of(self.p, self.es)
    }
    unsafe fn del(&mut self, key: *const c_void, keysize: usize, keyoffset: usize, mode: c_int) -> isize {
        let np = (self.l.hmdel_key)(
            self.p,
            self.es,
            key as *mut c_void,
            keysize,
            keyoffset,
            mode,
        );
        self.p = np;
        if np.is_null() {
            -99
        } else {
            temp_of(np, self.es)
        }
    }
    unsafe fn snap(&self, t: &mut Trace, tag: &str, kk: KeyKind) {
        snap_map(t, tag, self.p, self.es, kk);
    }
    unsafe fn free(&mut self) {
        if !self.p.is_null() {
            (self.l.hmfree_func)((self.p as *mut u8).sub(self.es) as *mut c_void, self.es);
        }
        self.p = std::ptr::null_mut();
    }
}

// ===========================================================================
// rows 1-7 : stbds_hash_bytes
// ===========================================================================

#[test]
fn row01_hash_bytes_len0_null() {
    differential("row01", |l, t| unsafe {
        for &s in &[0usize, 1, SEED, 1usize << 63, usize::MAX] {
            t.rec("h", (l.hash_bytes)(std::ptr::null_mut(), 0, s));
        }
        // len 0 with a real (but unread) buffer
        let mut b = [7u8; 8];
        t.rec("h0", (l.hash_bytes)(b.as_mut_ptr() as *mut c_void, 0, SEED));
    });
}

fn hash_bytes_lens(label: &str, lens: &[usize], per: usize, rngseed: u64) {
    // pre-generate all inputs so both libraries see identical bytes
    let mut rng = Rng::new(rngseed);
    let mut cases: Vec<(Vec<u8>, usize, usize)> = Vec::new();
    for &len in lens {
        for _ in 0..per {
            let b = rng.bytes(len.max(1));
            let seed = rng.next_u64() as usize;
            cases.push((b, len, seed));
        }
    }
    differential(label, |l, t| unsafe {
        for (b, len, seed) in &cases {
            let mut b = b.clone();
            t.rec(
                "h",
                (l.hash_bytes)(b.as_mut_ptr() as *mut c_void, *len, *seed),
            );
        }
    });
}

#[test]
fn row02_hash_bytes_len_1_to_7() {
    hash_bytes_lens("row02", &[1, 2, 3, 4, 5, 6, 7], 200, 0xB0B0);
}

#[test]
fn row03_hash_bytes_len8() {
    hash_bytes_lens("row03", &[8], 400, 0xC0FFEE);
}

#[test]
fn row04_hash_bytes_len_9_to_23() {
    hash_bytes_lens("row04", &(9..24).collect::<Vec<_>>(), 200, 0xDEAD);
}

#[test]
fn row05_hash_bytes_long() {
    hash_bytes_lens("row05", &[24, 64, 256, 1000], 100, 0xFEED);
}

#[test]
fn row06_hash_bytes_extremes() {
    differential("row06", |l, t| unsafe {
        for len in 1usize..18 {
            for fill in [0u8, 0xff, 0x80, 0x7f] {
                let mut b = vec![fill; len];
                for &s in &[0usize, SEED, usize::MAX] {
                    t.rec(
                        "h",
                        (l.hash_bytes)(b.as_mut_ptr() as *mut c_void, len, s),
                    );
                }
            }
        }
    });
}

#[test]
fn row07_hash_bytes_random_seeds() {
    let mut rng = Rng::new(0x5EED);
    let payload = rng.bytes(16);
    let seeds: Vec<usize> = (0..500).map(|_| rng.next_u64() as usize).collect();
    differential("row07", |l, t| unsafe {
        let mut p = payload.clone();
        for &s in &seeds {
            t.rec("h", (l.hash_bytes)(p.as_mut_ptr() as *mut c_void, 16, s));
        }
    });
}

// ===========================================================================
// rows 8-11 : stbds_hash_string
// ===========================================================================

#[test]
fn row08_hash_string_empty() {
    differential("row08", |l, t| unsafe {
        let mut e = cstring(b"");
        for &s in &[0usize, 1, SEED, 1usize << 63, usize::MAX] {
            t.rec("h", (l.hash_string)(e.as_mut_ptr() as *mut c_char, s));
        }
    });
}

fn hash_string_cases(label: &str, gen: impl Fn(&mut Rng, usize) -> Vec<u8>, lens: &[usize], per: usize, sd: u64) {
    let mut rng = Rng::new(sd);
    let mut cases: Vec<(Vec<u8>, usize)> = Vec::new();
    for &len in lens {
        for _ in 0..per {
            let s = cstring(&gen(&mut rng, len));
            let seed = rng.next_u64() as usize;
            cases.push((s, seed));
        }
    }
    differential(label, |l, t| unsafe {
        for (s, seed) in &cases {
            let mut s = s.clone();
            t.rec("h", (l.hash_string)(s.as_mut_ptr() as *mut c_char, *seed));
        }
    });
}

#[test]
fn row09_hash_string_ascii() {
    hash_string_cases(
        "row09",
        |r, n| r.ascii(n),
        &(1..41).collect::<Vec<_>>(),
        200,
        0x1234,
    );
}

#[test]
fn row10_hash_string_highbit() {
    hash_string_cases(
        "row10",
        |r, n| r.nonzero(n),
        &(1..33).collect::<Vec<_>>(),
        200,
        0x9999,
    );
}

#[test]
fn row11_hash_string_long() {
    hash_string_cases("row11", |r, n| r.nonzero(n), &[256, 1024], 50, 0x4242);
}

// ===========================================================================
// row 12 : rand_seed + seed LCG evolution
// ===========================================================================

#[test]
fn row12_seed_evolution() {
    differential("row12", |l, t| unsafe {
        for &s in &[0usize, 1, usize::MAX, 1usize << 63, SEED] {
            (l.rand_seed)(s);
            let mut maps = Vec::new();
            for i in 0..8 {
                let m = M::shmode(l, 16, SH_STRDUP);
                t.rec(&format!("seed[{i}]"), (*table_of(m.p, 16)).seed);
                maps.push(m);
            }
            for mut m in maps {
                m.free();
            }
            // the seed also feeds implicit table creation in hmput_key
            let mut m = M::none(l, 8);
            let k = [1u8, 2, 3, 4];
            m.put_bin(&k, 4);
            t.rec("implicit_seed", (*table_of(m.p, 8)).seed);
            m.free();
        }
    });
}

// ===========================================================================
// rows 13-15 : stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[test]
fn row13_arrgrowf_from_null() {
    differential("row13", |l, t| unsafe {
        for &es in &[1usize, 4, 8, 16, 24] {
            for &addlen in &[0usize, 1, 3, 7] {
                for &min_cap in &[0usize, 1, 4, 5, 100] {
                    let a = (l.arrgrowf)(std::ptr::null_mut(), es, addlen, min_cap);
                    snap_arr(t, &format!("a[{es},{addlen},{min_cap}]"), a, es, false);
                    if !a.is_null() {
                        (l.arrfreef)(a);
                    }
                }
            }
        }
    });
}

#[test]
fn row14_arrgrowf_regrow() {
    differential("row14", |l, t| unsafe {
        for &es in &[4usize, 8, 16] {
            let mut a = (l.arrgrowf)(std::ptr::null_mut(), es, 1, 0);
            snap_arr(t, "init", a, es, false);
            // no-op: min_cap <= capacity
            a = (l.arrgrowf)(a, es, 0, 1);
            snap_arr(t, "noop", a, es, false);
            // pretend we filled it
            (*((a as *mut u8).sub(HDR) as *mut Header)).length = 4;
            // min_cap inside (cap, 2*cap)
            a = (l.arrgrowf)(a, es, 0, 5);
            snap_arr(t, "grow5", a, es, false);
            // doubling chain
            for k in 0..8 {
                a = (l.arrgrowf)(a, es, 1, 0);
                (*((a as *mut u8).sub(HDR) as *mut Header)).length =
                    (*((a as *mut u8).sub(HDR) as *const Header)).capacity;
                snap_arr(t, &format!("dbl{k}"), a, es, false);
            }
            // huge min_cap
            a = (l.arrgrowf)(a, es, 3, 4096);
            snap_arr(t, "huge", a, es, false);
            (l.arrfreef)(a);
        }
    });
}

#[test]
fn row15_arrgrow_then_free() {
    differential("row15", |l, t| unsafe {
        for &es in &[1usize, 8, 32] {
            let a = (l.arrgrowf)(std::ptr::null_mut(), es, 4, 0);
            let h = (a as *mut u8).sub(HDR) as *mut Header;
            (*h).length = 4;
            for i in 0..4 * es {
                *(a as *mut u8).add(i) = (i as u8).wrapping_mul(3);
            }
            snap_arr(t, "before_free", a, es, true);
            (l.arrfreef)(a);
            t.rec("freed", es);
        }
    });
}

// ===========================================================================
// rows 16-22 : stbds_hmput_key, binary mode
// ===========================================================================

fn bin_put_test(label: &str, es: usize, keysize: usize, keys: Vec<Vec<u8>>) {
    differential(label, |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, es);
        for (i, k) in keys.iter().enumerate() {
            let idx = m.put_bin(k, keysize);
            t.rec(&format!("put[{i}]"), idx);
        }
        m.snap(t, "final", KeyKind::Raw);
        m.free();
    });
}

fn rand_keys(n: usize, keysize: usize, sd: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(sd);
    (0..n)
        .map(|_| {
            let mut v = rng.bytes(keysize);
            v.resize(keysize, 0);
            v
        })
        .collect()
}

#[test]
fn row16_put_binary_single() {
    bin_put_test("row16", 8, 4, vec![vec![1, 2, 3, 4]]);
}

#[test]
fn row17_put_binary_five() {
    bin_put_test(
        "row17",
        8,
        4,
        (0u32..5).map(|i| i.to_le_bytes().to_vec()).collect(),
    );
}

#[test]
fn row18_put_binary_six_grows() {
    bin_put_test(
        "row18",
        8,
        4,
        (0u32..6).map(|i| i.to_le_bytes().to_vec()).collect(),
    );
    bin_put_test(
        "row18b",
        8,
        4,
        (0u32..13).map(|i| i.to_le_bytes().to_vec()).collect(),
    );
}

#[test]
fn row19_put_binary_100_random_u32() {
    let mut ks = rand_keys(100, 4, 0xA11CE);
    // duplicates revisited
    for i in 0..20 {
        ks.push(ks[i].clone());
    }
    bin_put_test("row19", 8, 4, ks);
}

#[test]
fn row20_put_binary_1000_random_u64() {
    bin_put_test("row20", 16, 8, rand_keys(1000, 8, 0xB0B));
}

#[test]
fn row21_put_binary_narrow_and_wide_keys() {
    bin_put_test("row21_k1", 8, 1, rand_keys(200, 1, 0x11));
    bin_put_test("row21_k2", 8, 2, rand_keys(200, 2, 0x22));
    bin_put_test("row21_k16", 32, 16, rand_keys(200, 16, 0x33));
    bin_put_test("row21_k8_es24", 24, 8, rand_keys(200, 8, 0x44));
}

#[test]
fn row22_put_binary_update_existing() {
    differential("row22", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        let keys = rand_keys(30, 8, 0xC0DE);
        for k in &keys {
            m.put_bin(k, 8);
        }
        m.snap(t, "after_inserts", KeyKind::Raw);
        for (i, k) in keys.iter().enumerate() {
            let idx = m.put_bin(k, 8);
            t.rec(&format!("reput[{i}]"), idx);
        }
        m.snap(t, "after_reput", KeyKind::Raw);
        m.free();
    });
}

// ===========================================================================
// rows 23-29 : string modes
// ===========================================================================

fn str_strings(n: usize, maxlen: usize, sd: u64) -> Vec<Vec<u8>> {
    let mut rng = Rng::new(sd);
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0usize;
    while out.len() < n {
        let len = rng.below(maxlen) + 1;
        let mut v = rng.ascii(len);
        v.extend_from_slice(format!("#{i}", ).as_bytes());
        i += 1;
        if seen.insert(v.clone()) {
            out.push(cstring(&v));
        }
    }
    out
}

fn str_mode_test(label: &str, shmode: Option<c_int>, es: usize, strs: Vec<Vec<u8>>, mode: c_int) {
    differential(label, |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut owned: Vec<Vec<u8>> = strs.clone();
        let mut m = match shmode {
            Some(sm) => M::shmode(l, es, sm),
            None => M::none(l, es),
        };
        for (i, s) in owned.iter_mut().enumerate() {
            let idx = m.put_str(s.as_ptr() as *const c_char, mode);
            t.rec(&format!("put[{i}]"), idx);
        }
        m.snap(t, "final", KeyKind::StrPtr);
        m.free();
        // keep the source strings alive until after free
        std::hint::black_box(&owned);
    });
}

#[test]
fn row23_put_string_null_map_single() {
    str_mode_test("row23", None, 16, vec![cstring(b"hello")], HM_STRING);
}

#[test]
fn row24_put_string_implicit_default_200() {
    str_mode_test("row24", None, 16, str_strings(200, 20, 0xDD1), HM_STRING);
}

#[test]
fn row25_strdup_single() {
    differential("row25", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut key = cstring(b"hello world");
        let mut m = M::shmode(l, 16, SH_STRDUP);
        let idx = m.put_str(key.as_ptr() as *const c_char, HM_STRING);
        t.rec("idx", idx);
        let stored = *(elem(m.p, 16, (idx + 1) as usize) as *const *const c_char);
        t.rec("copied", stored != key.as_ptr() as *const c_char);
        t.rec_str("content", cstr(stored));
        m.snap(t, "final", KeyKind::StrPtr);
        m.free();
        std::hint::black_box(&mut key);
    });
}

#[test]
fn row26_strdup_200() {
    let mut s = str_strings(200, 40, 0xDD2);
    s.push(cstring(b""));
    for i in 0..30 {
        s.push(s[i].clone());
    }
    str_mode_test("row26", Some(SH_STRDUP), 16, s, HM_STRING);
}

#[test]
fn row27_arena_200_with_long_strings() {
    let mut s = str_strings(200, 30, 0xDD3);
    s.push(cstring(&vec![b'x'; 600]));
    s.push(cstring(&vec![b'y'; 1500]));
    s.push(cstring(&vec![b'z'; 100]));
    str_mode_test("row27", Some(SH_ARENA), 16, s, HM_STRING);
}

#[test]
fn row28_shnone_string_mode() {
    // string.mode == SH_NONE with mode == HM_STRING: the `default:` arm memcpy's
    // the first `keysize` *characters* of the key into the element.  Distinct
    // keys only -- comparing an existing key would strcmp a garbage pointer
    // (documented UB in ERRORS.md row 24).
    let strs: Vec<Vec<u8>> = (0..5)
        .map(|i| cstring(format!("distinct_key_{i:04}").as_bytes()))
        .collect();
    differential("row28", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut owned = strs.clone();
        let mut m = M::shmode(l, 16, SH_NONE);
        for (i, s) in owned.iter_mut().enumerate() {
            self_put(&mut m, s.as_ptr() as *const c_char);
            t.rec(&format!("put[{i}]"), temp_of(m.p, 16));
        }
        m.snap(t, "final", KeyKind::Raw);
        m.free();
        std::hint::black_box(&owned);
    });
}

unsafe fn self_put(m: &mut M, key: *const c_char) {
    m.p = (m.l.hmput_key)(m.p, m.es, key as *mut c_void, 8, HM_STRING);
    let t = temp_of(m.p, m.es);
    fill_tail(m.p, m.es, t, 8, 0x33);
}

#[test]
fn row29_explicit_default_mode() {
    str_mode_test("row29", Some(SH_DEFAULT), 16, str_strings(120, 25, 0xDD4), HM_STRING);
}

// ===========================================================================
// rows 30-34 : lookups
// ===========================================================================

#[test]
fn row30_get_ts_null_map() {
    differential("row30", |l, t| unsafe {
        for &es in &[8usize, 16, 32] {
            let mut tmp: isize = 12345;
            let key = [9u8; 8];
            let p = (l.hmget_key_ts)(
                std::ptr::null_mut(),
                es,
                key.as_ptr() as *mut c_void,
                8,
                &mut tmp,
                HM_BINARY,
            );
            t.rec("temp", tmp);
            snap_map(t, &format!("m{es}"), p, es, KeyKind::Raw);
            (l.hmfree_func)((p as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}

#[test]
fn row31_get_ts_no_table() {
    differential("row31", |l, t| unsafe {
        let es = 16usize;
        // array with a header but no hash table
        let a = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
        (*((a as *mut u8).sub(HDR) as *mut Header)).length = 1;
        for i in 0..es {
            *(a as *mut u8).add(i) = 0;
        }
        let map = (a as *mut u8).add(es) as *mut c_void;
        let mut tmp: isize = 4242;
        let key = [3u8; 8];
        let p = (l.hmget_key_ts)(
            map,
            es,
            key.as_ptr() as *mut c_void,
            8,
            &mut tmp,
            HM_BINARY,
        );
        t.rec("temp", tmp);
        t.rec("same_ptr", p == map);
        snap_map(t, "m", p, es, KeyKind::Raw);
        (l.hmfree_func)(a, es);
    });
}

#[test]
fn row32_get_ts_binary_present_absent() {
    let present = rand_keys(100, 8, 0x777);
    let absent = rand_keys(100, 8, 0x888);
    differential("row32", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        for k in &present {
            m.put_bin(k, 8);
        }
        for (i, k) in present.iter().enumerate() {
            t.rec(&format!("hit[{i}]"), m.get_ts(k.as_ptr() as *const c_void, 8, HM_BINARY));
        }
        for (i, k) in absent.iter().enumerate() {
            t.rec(&format!("miss[{i}]"), m.get_ts(k.as_ptr() as *const c_void, 8, HM_BINARY));
        }
        m.snap(t, "final", KeyKind::Raw);
        m.free();
    });
}

#[test]
fn row33_get_ts_string_strdup() {
    let present = str_strings(100, 20, 0x991);
    let absent = str_strings(100, 20, 0x992);
    differential("row33", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut owned = present.clone();
        let mut m = M::shmode(l, 16, SH_STRDUP);
        for s in owned.iter_mut() {
            m.put_str(s.as_ptr() as *const c_char, HM_STRING);
        }
        for (i, s) in owned.iter().enumerate() {
            t.rec(
                &format!("hit[{i}]"),
                m.get_ts(s.as_ptr() as *const c_void, 8, HM_STRING),
            );
        }
        for (i, s) in absent.iter().enumerate() {
            t.rec(
                &format!("miss[{i}]"),
                m.get_ts(s.as_ptr() as *const c_void, 8, HM_STRING),
            );
        }
        m.snap(t, "final", KeyKind::StrPtr);
        m.free();
        std::hint::black_box(&owned);
    });
}

#[test]
fn row34_get_nonts_wrapper() {
    let present = rand_keys(80, 8, 0xAAA1);
    let absent = rand_keys(80, 8, 0xAAA2);
    let strs = str_strings(80, 18, 0xAAA3);
    differential("row34", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        for k in &present {
            m.put_bin(k, 8);
        }
        for (i, k) in present.iter().enumerate() {
            t.rec(&format!("bhit[{i}]"), m.get(k.as_ptr() as *const c_void, 8, HM_BINARY));
            snap_map(t, &format!("bh{i}"), m.p, 16, KeyKind::Raw);
        }
        for (i, k) in absent.iter().enumerate() {
            t.rec(&format!("bmiss[{i}]"), m.get(k.as_ptr() as *const c_void, 8, HM_BINARY));
        }
        m.free();

        let mut owned = strs.clone();
        let mut sm = M::shmode(l, 16, SH_STRDUP);
        for s in owned.iter_mut() {
            sm.put_str(s.as_ptr() as *const c_char, HM_STRING);
        }
        for (i, s) in owned.iter().enumerate() {
            t.rec(&format!("shit[{i}]"), sm.get(s.as_ptr() as *const c_void, 8, HM_STRING));
        }
        sm.snap(t, "sfinal", KeyKind::StrPtr);
        sm.free();
        std::hint::black_box(&owned);

        // get on a NULL map through the wrapper
        let p = (l.hmget_key)(
            std::ptr::null_mut(),
            16,
            present[0].as_ptr() as *mut c_void,
            8,
            HM_BINARY,
        );
        snap_map(t, "nullget", p, 16, KeyKind::Raw);
        (l.hmfree_func)((p as *mut u8).sub(16) as *mut c_void, 16);
    });
}

// ===========================================================================
// rows 35-36 : hmput_default
// ===========================================================================

#[test]
fn row35_hmput_default() {
    differential("row35", |l, t| unsafe {
        for &es in &[8usize, 16, 24] {
            let a = (l.hmput_default)(std::ptr::null_mut(), es);
            snap_map(t, &format!("d{es}"), a, es, KeyKind::Raw);
            let b = (l.hmput_default)(a, es);
            t.rec("same", a == b);
            snap_map(t, &format!("d2_{es}"), b, es, KeyKind::Raw);
            (l.hmfree_func)((b as *mut u8).sub(es) as *mut c_void, es);

            // length == 0 array
            let arr = (l.arrgrowf)(std::ptr::null_mut(), es, 0, 1);
            let map = (arr as *mut u8).add(es) as *mut c_void;
            let c = (l.hmput_default)(map, es);
            snap_map(t, &format!("d3_{es}"), c, es, KeyKind::Raw);
            (l.hmfree_func)((c as *mut u8).sub(es) as *mut c_void, es);
        }
    });
}

#[test]
fn row36_hmput_default_then_puts() {
    let keys8 = rand_keys(40, 4, 0xBEE1);
    let keys16 = rand_keys(40, 8, 0xBEE2);
    differential("row36", |l, t| unsafe {
        (l.rand_seed)(SEED);
        for (es, ks, keysize) in [(8usize, &keys8, 4usize), (16, &keys16, 8)] {
            let mut m = M {
                l,
                p: (l.hmput_default)(std::ptr::null_mut(), es),
                es,
            };
            // set the default value bytes deterministically
            let d = elem(m.p, es, 0);
            for i in 0..es {
                *d.add(i) = 0xA0u8.wrapping_add(i as u8);
            }
            for (i, k) in ks.iter().enumerate() {
                t.rec(&format!("put{es}[{i}]"), m.put_bin(k, keysize));
            }
            m.snap(t, &format!("f{es}"), KeyKind::Raw);
            m.free();
        }
    });
}

// ===========================================================================
// rows 37-46 : hmdel_key
// ===========================================================================

#[test]
fn row37_del_single_element() {
    differential("row37", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        let k = [1u8, 2, 3, 4, 5, 6, 7, 8];
        m.put_bin(&k, 8);
        m.snap(t, "before", KeyKind::Raw);
        t.rec("del", m.del(k.as_ptr() as *const c_void, 8, 0, HM_BINARY));
        m.snap(t, "after", KeyKind::Raw);
        m.free();
    });
}

#[test]
fn row38_del_first_of_ten() {
    del_positional("row38", 0);
}

#[test]
fn row39_del_last_of_ten() {
    del_positional("row39", 9);
}

fn del_positional(label: &str, which: usize) {
    let keys = rand_keys(10, 8, 0xD00D);
    differential(label, |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        for k in &keys {
            m.put_bin(k, 8);
        }
        m.snap(t, "before", KeyKind::Raw);
        t.rec(
            "del",
            m.del(keys[which].as_ptr() as *const c_void, 8, 0, HM_BINARY),
        );
        m.snap(t, "after", KeyKind::Raw);
        for (i, k) in keys.iter().enumerate() {
            t.rec(
                &format!("get[{i}]"),
                m.get_ts(k.as_ptr() as *const c_void, 8, HM_BINARY),
            );
        }
        m.free();
    });
}

#[test]
fn row40_del_random_interleaved() {
    // pre-plan the op sequence so both libs run identically
    let keys = rand_keys(200, 8, 0xE0E0);
    let mut rng = Rng::new(0xF1F1);
    let ops: Vec<(u8, usize)> = (0..900)
        .map(|_| ((rng.next_u64() % 3) as u8, rng.below(keys.len())))
        .collect();
    differential("row40", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        for k in &keys {
            m.put_bin(k, 8);
        }
        for (i, (op, ki)) in ops.iter().enumerate() {
            let k = &keys[*ki];
            let r = match op {
                0 => m.put_bin(k, 8),
                1 => m.del(k.as_ptr() as *const c_void, 8, 0, HM_BINARY),
                _ => m.get_ts(k.as_ptr() as *const c_void, 8, HM_BINARY),
            };
            t.rec(&format!("op[{i}]"), (op, r));
            if i % 50 == 0 {
                m.snap(t, &format!("s{i}"), KeyKind::Raw);
            }
        }
        m.snap(t, "final", KeyKind::Raw);
        m.free();
    });
}

#[test]
fn row41_del_shrink_chain() {
    let keys = rand_keys(64, 8, 0x5151);
    differential("row41", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        for k in &keys {
            m.put_bin(k, 8);
        }
        m.snap(t, "full", KeyKind::Raw);
        for (i, k) in keys.iter().enumerate().take(60) {
            t.rec(
                &format!("del[{i}]"),
                m.del(k.as_ptr() as *const c_void, 8, 0, HM_BINARY),
            );
            m.snap(t, &format!("s{i}"), KeyKind::Raw);
        }
        m.free();
    });
}

#[test]
fn row42_del_then_reinsert_tombstone_reuse() {
    let keys = rand_keys(40, 8, 0x6161);
    differential("row42", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut m = M::none(l, 16);
        for k in &keys {
            m.put_bin(k, 8);
        }
        for round in 0..5 {
            for (i, k) in keys.iter().enumerate().take(10) {
                t.rec(
                    &format!("d{round}_{i}"),
                    m.del(k.as_ptr() as *const c_void, 8, 0, HM_BINARY),
                );
            }
            for (i, k) in keys.iter().enumerate().take(10) {
                t.rec(&format!("p{round}_{i}"), m.put_bin(k, 8));
            }
            m.snap(t, &format!("r{round}"), KeyKind::Raw);
        }
        m.free();
    });
}

fn str_del_test(label: &str, shmode: c_int, take: usize, kk: KeyKind) {
    let strs = str_strings(100, 24, 0x7171);
    differential(label, |l, t| unsafe {
        (l.rand_seed)(SEED);
        let mut owned = strs.clone();
        let mut m = M::shmode(l, 16, shmode);
        for s in owned.iter_mut() {
            m.put_str(s.as_ptr() as *const c_char, HM_STRING);
        }
        m.snap(t, "before", kk);
        for (i, s) in owned.iter().enumerate().take(take) {
            t.rec(
                &format!("del[{i}]"),
                m.del(s.as_ptr() as *const c_void, 8, 0, HM_STRING),
            );
            if i % 10 == 0 {
                m.snap(t, &format!("s{i}"), kk);
            }
        }
        m.snap(t, "after", kk);
        for (i, s) in owned.iter().enumerate() {
            t.rec(
                &format!("get[{i}]"),
                m.get_ts(s.as_ptr() as *const c_void, 8, HM_STRING),
            );
        }
        m.free();
        std::hint::black_box(&owned);
    });
}

#[test]
fn row43_del_strdup_all() {
    str_del_test("row43", SH_STRDUP, 100, KeyKind::StrPtrNoTk);
}

#[test]
fn row44_del_arena_half() {
    str_del_test("row44", SH_ARENA, 50, KeyKind::StrPtrNoTk);
}

#[test]
fn row45_del_default_half() {
    str_del_test("row45", SH_DEFAULT, 50, KeyKind::StrPtrNoTk);
}

#[test]
fn row46_del_keyoffset_nonzero() {
    // elemsize 16, key stored at offset 8.  We drive hmput_key with keyoffset 0
    // (it hardcodes 0) but delete with keyoffset 8 -- exercising the explicit
    // keyoffset parameter of hmdel_key.  To keep both sides consistent we build
    // the table by hand: put at offset 0, then also mirror the key at offset 8.
    let keys = rand_keys(20, 8, 0x8181);
    differential("row46", |l, t| unsafe {
        (l.rand_seed)(SEED);
        let es = 16usize;
        let mut m = M::none(l, es);
        for k in &keys {
            let idx = m.put_bin(k, 8);
            // mirror the key at offset 8 so keyoffset=8 lookups find it too
            let e = elem(m.p, es, (idx + 1) as usize);
            std::ptr::copy_nonoverlapping(k.as_ptr(), e.add(8), 8);
        }
        m.snap(t, "before", KeyKind::Raw);
        // delete using keyoffset 8
        for (i, k) in keys.iter().enumerate().take(5) {
            t.rec(
                &format!("del[{i}]"),
                m.del(k.as_ptr() as *const c_void, 8, 8, HM_BINARY),
            );
            m.snap(t, &format!("s{i}"), KeyKind::Raw);
        }
        m.free();
    });
}

// ===========================================================================
// row 47 : hmfree_func over all table shapes
// ===========================================================================

#[test]
fn row47_hmfree_all_shapes() {
    let strs = str_strings(30, 20, 0x9191);
    let keys = rand_keys(30, 8, 0x9192);
    differential("row47", |l, t| unsafe {
        (l.rand_seed)(SEED);
        for &sm in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            let mut owned = strs.clone();
            let mut m = M::shmode(l, 16, sm);
            if sm == SH_NONE {
                // avoid strcmp on garbage: single distinct insert only
                self_put(&mut m, owned[0].as_ptr() as *const c_char);
            } else {
                for s in owned.iter_mut() {
                    m.put_str(s.as_ptr() as *const c_char, HM_STRING);
                }
            }
            m.snap(
                t,
                &format!("m{sm}"),
                if sm == SH_NONE {
                    KeyKind::Raw
                } else {
                    KeyKind::StrPtr
                },
            );
            m.free();
            t.rec(&format!("freed{sm}"), sm);
            std::hint::black_box(&owned);
        }
        // binary map
        let mut m = M::none(l, 16);
        for k in &keys {
            m.put_bin(k, 8);
        }
        m.free();
        t.rec("freed_bin", 1);
        // array without hash table
        let a = (l.arrgrowf)(std::ptr::null_mut(), 16, 0, 1);
        (*((a as *mut u8).sub(HDR) as *mut Header)).length = 1;
        (l.hmfree_func)(a, 16);
        t.rec("freed_notable", 1);
    });
}

// ===========================================================================
// rows 48-53 : string arena
// ===========================================================================

#[test]
fn row48_stralloc_short() {
    differential("row48", |l, t| unsafe {
        let mut a = Arena::new();
        let mut s = cstring(b"short string");
        let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        t.rec_str("content", cstr(p));
        snap_arena(t, "a", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "after_reset", &a);
    });
}

#[test]
fn row49_stralloc_oversized_fresh() {
    differential("row49", |l, t| unsafe {
        let mut a = Arena::new();
        let mut s = cstring(&vec![b'Q'; 2000]);
        let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
        t.rec("len", cstr(p).len());
        t.rec("eq", cstr(p) == cstr(s.as_ptr() as *const c_char));
        snap_arena(t, "a", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "after_reset", &a);
    });
}

#[test]
fn row50_stralloc_oversized_after_populated() {
    differential("row50", |l, t| unsafe {
        let mut a = Arena::new();
        let mut small = cstring(b"tiny");
        let p1 = (l.stralloc)(&mut a, small.as_mut_ptr() as *mut c_char);
        t.rec_str("p1", cstr(p1));
        snap_arena(t, "a1", &a);
        let mut big = cstring(&vec![b'B'; 5000]);
        let p2 = (l.stralloc)(&mut a, big.as_mut_ptr() as *mut c_char);
        t.rec("p2len", cstr(p2).len());
        snap_arena(t, "a2", &a);
        // p1 must still be readable
        t.rec_str("p1_again", cstr(p1));
        let mut m = cstring(b"medium string here");
        let p3 = (l.stralloc)(&mut a, m.as_mut_ptr() as *mut c_char);
        t.rec_str("p3", cstr(p3));
        snap_arena(t, "a3", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "a4", &a);
    });
}

#[test]
fn row51_stralloc_500_random() {
    let mut rng = Rng::new(0xAABB);
    let strs: Vec<Vec<u8>> = (0..500)
        .map(|_| {
            let n = rng.below(100);
            cstring(&rng.ascii(n))
        })
        .collect();
    differential("row51", |l, t| unsafe {
        let mut a = Arena::new();
        let mut ptrs = Vec::new();
        for (i, s) in strs.iter().enumerate() {
            let mut s = s.clone();
            let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            ptrs.push((p, cstr(s.as_ptr() as *const c_char)));
            if i % 25 == 0 {
                snap_arena(t, &format!("a{i}"), &a);
            }
        }
        for (i, (p, expect)) in ptrs.iter().enumerate() {
            t.rec_str(&format!("read[{i}]"), format!("{} {}", cstr(*p), expect));
        }
        snap_arena(t, "final", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "reset", &a);
    });
}

#[test]
fn row52_stralloc_block_saturation() {
    differential("row52", |l, t| unsafe {
        let mut a = Arena::new();
        // each allocation is larger than the current remaining, forcing a grow
        for shift in 9..22u32 {
            let n = 1usize << shift;
            let mut s = cstring(&vec![b'k'; n]);
            let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            t.rec(&format!("len[{shift}]"), cstr(p).len());
            snap_arena(t, &format!("a{shift}"), &a);
        }
        // hammer it so `block` reaches saturation
        for i in 0..40 {
            let mut s = cstring(&vec![b'z'; 400_000]);
            let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
            t.rec(&format!("big[{i}]"), cstr(p).len());
            t.rec(&format!("block[{i}]"), a.block);
        }
        snap_arena(t, "final", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "reset", &a);
    });
}

#[test]
fn row53_arena_reuse_and_idempotent_reset() {
    differential("row53", |l, t| unsafe {
        let mut a = Arena::new();
        snap_arena(t, "fresh", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "reset_fresh", &a);
        (l.strreset)(&mut a);
        snap_arena(t, "reset_twice", &a);
        for round in 0..4 {
            for i in 0..20 {
                let mut s = cstring(format!("round{round}_item{i}").as_bytes());
                let p = (l.stralloc)(&mut a, s.as_mut_ptr() as *mut c_char);
                t.rec_str(&format!("p{round}_{i}"), cstr(p));
            }
            snap_arena(t, &format!("a{round}"), &a);
            (l.strreset)(&mut a);
            snap_arena(t, &format!("r{round}"), &a);
        }
    });
}

// ===========================================================================
// rows 54-55 : strkey / str_dups  (binary-style stdout comparison)
// ===========================================================================

#[test]
fn row54_strkey() {
    differential("row54", |l, t| unsafe {
        for &n in &[0i32, 1, 9, 10, 99, 100, 12345, i32::MAX, -1, i32::MIN, -999] {
            t.rec_str(&format!("k{n}"), cstr((l.strkey)(n as c_int)));
        }
    });
}

// row 55 (`str_dups` stdout) lives in `tests/stdout_diff.rs`: capturing fd 1
// requires a test binary with a single test, see the note there.

// ===========================================================================
// rows 56-57 : mixed pipelines
// ===========================================================================

#[test]
fn row56_mixed_strdup_pipeline() {
    let strs = str_strings(120, 30, 0xBEEF1);
    let mut rng = Rng::new(0xBEEF2);
    let ops: Vec<(u8, usize)> = (0..300)
        .map(|_| ((rng.next_u64() % 3) as u8, rng.below(strs.len())))
        .collect();
    differential("row56", |l, t| unsafe {
        (l.rand_seed)(0xDEADBEEF);
        let mut owned = strs.clone();
        let mut m = M::shmode(l, 24, SH_STRDUP);
        for (i, (op, ki)) in ops.iter().enumerate() {
            let s = &mut owned[*ki];
            let r = match op {
                0 => m.put_str(s.as_ptr() as *const c_char, HM_STRING),
                1 => m.del(s.as_ptr() as *const c_void, 8, 0, HM_STRING),
                _ => m.get_ts(s.as_ptr() as *const c_void, 8, HM_STRING),
            };
            t.rec(&format!("op[{i}]"), (op, r));
            if i % 10 == 0 {
                m.snap(t, &format!("s{i}"), KeyKind::StrPtrNoTk);
            }
        }
        m.free();
        std::hint::black_box(&owned);
    });
}

#[test]
fn row57_mixed_binary_pipeline() {
    let keys = rand_keys(120, 8, 0xCAFE1);
    let mut rng = Rng::new(0xCAFE2);
    let ops: Vec<(u8, usize)> = (0..300)
        .map(|_| ((rng.next_u64() % 4) as u8, rng.below(keys.len())))
        .collect();
    differential("row57", |l, t| unsafe {
        (l.rand_seed)(0x12345678);
        let mut m = M::none(l, 24);
        for (i, (op, ki)) in ops.iter().enumerate() {
            let k = &keys[*ki];
            let r = match op {
                0 | 3 => m.put_bin(k, 8),
                1 => m.del(k.as_ptr() as *const c_void, 8, 0, HM_BINARY),
                _ => m.get(k.as_ptr() as *const c_void, 8, HM_BINARY),
            };
            t.rec(&format!("op[{i}]"), (op, r));
            if i % 10 == 0 {
                m.snap(t, &format!("s{i}"), KeyKind::Raw);
            }
        }
        m.free();
    });
}
