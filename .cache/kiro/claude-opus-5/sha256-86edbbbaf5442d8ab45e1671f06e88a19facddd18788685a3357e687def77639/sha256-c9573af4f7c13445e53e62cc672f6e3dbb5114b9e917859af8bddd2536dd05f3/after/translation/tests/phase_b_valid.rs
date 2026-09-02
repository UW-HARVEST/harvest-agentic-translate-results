//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every scenario is a pure function of the
//! `Impl` it is handed, so the *same* scenario is replayed against the C `.so`
//! and the Rust `.so` and the resulting digests must be byte-identical.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Scenario plumbing
// ---------------------------------------------------------------------------

fn run_both<F: Fn(&Impl) -> Vec<u8>>(what: &str, f: F) {
    with_libs(|l| {
        let c = f(&l.c);
        let r = f(&l.r);
        assert_same(what, &c, &r);
    });
}

/// Value region of an element for a given key representation.
fn val_off(key_repr: KeyRepr) -> usize {
    match key_repr {
        KeyRepr::Inline(ks) => ks,
        KeyRepr::Pointer => 8,
    }
}

struct Map<'a> {
    im: &'a Impl,
    t: *mut c_void,
    elemsize: usize,
    keysize: usize,
    key_repr: KeyRepr,
    /// `temp_key` is only meaningfully defined right after a string-mode
    /// `hmput_key`; every other entry point leaves it as stale or
    /// uninitialised `realloc` memory. Digests include it only when true.
    tk: bool,
    /// Key buffers are kept alive for the lifetime of the map: in
    /// `STBDS_SH_DEFAULT` mode the map stores the *caller's* pointer.
    keep: Vec<Vec<u8>>,
}

impl<'a> Map<'a> {
    fn new(im: &'a Impl, elemsize: usize, keysize: usize, key_repr: KeyRepr) -> Map<'a> {
        Map { im, t: std::ptr::null_mut(), elemsize, keysize, key_repr, tk: false, keep: Vec::new() }
    }

    fn from_shmode(im: &'a Impl, elemsize: usize, keysize: usize, key_repr: KeyRepr, mode: c_int) -> Map<'a> {
        let t = unsafe { (im.shmode_func)(elemsize, mode) };
        Map { im, t, elemsize, keysize, key_repr, tk: false, keep: Vec::new() }
    }

    /// Stable, owned copy of `key` usable as the `void *key` argument.
    fn own_key(&mut self, key: &[u8]) -> *mut c_void {
        self.keep.push(key.to_vec());
        self.keep.last_mut().unwrap().as_mut_ptr() as *mut c_void
    }

    unsafe fn temp(&self) -> isize {
        unsafe { (*header(hash_to_arr(self.t, self.elemsize))).temp }
    }

    unsafe fn length(&self) -> usize {
        unsafe { (*header(hash_to_arr(self.t, self.elemsize))).length }
    }

    /// `hmput(t, k, v)` / `shput(t, k, v)` macro expansion.
    unsafe fn put(&mut self, key: &[u8], val: &[u8], mode: c_int) -> isize {
        unsafe {
            let kp = self.own_key(key);
            self.tk = mode >= 1 && self.key_repr == KeyRepr::Pointer;
            self.t = (self.im.hmput_key)(self.t, self.elemsize, kp, self.keysize, mode);
            let temp = self.temp();
            let e = (self.t as *mut u8).offset(temp * self.elemsize as isize);
            // The macro assigns `.key` too; for inline keys that repeats what
            // hmput_key already memcpy'd, for pointer keys `shput` leaves the
            // stored pointer alone.
            if let KeyRepr::Inline(ks) = self.key_repr {
                std::ptr::copy_nonoverlapping(key.as_ptr(), e, ks);
            }
            let vo = val_off(self.key_repr);
            let n = self.elemsize - vo;
            assert!(val.len() >= n);
            std::ptr::copy_nonoverlapping(val.as_ptr(), e.add(vo), n);
            temp
        }
    }

    /// `hmgeti(t,k)` + read of `.value`, i.e. `hmget`.
    unsafe fn get(&mut self, key: &[u8], mode: c_int) -> (isize, Vec<u8>) {
        unsafe {
            let kp = self.own_key(key);
            self.tk = false;
            self.t = (self.im.hmget_key)(self.t, self.elemsize, kp, self.keysize, mode);
            let temp = self.temp();
            let e = (self.t as *mut u8).offset(temp * self.elemsize as isize);
            let vo = val_off(self.key_repr);
            let v = std::slice::from_raw_parts(e.add(vo), self.elemsize - vo).to_vec();
            (temp, v)
        }
    }

    /// `hmgeti_ts(t,k,temp)`.
    unsafe fn get_ts(&mut self, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let kp = self.own_key(key);
            self.tk = false;
            let mut temp: isize = 0x5555_5555;
            self.t = (self.im.hmget_key_ts)(self.t, self.elemsize, kp, self.keysize, &mut temp, mode);
            temp
        }
    }

    /// `hmdel(t,k)` — returns the macro's result (`temp`, or 0 when t is NULL).
    unsafe fn del(&mut self, key: &[u8], mode: c_int) -> isize {
        unsafe {
            let kp = self.own_key(key);
            self.tk = false;
            self.t = (self.im.hmdel_key)(self.t, self.elemsize, kp, self.keysize, 0, mode);
            if self.t.is_null() { 0 } else { self.temp() }
        }
    }

    unsafe fn digest(&self) -> Vec<u8> {
        unsafe { digest_map(self.t, self.elemsize, self.key_repr, self.tk) }
    }

    unsafe fn snap(&self, d: &mut Digest, tag: &str) {
        unsafe {
            d.tag(tag);
            let m = self.digest();
            d.bytes(&m);
        }
    }

    unsafe fn free(&mut self) {
        unsafe {
            if !self.t.is_null() {
                (self.im.hmfree_func)(hash_to_arr(self.t, self.elemsize), self.elemsize);
                self.t = std::ptr::null_mut();
            }
        }
    }
}

const DEFAULT_SEED: usize = 0x3141_5926;

// ---------------------------------------------------------------------------
// Rows 1-6 — stbds_hash_bytes
// ---------------------------------------------------------------------------

fn hash_bytes_row(im: &Impl, lens: &[usize], rng_seed: u64, iters: usize) -> Vec<u8> {
    unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(rng_seed);
        let seeds: [usize; 5] = [DEFAULT_SEED, 0, 1, usize::MAX, 0xdead_beef_cafe_babe];
        for &len in lens {
            for _ in 0..iters {
                let mut buf = rng.bytes(len.max(1));
                for &s in &seeds {
                    let h = (im.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s);
                    d.usize(h);
                }
                let s = rng.next_u64() as usize;
                d.usize((im.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, s));
                // force the sign-extension corner: top bit set in tail byte 3
                if len >= 4 {
                    buf[3] |= 0x80;
                    buf[len - 1] |= 0x80;
                    d.usize((im.hash_bytes)(buf.as_mut_ptr() as *mut c_void, len, DEFAULT_SEED));
                }
            }
        }
        d.0
    }
}

#[test]
fn row01_hash_bytes_len0() {
    run_both("row01", |im| hash_bytes_row(im, &[0], 0xA1, 64));
}

#[test]
fn row02_hash_bytes_tail_only() {
    run_both("row02", |im| hash_bytes_row(im, &[1, 2, 3, 4, 5, 6, 7], 0xA2, 128));
}

#[test]
fn row03_hash_bytes_one_word() {
    run_both("row03", |im| hash_bytes_row(im, &[8], 0xA3, 256));
}

#[test]
fn row04_hash_bytes_word_plus_tail() {
    run_both("row04", |im| hash_bytes_row(im, &[9, 10, 11, 12, 13, 14, 15], 0xA4, 128));
}

#[test]
fn row05_hash_bytes_multiword() {
    run_both("row05", |im| hash_bytes_row(im, &[16, 24, 32], 0xA5, 128));
}

#[test]
fn row06_hash_bytes_large_random() {
    run_both("row06", |im| {
        let lens: Vec<usize> = (33..=256).collect();
        hash_bytes_row(im, &lens, 0xA6, 4)
    });
}

// ---------------------------------------------------------------------------
// Rows 7-10 — stbds_hash_string
// ---------------------------------------------------------------------------

fn hash_string_row(im: &Impl, lens: &[usize], alphabet: &[u8], rng_seed: u64, iters: usize) -> Vec<u8> {
    unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(rng_seed);
        let seeds: [usize; 5] = [DEFAULT_SEED, 0, 1, usize::MAX, 0x0123_4567_89ab_cdef];
        for &len in lens {
            for _ in 0..iters {
                let mut s = rand_cstring(&mut rng, len, alphabet);
                for &sd in &seeds {
                    d.usize((im.hash_string)(s.as_mut_ptr() as *mut c_char, sd));
                }
                let sd = rng.next_u64() as usize;
                d.usize((im.hash_string)(s.as_mut_ptr() as *mut c_char, sd));
            }
        }
        d.0
    }
}

#[test]
fn row07_hash_string_empty() {
    run_both("row07", |im| hash_string_row(im, &[0], ASCII, 0xB1, 8));
}

#[test]
fn row08_hash_string_ascii() {
    run_both("row08", |im| {
        let lens: Vec<usize> = (1..=64).collect();
        hash_string_row(im, &lens, ASCII, 0xB2, 16)
    });
}

#[test]
fn row09_hash_string_high_bit() {
    run_both("row09", |im| {
        let lens: Vec<usize> = (1..=64).collect();
        hash_string_row(im, &lens, &HIGH_BIT, 0xB3, 16)
    });
}

#[test]
fn row10_hash_string_long() {
    run_both("row10", |im| {
        let mixed: Vec<u8> = ASCII.iter().copied().chain(HIGH_BIT.iter().copied()).collect();
        let lens: Vec<usize> = (256..=1024).step_by(37).collect();
        hash_string_row(im, &lens, &mixed, 0xB4, 8)
    });
}

// ---------------------------------------------------------------------------
// Rows 11-15 — stbds_arrgrowf / stbds_arrfreef
// ---------------------------------------------------------------------------

#[test]
fn row11_arrgrowf_fresh_matrix() {
    run_both("row11", |im| unsafe {
        let mut d = Digest::default();
        for &elemsize in &[1usize, 2, 4, 8, 16, 64] {
            for &addlen in &[0usize, 1, 2, 3, 4, 5, 17, 100] {
                for &min_cap in &[0usize, 1, 3, 4, 7, 64] {
                    let a = (im.arrgrowf)(std::ptr::null_mut(), elemsize, addlen, min_cap);
                    d.tag("fresh");
                    d.usize(elemsize);
                    d.usize(addlen);
                    d.usize(min_cap);
                    d.u8((!a.is_null()) as u8);
                    if a.is_null() {
                        // `min_cap <= stbds_arrcap(NULL) == 0` early-out
                        continue;
                    }
                    let h = header(a);
                    d.usize((*h).length);
                    d.usize((*h).capacity);
                    d.isize((*h).temp);
                    d.u8((!(*h).hash_table.is_null()) as u8);
                    (im.arrfreef)(a);
                }
            }
        }
        d.0
    });
}

#[test]
fn row12_arrgrowf_early_out() {
    run_both("row12", |im| unsafe {
        let mut d = Digest::default();
        for &elemsize in &[1usize, 4, 8, 16] {
            let a = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 10);
            let cap = (*header(a)).capacity;
            // fill the payload so the digest sees stable bytes
            std::ptr::write_bytes(a as *mut u8, 0xA5, elemsize * cap);
            (*header(a)).length = 3;
            for &req in &[0usize, 1, 3, cap - 1, cap] {
                let b = (im.arrgrowf)(a, elemsize, 0, req);
                d.tag("early");
                d.usize(elemsize);
                d.usize(req);
                d.u8((a == b) as u8);
                d.bytes(&digest_array(b, elemsize, cap));
            }
            (im.arrfreef)(a);
        }
        d.0
    });
}

#[test]
fn row13_row14_arrgrowf_growth_branches() {
    run_both("row13_14", |im| unsafe {
        let mut d = Digest::default();
        for &elemsize in &[1usize, 4, 8, 24] {
            for &(addlen, min_cap) in &[
                (1usize, 0usize),  // min_len = 4 -> < 2*cap(4)=8 -> cap 8
                (3, 0),
                (4, 0),
                (5, 0),            // min_len 9 >= 8 -> cap 9
                (100, 0),
                (0, 5),
                (0, 9),
                (0, 1000),
            ] {
                let mut a = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
                (*header(a)).length = 4;
                std::ptr::write_bytes(a as *mut u8, 0x3C, elemsize * 4);
                a = (im.arrgrowf)(a, elemsize, addlen, min_cap);
                d.tag("grow");
                d.usize(elemsize);
                d.usize(addlen);
                d.usize(min_cap);
                d.bytes(&digest_array(a, elemsize, 4));
                (im.arrfreef)(a);
            }
        }
        d.0
    });
}

#[test]
fn row15_arrgrowf_growth_chain() {
    run_both("row15", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0xC1);
        for &elemsize in &[1usize, 4, 8, 16, 40] {
            let mut a = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            for _ in 0..10 {
                let addlen = rng.range(1, 9);
                (*header(a)).length = (*header(a)).capacity.min((*header(a)).length + 1);
                a = (im.arrgrowf)(a, elemsize, addlen, 0);
                d.tag("chain");
                d.usize(elemsize);
                d.usize(addlen);
                d.usize((*header(a)).length);
                d.usize((*header(a)).capacity);
                d.isize((*header(a)).temp);
            }
            (im.arrfreef)(a);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 16 — stbds_rand_seed and the seed LCG
// ---------------------------------------------------------------------------

#[test]
fn row16_rand_seed_lcg() {
    run_both("row16", |im| unsafe {
        let mut d = Digest::default();
        for &s in &[0usize, 1, DEFAULT_SEED, usize::MAX, 0x1234_5678_9abc_def0] {
            (im.rand_seed)(s);
            for _ in 0..12 {
                // every fresh table consumes one LCG step
                let t = (im.shmode_func)(16, 0);
                let raw = hash_to_arr(t, 16);
                let ht = (*header(raw)).hash_table as *const HashIndex;
                d.usize((*ht).seed);
                (im.hmfree_func)(raw, 16);
            }
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 17 — stbds_hmput_default
// ---------------------------------------------------------------------------

#[test]
fn row17_hmput_default() {
    run_both("row17", |im| unsafe {
        let mut d = Digest::default();
        (im.rand_seed)(DEFAULT_SEED);
        let elemsize = 8usize;

        // a == NULL
        let t = (im.hmput_default)(std::ptr::null_mut(), elemsize);
        d.tag("null");
        d.bytes(&digest_map(t, elemsize, KeyRepr::Inline(4), false));
        // set the default value like hmdefault does: (t)[-1].value = v
        let dm1 = (t as *mut u8).offset(-(elemsize as isize));
        std::ptr::copy_nonoverlapping([0x11u8, 0x22, 0x33, 0x44].as_ptr(), dm1.add(4), 4);
        d.bytes(&digest_map(t, elemsize, KeyRepr::Inline(4), false));

        // already length > 0: unchanged
        let t2 = (im.hmput_default)(t, elemsize);
        d.tag("again");
        d.u8((t == t2) as u8);
        d.bytes(&digest_map(t2, elemsize, KeyRepr::Inline(4), false));
        (im.hmfree_func)(hash_to_arr(t2, elemsize), elemsize);

        // raw array with length == 0
        let raw = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
        let hp = (raw as *mut u8).add(elemsize) as *mut c_void;
        let t3 = (im.hmput_default)(hp, elemsize);
        d.tag("len0");
        d.bytes(&digest_map(t3, elemsize, KeyRepr::Inline(4), false));
        (im.hmfree_func)(hash_to_arr(t3, elemsize), elemsize);
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 18-24 — stbds_hmput_key, binary mode
// ---------------------------------------------------------------------------

fn i32key(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

fn i32val(v: i32) -> Vec<u8> {
    v.to_ne_bytes().to_vec()
}

fn put_seq(im: &Impl, keys: &[i32], mode: c_int) -> Vec<u8> {
    unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
        for (i, &k) in keys.iter().enumerate() {
            let t = m.put(&i32key(k), &i32val(i as i32 * 7 + 1), mode);
            d.tag("put");
            d.isize(t);
            m.snap(&mut d, "st");
        }
        for &k in keys {
            let (t, v) = m.get(&i32key(k), mode);
            d.tag("get");
            d.isize(t);
            d.bytes(&v);
        }
        m.free();
        d.0
    }
}

#[test]
fn row18_hmput_one() {
    run_both("row18", |im| put_seq(im, &[42], 0));
}

#[test]
fn row19_hmput_crosses_grow_threshold() {
    run_both("row19", |im| {
        let mut out = Digest::default();
        for n in 2..=10usize {
            let keys: Vec<i32> = (0..n as i32).map(|i| i * 3 + 1).collect();
            out.bytes(&put_seq(im, &keys, 0));
        }
        out.0
    });
}

#[test]
fn row20_hmput_many() {
    run_both("row20", |im| {
        let mut out = Digest::default();
        let mut rng = Rng::new(0xD1);
        for &n in &[100usize, 1000] {
            let mut keys: Vec<i32> = Vec::new();
            let mut seen = std::collections::HashSet::new();
            while keys.len() < n {
                let k = rng.i32();
                if seen.insert(k) {
                    keys.push(k);
                }
            }
            out.bytes(&put_seq(im, &keys, 0));
        }
        out.0
    });
}

#[test]
fn row21_hmput_existing_keys() {
    run_both("row21", |im| unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut rng = Rng::new(0xD2);
        let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
        let keys: Vec<i32> = (0..40).map(|_| rng.i32()).collect();
        for (i, &k) in keys.iter().enumerate() {
            m.put(&i32key(k), &i32val(i as i32), 0);
        }
        // re-put every key several times in shuffled order
        for round in 0..4 {
            for i in 0..keys.len() {
                let j = rng.below(keys.len());
                let k = keys[j];
                let t = m.put(&i32key(k), &i32val(round * 1000 + i as i32), 0);
                d.tag("reput");
                d.isize(t);
                d.usize(m.length());
            }
        }
        m.snap(&mut d, "final");
        m.free();
        d.0
    });
}

#[test]
fn row22_hmput_keysizes() {
    run_both("row22", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0xD3);
        for &keysize in &[1usize, 2, 4, 8, 12, 16, 32] {
            let elemsize = keysize + 8;
            (im.rand_seed)(DEFAULT_SEED);
            let mut m = Map::new(im, elemsize, keysize, KeyRepr::Inline(keysize));
            let mut keys: Vec<Vec<u8>> = Vec::new();
            let n = if keysize == 1 { 64 } else { 120 };
            let mut seen = std::collections::HashSet::new();
            while keys.len() < n {
                let k = rng.bytes(keysize);
                if seen.insert(k.clone()) {
                    keys.push(k);
                }
            }
            for (i, k) in keys.iter().enumerate() {
                let t = m.put(k, &(i as u64).to_ne_bytes(), 0);
                d.tag("put");
                d.usize(keysize);
                d.isize(t);
            }
            m.snap(&mut d, "after-puts");
            for k in &keys {
                let (t, v) = m.get(k, 0);
                d.isize(t);
                d.bytes(&v);
            }
            // absent keys
            for _ in 0..32 {
                let k = rng.bytes(keysize);
                let (t, v) = m.get(&k, 0);
                d.isize(t);
                d.bytes(&v);
            }
            m.free();
        }
        d.0
    });
}

#[test]
fn row23_hmput_low_entropy_keys() {
    run_both("row23", |im| unsafe {
        let mut d = Digest::default();
        for &keysize in &[1usize, 4, 8, 16] {
            let elemsize = keysize + 8;
            (im.rand_seed)(DEFAULT_SEED);
            let mut m = Map::new(im, elemsize, keysize, KeyRepr::Inline(keysize));
            // all-zero, all-0xFF and single-bit keys maximise bucket collisions
            let mut keys: Vec<Vec<u8>> = vec![vec![0u8; keysize], vec![0xFFu8; keysize]];
            for byte in 0..keysize {
                for bit in 0..8 {
                    let mut k = vec![0u8; keysize];
                    k[byte] = 1 << bit;
                    keys.push(k);
                }
            }
            keys.dedup();
            for (i, k) in keys.iter().enumerate() {
                let t = m.put(k, &(i as u64).to_ne_bytes(), 0);
                d.isize(t);
            }
            m.snap(&mut d, "low-entropy");
            for k in &keys {
                let (t, v) = m.get(k, 0);
                d.isize(t);
                d.bytes(&v);
            }
            m.free();
        }
        d.0
    });
}

#[test]
fn row24_hmput_negative_mode() {
    run_both("row24", |im| {
        let mut out = Digest::default();
        let mut rng = Rng::new(0xD4);
        let keys: Vec<i32> = (0..50).map(|_| rng.i32()).collect();
        for &mode in &[0i32, -1, -7, i32::MIN] {
            out.tag("mode");
            out.i64(mode as i64);
            out.bytes(&put_seq(im, &keys, mode));
        }
        out.0
    });
}

// ---------------------------------------------------------------------------
// Rows 25-30 — stbds_hmput_key, string modes
// ---------------------------------------------------------------------------

const SH_NONE: c_int = 0;
const SH_DEFAULT: c_int = 1;
const SH_STRDUP: c_int = 2;
const SH_ARENA: c_int = 3;

/// String-map scenario: `elemsize = 16` (`char *key; size_t value;`).
fn string_map_row(
    im: &Impl,
    sh_mode: Option<c_int>,
    put_mode: c_int,
    rng_seed: u64,
    lens: &[usize],
    n: usize,
) -> Vec<u8> {
    unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut rng = Rng::new(rng_seed);
        let mut m = match sh_mode {
            Some(sm) => Map::from_shmode(im, 16, 8, KeyRepr::Pointer, sm),
            None => Map::new(im, 16, 8, KeyRepr::Pointer),
        };
        let mut keys: Vec<Vec<u8>> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        while keys.len() < n {
            let len = lens[rng.below(lens.len())];
            let k = rand_cstring(&mut rng, len, ASCII);
            if seen.insert(k.clone()) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let t = m.put(k, &(i as u64 * 3 + 1).to_ne_bytes(), put_mode);
            d.tag("put");
            d.isize(t);
            m.snap(&mut d, "st");
        }
        // duplicates, from *distinct* buffers with equal contents
        for round in 0..3u64 {
            for i in 0..keys.len() {
                let j = rng.below(keys.len());
                let k = keys[j].clone();
                let t = m.put(&k, &(round * 100 + i as u64).to_ne_bytes(), put_mode);
                d.tag("dup");
                d.isize(t);
                d.usize(m.length());
                m.snap(&mut d, "st");
            }
        }
        for k in &keys {
            let (t, v) = m.get(k, put_mode);
            d.tag("get");
            d.isize(t);
            d.bytes(&v);
        }
        for _ in 0..16 {
            let len = rng.range(1, 12);
            let k = rand_cstring(&mut rng, len, ASCII);
            let (t, v) = m.get(&k, put_mode);
            d.tag("miss");
            d.isize(t);
            d.bytes(&v);
        }
        m.snap(&mut d, "final");
        m.free();
        d.0
    }
}

#[test]
fn row25_string_implicit_sh_default() {
    run_both("row25", |im| string_map_row(im, None, 1, 0xE1, &[1, 2, 3, 7, 8, 9, 15, 16, 31], 40));
}

#[test]
fn row26_string_sh_strdup() {
    run_both("row26", |im| string_map_row(im, Some(SH_STRDUP), 1, 0xE2, &[0, 1, 5, 12, 30], 40));
}

#[test]
fn row27_string_sh_arena() {
    run_both("row27", |im| {
        let mut out = Digest::default();
        out.bytes(&string_map_row(im, Some(SH_ARENA), 1, 0xE3, &[0, 1, 5, 12, 30], 40));
        // long keys force arena block growth inside the map
        out.bytes(&string_map_row(im, Some(SH_ARENA), 1, 0xE4, &[400, 500, 600, 1200], 12));
        out.0
    });
}

#[test]
fn row28_string_sh_none_memcpy_branch() {
    // `shmode_func(elemsize, STBDS_SH_NONE)` + `mode = 1`: the `switch` in
    // hmput_key falls through to `default:` and memcpy's the raw key bytes even
    // though the *lookup* path uses strcmp. Only distinct keys are inserted:
    // any hash match would make C deref the inline bytes as a `char *`.
    run_both("row28", |im| unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut rng = Rng::new(0xE5);
        let mut m = Map::from_shmode(im, 16, 8, KeyRepr::Inline(8), SH_NONE);
        let mut seen = std::collections::HashSet::new();
        let mut keys: Vec<Vec<u8>> = Vec::new();
        while keys.len() < 30 {
            let k = rand_cstring(&mut rng, 7, ASCII);
            if seen.insert(k.clone()) {
                keys.push(k);
            }
        }
        for (i, k) in keys.iter().enumerate() {
            let t = m.put(k, &(i as u64).to_ne_bytes(), 1);
            d.tag("put");
            d.isize(t);
            m.snap(&mut d, "st");
        }
        m.free();
        d.0
    });
}

#[test]
fn row29_string_mode_out_of_range() {
    run_both("row29", |im| {
        let mut out = Digest::default();
        for &mode in &[2i32, 7, i32::MAX] {
            out.tag("mode");
            out.i64(mode as i64);
            out.bytes(&string_map_row(im, None, mode, 0xE6, &[1, 4, 9, 17], 30));
            out.bytes(&string_map_row(im, Some(SH_STRDUP), mode, 0xE7, &[1, 4, 9, 17], 30));
        }
        out.0
    });
}

#[test]
fn row30_string_duplicate_keys_temp_key() {
    run_both("row30", |im| unsafe {
        let mut d = Digest::default();
        for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut rng = Rng::new(0xE8 + sm as u64);
            let mut m = Map::from_shmode(im, 16, 8, KeyRepr::Pointer, sm);
            let keys: Vec<Vec<u8>> =
                (0..24).map(|i| cstring(format!("key_{i}").as_bytes())).collect();
            for (i, k) in keys.iter().enumerate() {
                m.put(k, &(i as u64).to_ne_bytes(), 1);
            }
            // hammer duplicates so both the forward-half and the wrap-around
            // half of the bucket scan report "found" (the C code only writes
            // temp_key in the forward half)
            for _ in 0..200 {
                let j = rng.below(keys.len());
                let kk = keys[j].clone();
                let t = m.put(&kk, &(j as u64 ^ 0xAA).to_ne_bytes(), 1);
                d.isize(t);
                m.snap(&mut d, "dup");
            }
            m.free();
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 31-32 — stbds_hmget_key / stbds_hmget_key_ts shapes
// ---------------------------------------------------------------------------

#[test]
fn row31_row32_hmget_shapes() {
    run_both("row31_32", |im| unsafe {
        let mut d = Digest::default();
        for &mode in &[0i32, 1] {
            let (elemsize, keysize, repr) = if mode == 0 {
                (8usize, 4usize, KeyRepr::Inline(4))
            } else {
                (16usize, 8usize, KeyRepr::Pointer)
            };
            (im.rand_seed)(DEFAULT_SEED);
            let k0 = if mode == 0 { i32key(5) } else { cstring(b"five") };

            // a == NULL
            let mut m = Map::new(im, elemsize, keysize, repr);
            let t = m.get_ts(&k0, mode);
            d.tag("null");
            d.isize(t);
            m.snap(&mut d, "st");
            m.free();

            // hash_table == NULL (array built by arrgrowf)
            let raw = (im.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 1);
            std::ptr::write_bytes(raw as *mut u8, 0, elemsize);
            (*header(raw)).length = 1;
            let mut m2 = Map::new(im, elemsize, keysize, repr);
            m2.t = (raw as *mut u8).add(elemsize) as *mut c_void;
            let t = m2.get_ts(&k0, mode);
            d.tag("no-table");
            d.isize(t);
            m2.snap(&mut d, "st");
            let (t, v) = m2.get(&k0, mode);
            d.tag("no-table-get");
            d.isize(t);
            d.bytes(&v);
            m2.snap(&mut d, "st");
            (im.hmfree_func)(raw, elemsize);
            m2.t = std::ptr::null_mut();

            // present / absent keys on a populated map
            let mut m3 = Map::new(im, elemsize, keysize, repr);
            let keys: Vec<Vec<u8>> = (0..20)
                .map(|i| {
                    if mode == 0 {
                        i32key(i * 5)
                    } else {
                        cstring(format!("k{i}").as_bytes())
                    }
                })
                .collect();
            for (i, k) in keys.iter().enumerate() {
                m3.put(k, &(i as u64).to_ne_bytes(), mode);
            }
            for k in &keys {
                d.tag("hit");
                d.isize(m3.get_ts(k, mode));
                let (t, v) = m3.get(k, mode);
                d.isize(t);
                d.bytes(&v);
            }
            for i in 100..120 {
                let k = if mode == 0 {
                    i32key(i)
                } else {
                    cstring(format!("z{i}").as_bytes())
                };
                d.tag("absent");
                d.isize(m3.get_ts(&k, mode));
                let (t, v) = m3.get(&k, mode);
                d.isize(t);
                d.bytes(&v);
            }
            m3.snap(&mut d, "final");
            m3.free();
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 33 — stbds_shmode_func
// ---------------------------------------------------------------------------

#[test]
fn row33_shmode_func() {
    run_both("row33", |im| unsafe {
        let mut d = Digest::default();
        for &mode in &[0i32, 1, 2, 3] {
            for &elemsize in &[8usize, 16, 24, 64] {
                (im.rand_seed)(DEFAULT_SEED);
                let t = (im.shmode_func)(elemsize, mode);
                d.tag("shmode");
                d.i64(mode as i64);
                d.usize(elemsize);
                d.bytes(&digest_map(t, elemsize, KeyRepr::Inline(8), false));
                (im.hmfree_func)(hash_to_arr(t, elemsize), elemsize);
            }
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 34-40 — stbds_hmdel_key
// ---------------------------------------------------------------------------

#[test]
fn row34_del_last_element() {
    run_both("row34", |im| unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
        let keys: Vec<i32> = (0..12).map(|i| i * 11 + 3).collect();
        for (i, &k) in keys.iter().enumerate() {
            m.put(&i32key(k), &i32val(i as i32), 0);
        }
        // delete in reverse insertion order => always old_index == final_index
        for &k in keys.iter().rev() {
            let r = m.del(&i32key(k), 0);
            d.tag("del");
            d.isize(r);
            m.snap(&mut d, "st");
        }
        m.free();
        d.0
    });
}

#[test]
fn row35_del_non_last_element() {
    run_both("row35", |im| unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
        let keys: Vec<i32> = (0..12).map(|i| i * 11 + 3).collect();
        for (i, &k) in keys.iter().enumerate() {
            m.put(&i32key(k), &i32val(i as i32), 0);
        }
        // delete in insertion order => the last element is swapped in each time
        for &k in keys.iter() {
            let r = m.del(&i32key(k), 0);
            d.tag("del");
            d.isize(r);
            m.snap(&mut d, "st");
        }
        m.free();
        d.0
    });
}

#[test]
fn row36_del_absent_and_degenerate() {
    run_both("row36", |im| unsafe {
        let mut d = Digest::default();
        (im.rand_seed)(DEFAULT_SEED);

        // NULL map
        let mut m0 = Map::new(im, 8, 4, KeyRepr::Inline(4));
        let r = m0.del(&i32key(1), 0);
        d.tag("null");
        d.isize(r);
        d.u8(m0.t.is_null() as u8);

        // no hash table
        let raw = (im.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        std::ptr::write_bytes(raw as *mut u8, 0, 8);
        (*header(raw)).length = 1;
        (*header(raw)).temp = 0x7777;
        let mut m1 = Map::new(im, 8, 4, KeyRepr::Inline(4));
        m1.t = (raw as *mut u8).add(8) as *mut c_void;
        let r = m1.del(&i32key(1), 0);
        d.tag("no-table");
        d.isize(r);
        m1.snap(&mut d, "st");
        (im.hmfree_func)(raw, 8);
        m1.t = std::ptr::null_mut();

        // absent keys on a populated map
        let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
        for i in 0..20 {
            m.put(&i32key(i * 7), &i32val(i), 0);
        }
        for i in 0..20 {
            let r = m.del(&i32key(i * 7 + 1), 0);
            d.tag("absent");
            d.isize(r);
            m.snap(&mut d, "st");
        }
        m.free();
        d.0
    });
}

#[test]
fn row37_row38_del_rebuild_and_shrink() {
    run_both("row37_38", |im| unsafe {
        let mut d = Digest::default();
        for n in [10usize, 20, 40, 100] {
            (im.rand_seed)(DEFAULT_SEED);
            let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
            let keys: Vec<i32> = (0..n as i32).map(|i| i * 13 + 5).collect();
            for (i, &k) in keys.iter().enumerate() {
                m.put(&i32key(k), &i32val(i as i32), 0);
            }
            m.snap(&mut d, "filled");
            // deleting every other key accumulates tombstones while used_count
            // stays above the shrink threshold => in-place rebuild; the second
            // sweep then drives the shrink.
            for &k in keys.iter().step_by(2) {
                let r = m.del(&i32key(k), 0);
                d.isize(r);
                m.snap(&mut d, "del-a");
            }
            for &k in keys.iter().skip(1).step_by(2) {
                let r = m.del(&i32key(k), 0);
                d.isize(r);
                m.snap(&mut d, "del-b");
            }
            m.free();
        }
        d.0
    });
}

#[test]
fn row39_row40_del_string_modes() {
    run_both("row39_40", |im| unsafe {
        let mut d = Digest::default();
        for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &mode in &[1i32, 2] {
                for forward in [true, false] {
                    // `mode == 2` (out-of-enum) is only *safe* when the deleted
                    // element is the last one: for `mode != STBDS_HM_STRING` the
                    // C code re-finds the swapped-in element by hashing the
                    // element bytes *as a string*, which walks off the element
                    // and is undefined behaviour in the original. So mode 2 is
                    // exercised in reverse (last-element) order only.
                    if mode == 2 && forward {
                        continue;
                    }
                    (im.rand_seed)(DEFAULT_SEED);
                    let mut m = Map::from_shmode(im, 16, 8, KeyRepr::Pointer, sm);
                    let keys: Vec<Vec<u8>> = (0..24)
                        .map(|i| cstring(format!("string_key_{i}").as_bytes()))
                        .collect();
                    for (i, k) in keys.iter().enumerate() {
                        m.put(k, &(i as u64).to_ne_bytes(), mode);
                    }
                    m.snap(&mut d, "filled");
                    let order: Vec<&Vec<u8>> = if forward {
                        keys.iter().collect()
                    } else {
                        keys.iter().rev().collect()
                    };
                    for k in order {
                        let r = m.del(k, mode);
                        d.tag("del");
                        d.i64(sm as i64);
                        d.i64(mode as i64);
                        d.u8(forward as u8);
                        d.isize(r);
                        m.snap(&mut d, "st");
                    }
                    m.free();
                }
            }
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 41-43, 55 — randomised mixed put/get/del fuzz
// ---------------------------------------------------------------------------

fn fuzz_binary(
    im: &Impl,
    elemsize: usize,
    keysize: usize,
    keyspace: usize,
    ops: usize,
    seed: u64,
    hash_seed: usize,
) -> Vec<u8> {
    unsafe {
        (im.rand_seed)(hash_seed);
        let mut d = Digest::default();
        let mut rng = Rng::new(seed);
        let mut m = Map::new(im, elemsize, keysize, KeyRepr::Inline(keysize));
        let keypool: Vec<Vec<u8>> = {
            let mut r2 = Rng::new(seed ^ 0x5a5a);
            (0..keyspace).map(|_| r2.bytes(keysize)).collect()
        };
        let mut val = [0u8; 64];
        for step in 0..ops {
            let k = keypool[rng.below(keyspace)].clone();
            match rng.below(10) {
                0..=4 => {
                    for (i, b) in val.iter_mut().enumerate() {
                        *b = (step as u8).wrapping_mul(31).wrapping_add(i as u8);
                    }
                    let t = m.put(&k, &val, 0);
                    d.tag("p");
                    d.isize(t);
                }
                5..=7 => {
                    let (t, v) = m.get(&k, 0);
                    d.tag("g");
                    d.isize(t);
                    d.bytes(&v);
                    d.isize(m.get_ts(&k, 0));
                }
                _ => {
                    let r = m.del(&k, 0);
                    d.tag("d");
                    d.isize(r);
                }
            }
            if step % 16 == 0 {
                m.snap(&mut d, "st");
            }
        }
        m.snap(&mut d, "final");
        m.free();
        d.0
    }
}

#[test]
fn row41_fuzz_keysize4() {
    run_both("row41", |im| fuzz_binary(im, 8, 4, 64, 2000, 0xF1, DEFAULT_SEED));
}

#[test]
fn row42_fuzz_keysize8() {
    run_both("row42", |im| fuzz_binary(im, 16, 8, 64, 2000, 0xF2, DEFAULT_SEED));
}

#[test]
fn row55_fuzz_random_hash_seed() {
    run_both("row55", |im| {
        let mut out = Digest::default();
        let mut rng = Rng::new(0xF9);
        for _ in 0..6 {
            let hs = rng.next_u64() as usize;
            out.usize(hs);
            out.bytes(&fuzz_binary(im, 8, 4, 48, 600, rng.next_u64(), hs));
            out.bytes(&fuzz_binary(im, 24, 16, 48, 600, rng.next_u64(), hs));
        }
        out.0
    });
}

fn fuzz_string(im: &Impl, sh_mode: c_int, mode: c_int, ops: usize, seed: u64, allow_del: bool) -> Vec<u8> {
    unsafe {
        (im.rand_seed)(DEFAULT_SEED);
        let mut d = Digest::default();
        let mut rng = Rng::new(seed);
        let mut m = Map::from_shmode(im, 16, 8, KeyRepr::Pointer, sh_mode);
        let raw_pool: Vec<Vec<u8>> = {
            let mut r2 = Rng::new(seed ^ 0x1234);
            (0..48)
                .map(|_| {
                    let n = r2.range(0, 20);
                    rand_cstring(&mut r2, n, ASCII)
                })
                .collect()
        };
        // dedup so equal-content keys always come from *distinct buffers*
        let mut uniq = std::collections::HashSet::new();
        let keypool: Vec<Vec<u8>> = raw_pool.into_iter().filter(|k| uniq.insert(k.clone())).collect();
        for step in 0..ops {
            let k = keypool[rng.below(keypool.len())].clone();
            match rng.below(10) {
                0..=4 => {
                    let t = m.put(&k, &(step as u64).to_ne_bytes(), mode);
                    d.tag("p");
                    d.isize(t);
                }
                5..=7 => {
                    let (t, v) = m.get(&k, mode);
                    d.tag("g");
                    d.isize(t);
                    d.bytes(&v);
                }
                _ => {
                    if allow_del {
                        let r = m.del(&k, mode);
                        d.tag("d");
                        d.isize(r);
                    } else {
                        // `mode != STBDS_HM_STRING` deletes of a non-last
                        // element are undefined behaviour in the C original
                        // (see row39_40); keep this fuzz to put/get.
                        let (t, v) = m.get(&k, mode);
                        d.tag("g2");
                        d.isize(t);
                        d.bytes(&v);
                    }
                }
            }
            if step % 16 == 0 {
                m.snap(&mut d, "st");
            }
        }
        m.snap(&mut d, "final");
        m.free();
        d.0
    }
}

#[test]
fn row43_fuzz_string_modes() {
    run_both("row43", |im| {
        let mut out = Digest::default();
        for &sm in &[SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            for &mode in &[1i32, 2] {
                out.tag("cfg");
                out.i64(sm as i64);
                out.i64(mode as i64);
                out.bytes(&fuzz_string(
                    im,
                    sm,
                    mode,
                    1200,
                    0xFA + sm as u64 * 7 + mode as u64,
                    mode == 1,
                ));
            }
        }
        out.0
    });
}

// ---------------------------------------------------------------------------
// Row 44 — stbds_hmfree_func
// ---------------------------------------------------------------------------

#[test]
fn row44_hmfree_variants() {
    run_both("row44", |im| unsafe {
        let mut d = Digest::default();
        // NULL
        (im.hmfree_func)(std::ptr::null_mut(), 8);
        d.tag("null-ok");

        // hash_table == NULL
        (im.rand_seed)(DEFAULT_SEED);
        let raw = (im.arrgrowf)(std::ptr::null_mut(), 8, 0, 1);
        std::ptr::write_bytes(raw as *mut u8, 0, 8);
        (*header(raw)).length = 1;
        (im.hmfree_func)(raw, 8);
        d.tag("no-table-ok");

        // binary, default, strdup and arena maps
        for &sm in &[-1i32, SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
            (im.rand_seed)(DEFAULT_SEED);
            let mode = if sm <= 0 { 0 } else { 1 };
            let mut m = if sm < 0 {
                Map::new(im, 16, 8, KeyRepr::Inline(8))
            } else if sm == SH_NONE {
                Map::from_shmode(im, 16, 8, KeyRepr::Inline(8), sm)
            } else {
                Map::from_shmode(im, 16, 8, KeyRepr::Pointer, sm)
            };
            for i in 0..30u64 {
                let k = if mode == 0 {
                    let mut v = i.to_ne_bytes().to_vec();
                    v.resize(8, 0);
                    v
                } else {
                    cstring(format!("free_key_{i}").as_bytes())
                };
                m.put(&k, &i.to_ne_bytes(), mode);
            }
            m.snap(&mut d, "pre-free");
            m.free();
            d.tag("freed");
            d.i64(sm as i64);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 45-51 — stbds_stralloc / stbds_strreset
// ---------------------------------------------------------------------------

fn arena_scenario(im: &Impl, initial_block: u8, lens: &[usize]) -> Vec<u8> {
    unsafe {
        let mut d = Digest::default();
        let mut a = StringArena {
            storage: std::ptr::null_mut(),
            remaining: 0,
            block: initial_block,
            mode: 0,
        };
        let ap = &raw mut a as *mut c_void;
        let mut kept: Vec<(*mut c_char, Vec<u8>)> = Vec::new();
        let mut rng = Rng::new(0x1357 + initial_block as u64);
        for &len in lens {
            let mut s = rand_cstring(&mut rng, len, ASCII);
            let p = (im.stralloc)(ap, s.as_mut_ptr() as *mut c_char);
            d.tag("alloc");
            d.usize(len);
            d.bytes(&cstr_bytes(p));
            digest_arena(&mut d, &raw const a);
            kept.push((p, s[..len].to_vec()));
            // every previously returned string must still be intact
            for (q, expect) in &kept {
                let got = cstr_bytes(*q);
                assert_eq!(&got, expect, "{}: arena string clobbered", im.name);
                d.bytes(&got);
            }
        }
        (im.strreset)(ap);
        d.tag("reset");
        digest_arena(&mut d, &raw const a);
        d.0
    }
}

#[test]
fn row45_arena_first_block() {
    run_both("row45", |im| arena_scenario(im, 0, &[10]));
}

#[test]
fn row46_arena_many_short_strings() {
    run_both("row46", |im| {
        let mut rng = Rng::new(0x2468);
        // enough allocations to exhaust several blocks and advance `block`
        let lens: Vec<usize> = (0..600).map(|_| rng.range(1, 40)).collect();
        arena_scenario(im, 0, &lens)
    });
}

#[test]
fn row47_arena_exact_boundary() {
    run_both("row47", |im| unsafe {
        let mut d = Digest::default();
        let mut a = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let ap = &raw mut a as *mut c_void;
        let mut keep: Vec<Vec<u8>> = Vec::new();
        // first allocation creates a 512-byte block
        let mut s = cstring(&vec![b'a'; 99]);
        let p = (im.stralloc)(ap, s.as_mut_ptr() as *mut c_char);
        d.bytes(&cstr_bytes(p));
        digest_arena(&mut d, &raw const a);
        keep.push(s);
        // exactly consume the rest: len == remaining
        let rem = a.remaining;
        let mut s2 = cstring(&vec![b'b'; rem - 1]);
        let p2 = (im.stralloc)(ap, s2.as_mut_ptr() as *mut c_char);
        d.bytes(&cstr_bytes(p2));
        digest_arena(&mut d, &raw const a);
        assert_eq!(a.remaining, 0, "{}: expected the block to be exhausted", im.name);
        keep.push(s2);
        // one more byte -> brand new block
        let mut s3 = cstring(b"x");
        let p3 = (im.stralloc)(ap, s3.as_mut_ptr() as *mut c_char);
        d.bytes(&cstr_bytes(p3));
        digest_arena(&mut d, &raw const a);
        keep.push(s3);
        d.bytes(&cstr_bytes(p));
        d.bytes(&cstr_bytes(p2));
        (im.strreset)(ap);
        digest_arena(&mut d, &raw const a);
        let _ = keep;
        d.0
    });
}

#[test]
fn row48_arena_oversized_strings() {
    run_both("row48", |im| {
        let mut out = Digest::default();
        // storage == NULL on the first (over-sized) allocation
        out.bytes(&arena_scenario(im, 0, &[600]));
        out.bytes(&arena_scenario(im, 0, &[5000, 20]));
        // storage != NULL when the over-sized allocation happens
        out.bytes(&arena_scenario(im, 0, &[20, 600, 30, 5000, 40]));
        out.0
    });
}

#[test]
fn row49_arena_block_counter_extremes() {
    run_both("row49", |im| {
        let mut out = Digest::default();
        // `blocksize = 512 << (block >> 1)`; block >= 110 shifts every bit out
        // (blocksize == 0), block >= 128 wraps the x86-64 shift count mod 64.
        for &b in &[
            0u8, 1, 2, 3, 4, 5, 6, 20, 21, 22, 23, 24, 110, 111, 126, 127, 128, 129, 254, 255,
        ] {
            out.tag("block");
            out.u8(b);
            out.bytes(&arena_scenario(im, b, &[8, 40, 8]));
        }
        out.0
    });
}

#[test]
fn row50_arena_empty_and_mixed() {
    run_both("row50", |im| arena_scenario(im, 0, &[0, 0, 0, 1, 0, 1, 700, 0, 1, 2, 0]));
}

#[test]
fn row51_arena_reset_and_reuse() {
    run_both("row51", |im| unsafe {
        let mut d = Digest::default();
        let mut a = StringArena { storage: std::ptr::null_mut(), remaining: 0, block: 0, mode: 0 };
        let ap = &raw mut a as *mut c_void;
        let mut rng = Rng::new(0x99);
        for cycle in 0..4usize {
            let mut keep = Vec::new();
            for _ in 0..200 {
                let n = rng.range(1, 30);
                let mut s = rand_cstring(&mut rng, n, ASCII);
                let p = (im.stralloc)(ap, s.as_mut_ptr() as *mut c_char);
                d.bytes(&cstr_bytes(p));
                digest_arena(&mut d, &raw const a);
                keep.push(s);
            }
            (im.strreset)(ap);
            d.tag("cycle");
            d.usize(cycle);
            digest_arena(&mut d, &raw const a);
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 52 — strkey
// ---------------------------------------------------------------------------

#[test]
fn row52_strkey() {
    run_both("row52", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x5150);
        let mut ns: Vec<i32> =
            vec![0, 1, -1, 9, 11, 12345, -12345, i32::MAX, i32::MIN, 10, -10, 100, 1000];
        ns.extend((0..200).map(|_| rng.i32()));
        for n in ns {
            let p = (im.strkey)(n);
            d.tag("strkey");
            d.i64(n as i64);
            d.bytes(&cstr_bytes(p));
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Rows 53-54 — intput
// ---------------------------------------------------------------------------

#[test]
fn row53_intput_non_aborting() {
    run_both("row53", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x7777);
        let mut ns: Vec<i32> = vec![0, 1, 7, 8, 10, 12, -1, -9, -11, 3, i32::MAX, i32::MIN];
        ns.extend((0..60).map(|_| rng.i32()));
        for n in ns {
            if n == 9 || n == 11 {
                continue;
            }
            (im.rand_seed)(DEFAULT_SEED);
            (im.intput)(n);
            d.tag("intput-ok");
            d.i64(n as i64);
        }
        d.0
    });
}

/// Row 54 — the exact macro expansion of `intput`, driven through the low-level
/// exports, with a full map digest after every single step.
#[test]
fn row54_intput_expansion_replay() {
    run_both("row54", |im| unsafe {
        let mut d = Digest::default();
        let mut rng = Rng::new(0x8888);
        let mut ns: Vec<i32> = vec![0, 1, 7, 8, 9, 10, 11, 12, -1, i32::MAX, i32::MIN];
        ns.extend((0..80).map(|_| rng.i32()));
        for num in ns {
            (im.rand_seed)(DEFAULT_SEED);
            let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
            for (k, v) in [(num, 7), (11, 3), (9, num)] {
                let t = m.put(&i32key(k), &i32val(v), 0);
                d.tag("put");
                d.i64(num as i64);
                d.isize(t);
                m.snap(&mut d, "st");
            }
            for k in [9, 11, num] {
                let (t, v) = m.get(&i32key(k), 0);
                d.tag("get");
                d.isize(t);
                d.bytes(&v);
                m.snap(&mut d, "st");
            }
            // the C original leaks `intmap`; free it here so the test itself
            // does not (the map contents have already been digested)
            m.free();
        }
        d.0
    });
}

// ---------------------------------------------------------------------------
// Row 56 — interleaved fresh-table creation keeps the seed LCG in lockstep
// ---------------------------------------------------------------------------

#[test]
fn row56_interleaved_table_creation() {
    run_both("row56", |im| unsafe {
        let mut d = Digest::default();
        (im.rand_seed)(DEFAULT_SEED);
        let mut rng = Rng::new(0x9999);
        let mut live: Vec<(*mut c_void, usize)> = Vec::new();
        let mut maps: Vec<Map> = Vec::new();
        for step in 0..90usize {
            match rng.below(3) {
                0 => {
                    let t = (im.shmode_func)(16, (step % 4) as c_int);
                    let raw = hash_to_arr(t, 16);
                    let ht = (*header(raw)).hash_table as *const HashIndex;
                    d.tag("shmode");
                    d.usize((*ht).seed);
                    live.push((t, 16));
                }
                1 => {
                    let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
                    for i in 0..(step % 9 + 1) {
                        m.put(&i32key((step * 100 + i) as i32), &i32val(i as i32), 0);
                    }
                    let raw = hash_to_arr(m.t, 8);
                    let ht = (*header(raw)).hash_table as *const HashIndex;
                    d.tag("hmput");
                    d.usize((*ht).seed);
                    m.snap(&mut d, "st");
                    maps.push(m);
                }
                _ => {
                    // shrink/rebuild builds fresh tables from `ot`, which must
                    // NOT advance the global seed
                    let mut m = Map::new(im, 8, 4, KeyRepr::Inline(4));
                    for i in 0..30i32 {
                        m.put(&i32key(i * 3), &i32val(i), 0);
                    }
                    for i in 0..30i32 {
                        m.del(&i32key(i * 3), 0);
                        m.snap(&mut d, "shrink");
                    }
                    maps.push(m);
                }
            }
        }
        for (t, es) in live {
            (im.hmfree_func)(hash_to_arr(t, es), es);
        }
        for m in maps.iter_mut() {
            m.free();
        }
        d.0
    });
}
