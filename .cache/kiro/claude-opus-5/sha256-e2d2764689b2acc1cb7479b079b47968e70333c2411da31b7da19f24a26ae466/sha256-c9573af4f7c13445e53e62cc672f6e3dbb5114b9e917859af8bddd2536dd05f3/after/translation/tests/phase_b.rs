//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//! Every call goes through a `.so` export loaded with `libloading`.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

const E_INT_INT: usize = 8; // struct { int key; int value; }
const E_SZ_SZ: usize = 16; // struct { size_t key; size_t value; }
const E_STR: usize = 16; // struct { char *key; int value; }

// ---------------------------------------------------------------------------
// Caller-side re-implementation of the array macros (raw bytes + elemsize)
// ---------------------------------------------------------------------------

struct Arr<'a> {
    lib: &'a Lib,
    a: *mut u8,
    elemsize: usize,
}

impl<'a> Arr<'a> {
    fn new(lib: &'a Lib, elemsize: usize) -> Arr<'a> {
        Arr { lib, a: std::ptr::null_mut(), elemsize }
    }
    unsafe fn len(&self) -> usize {
        if self.a.is_null() { 0 } else { unsafe { (*header(self.a)).length } }
    }
    unsafe fn cap(&self) -> usize {
        if self.a.is_null() { 0 } else { unsafe { (*header(self.a)).capacity } }
    }
    unsafe fn grow(&mut self, addlen: usize, min_cap: usize) {
        unsafe {
            self.a = (self.lib.arrgrowf)(self.a as *mut c_void, self.elemsize, addlen, min_cap)
                as *mut u8;
        }
    }
    /// `stbds_arrmaybegrow(a,n)`
    unsafe fn maybegrow(&mut self, n: usize) {
        unsafe {
            if self.a.is_null() || (*header(self.a)).length + n > (*header(self.a)).capacity {
                self.grow(n, 0);
            }
        }
    }
    unsafe fn put(&mut self, v: &[u8]) {
        unsafe {
            self.maybegrow(1);
            let h = header(self.a);
            let i = (*h).length;
            (*h).length = i + 1;
            std::ptr::copy_nonoverlapping(v.as_ptr(), self.a.add(i * self.elemsize), self.elemsize);
        }
    }
    unsafe fn pop(&mut self) -> Vec<u8> {
        unsafe {
            let h = header(self.a);
            (*h).length -= 1;
            let i = (*h).length;
            std::slice::from_raw_parts(self.a.add(i * self.elemsize), self.elemsize).to_vec()
        }
    }
    unsafe fn setcap(&mut self, n: usize) {
        unsafe { self.grow(0, n) }
    }
    unsafe fn setlen(&mut self, n: usize) {
        unsafe {
            if self.cap() < n {
                self.setcap(n);
            }
            if !self.a.is_null() {
                (*header(self.a)).length = n;
            }
        }
    }
    /// `stbds_arraddnindex(a,n)`
    unsafe fn addnindex(&mut self, n: usize) -> isize {
        unsafe {
            self.maybegrow(n);
            if n != 0 {
                let h = header(self.a);
                (*h).length += n;
                ((*h).length - n) as isize
            } else {
                self.len() as isize
            }
        }
    }
    unsafe fn deln(&mut self, i: usize, n: usize) {
        unsafe {
            let h = header(self.a);
            let cnt = (*h).length - n - i;
            std::ptr::copy(
                self.a.add((i + n) * self.elemsize),
                self.a.add(i * self.elemsize),
                self.elemsize * cnt,
            );
            (*h).length -= n;
        }
    }
    unsafe fn delswap(&mut self, i: usize) {
        unsafe {
            let h = header(self.a);
            let last = (*h).length - 1;
            std::ptr::copy(
                self.a.add(last * self.elemsize),
                self.a.add(i * self.elemsize),
                self.elemsize,
            );
            (*h).length -= 1;
        }
    }
    unsafe fn insn(&mut self, i: usize, n: usize) {
        unsafe {
            self.addnindex(n);
            let h = header(self.a);
            let cnt = (*h).length - n - i;
            std::ptr::copy(
                self.a.add(i * self.elemsize),
                self.a.add((i + n) * self.elemsize),
                self.elemsize * cnt,
            );
        }
    }
    unsafe fn ins(&mut self, i: usize, v: &[u8]) {
        unsafe {
            self.insn(i, 1);
            std::ptr::copy_nonoverlapping(v.as_ptr(), self.a.add(i * self.elemsize), self.elemsize);
        }
    }
    unsafe fn snap(&self) -> ArrSnap {
        unsafe { snap_arr(self.a, self.elemsize) }
    }
    unsafe fn free(&mut self) {
        unsafe {
            if !self.a.is_null() {
                (self.lib.arrfreef)(self.a as *mut c_void);
                self.a = std::ptr::null_mut();
            }
        }
    }
}

/// Persistent, identical key strings handed to BOTH libraries (so
/// `STBDS_SH_DEFAULT` stores byte-identical pointers on both sides).
fn key_pool(n: usize) -> Vec<*mut c_char> {
    use std::sync::OnceLock;
    static POOL: OnceLock<Vec<Box<[u8]>>> = OnceLock::new();
    let pool = POOL.get_or_init(|| {
        (0..4096usize)
            .map(|i| {
                // At least 20 bytes: the `STBDS_SH_NONE` default arm of
                // `stbds_hmput_key` memcpy's `keysize` (8) bytes out of the key
                // buffer, so short keys would read past the allocation.
                let mut s = format!("key_{:06}_{}", i, "x".repeat(8 + i % 37)).into_bytes();
                s.push(0);
                s.into_boxed_slice()
            })
            .collect()
    });
    pool[..n]
        .iter()
        .map(|b| b.as_ptr() as *mut c_char)
        .collect()
}

// ===========================================================================
// Rows 1-6: stbds_hash_bytes
// ===========================================================================

fn hash_bytes_row(seeds: &[usize], lens: &[usize], high_bit: bool, rng_seed: u64, label: &str) {
    let p = libs();
    let mut rng = Rng::new(rng_seed);
    for &len in lens {
        for &seed in seeds {
            for _ in 0..24 {
                let mut buf: Vec<u8> = rng.bytes(len);
                if high_bit {
                    for b in buf.iter_mut() {
                        *b |= 0x80;
                    }
                }
                let (a, b) = unsafe {
                    let pa = if len == 0 {
                        std::ptr::null_mut()
                    } else {
                        buf.as_mut_ptr() as *mut c_void
                    };
                    (
                        (p.c.hash_bytes)(pa, len, seed),
                        (p.r.hash_bytes)(pa, len, seed),
                    )
                };
                assert_eq!(a, b, "{label}: len={len} seed={seed:#x} buf={buf:02x?}");
            }
        }
    }
}

const SEEDS: [usize; 6] = [0, 1, 0x3141_5926, usize::MAX, usize::MAX / 3, 0xdead_beef_cafe_babe];

#[test]
fn row01_hash_bytes_len0() {
    hash_bytes_row(&SEEDS, &[0], false, 1, "row01");
}

#[test]
fn row02_hash_bytes_tail_1_to_7() {
    hash_bytes_row(&SEEDS, &[1, 2, 3, 4, 5, 6, 7], false, 2, "row02");
}

#[test]
fn row03_hash_bytes_len8() {
    hash_bytes_row(&SEEDS, &[8], false, 3, "row03");
}

#[test]
fn row04_hash_bytes_9_to_64() {
    let lens: Vec<usize> = (9..=64).collect();
    hash_bytes_row(&SEEDS, &lens, false, 4, "row04");
}

#[test]
fn row05_hash_bytes_high_bit() {
    let lens: Vec<usize> = (1..=64).collect();
    hash_bytes_row(&SEEDS, &lens, true, 5, "row05");
}

#[test]
fn row06_hash_bytes_large() {
    hash_bytes_row(&SEEDS, &[256, 257, 1023, 1024, 4096], false, 6, "row06");
}

// ===========================================================================
// Row 7: stbds_hash_string
// ===========================================================================

#[test]
fn row07_hash_string() {
    let p = libs();
    let mut rng = Rng::new(7);
    for len in 0..=64usize {
        for &seed in &SEEDS {
            for variant in 0..3 {
                let mut s: Vec<u8> = (0..len)
                    .map(|_| match variant {
                        0 => (rng.byte() % 94) + 33,      // printable ASCII
                        1 => rng.byte() | 0x80,           // high bit set
                        _ => (rng.byte() % 255).max(1),   // any non-NUL
                    })
                    .collect();
                s.push(0);
                let (a, b) = unsafe {
                    (
                        (p.c.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                        (p.r.hash_string)(s.as_mut_ptr() as *mut c_char, seed),
                    )
                };
                assert_eq!(a, b, "row07: len={len} seed={seed:#x} s={s:02x?}");
            }
        }
    }
}

// ===========================================================================
// Row 8 / 53: global seed LCG lock-step
// ===========================================================================

#[test]
fn row08_rand_seed_lcg_lockstep() {
    let p = libs();
    for &seed in &SEEDS {
        reset_seeds(p, seed);
        // Ten fresh tables in a row: each advances the global LCG.
        let mut cs = Vec::new();
        let mut rs = Vec::new();
        for _ in 0..10 {
            unsafe {
                let tc = (p.c.shmode_func)(E_STR, STBDS_SH_ARENA) as *mut u8;
                let tr = (p.r.shmode_func)(E_STR, STBDS_SH_ARENA) as *mut u8;
                cs.push(snap_map(tc, E_STR, KeyRepr::Raw));
                rs.push(snap_map(tr, E_STR, KeyRepr::Raw));
                (p.c.hmfree_func)(tc.sub(E_STR) as *mut c_void, E_STR);
                (p.r.hmfree_func)(tr.sub(E_STR) as *mut c_void, E_STR);
            }
        }
        assert_eq!(cs, rs, "row08: seed={seed:#x}");
        // the seeds must actually differ between successive tables
        let seeds: Vec<usize> = cs.iter().map(|s| s.table.as_ref().unwrap().seed).collect();
        assert!(seeds.windows(2).all(|w| w[0] != w[1]), "LCG did not advance");
    }
}

// ===========================================================================
// Rows 9-13: stbds_arrgrowf / stbds_arrfreef
// ===========================================================================

#[test]
fn row09_arrgrowf_null_noop() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 24, 32] {
        let (a, b) = unsafe {
            (
                (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0),
                (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 0),
            )
        };
        assert!(a.is_null() && b.is_null(), "row09: elemsize={elemsize} {a:?} {b:?}");
    }
}

#[test]
fn row10_arrgrowf_fresh_random() {
    let p = libs();
    let mut rng = Rng::new(10);
    for _ in 0..400 {
        let elemsize = [1usize, 2, 4, 8, 16, 24, 32][rng.below(7)];
        let addlen = rng.below(64);
        let min_cap = rng.below(64);
        let mut c_a = Arr::new(&p.c, elemsize);
        let mut r_a = Arr::new(&p.r, elemsize);
        unsafe {
            c_a.grow(addlen, min_cap);
            r_a.grow(addlen, min_cap);
            assert_eq!(
                c_a.snap(),
                r_a.snap(),
                "row10: elemsize={elemsize} addlen={addlen} min_cap={min_cap}"
            );
            c_a.free();
            r_a.free();
        }
    }
}

#[test]
fn row11_arrgrowf_repeated_growth() {
    let p = libs();
    let mut rng = Rng::new(11);
    for _ in 0..80 {
        let elemsize = [4usize, 8, 16][rng.below(3)];
        let mut c_a = Arr::new(&p.c, elemsize);
        let mut r_a = Arr::new(&p.r, elemsize);
        unsafe {
            for _ in 0..30 {
                // alternate the doubling path and the explicit-min_cap path
                let addlen = rng.below(8);
                let min_cap = if rng.bool() {
                    0
                } else {
                    // stay bounded: exercise both the "min_cap >= 2*cap"
                    // explicit path and the doubling path without asking for
                    // gigabytes.
                    (c_a.cap() * 2 + rng.below(9)).min(4096)
                };
                c_a.grow(addlen, min_cap);
                r_a.grow(addlen, min_cap);
                // pretend the added elements exist so arrlen advances
                let n = rng.below(3);
                if !c_a.a.is_null() && c_a.len() + n <= c_a.cap() {
                    (*header(c_a.a)).length += n;
                    (*header(r_a.a)).length += n;
                    for i in (c_a.len() - n)..c_a.len() {
                        let v = rng.bytes(elemsize);
                        std::ptr::copy_nonoverlapping(
                            v.as_ptr(),
                            c_a.a.add(i * elemsize),
                            elemsize,
                        );
                        std::ptr::copy_nonoverlapping(
                            v.as_ptr(),
                            r_a.a.add(i * elemsize),
                            elemsize,
                        );
                    }
                }
                assert_eq!(c_a.snap(), r_a.snap(), "row11: elemsize={elemsize}");
            }
            c_a.free();
            r_a.free();
        }
    }
}

#[test]
fn row12_arrgrowf_early_return() {
    let p = libs();
    for elemsize in [4usize, 8, 16] {
        let mut c_a = Arr::new(&p.c, elemsize);
        let mut r_a = Arr::new(&p.r, elemsize);
        unsafe {
            c_a.grow(0, 10);
            r_a.grow(0, 10);
            let before_c = c_a.snap();
            // min_cap <= cap -> untouched, and the same pointer comes back
            let pc = c_a.a;
            let pr = r_a.a;
            for mc in [0usize, 1, 5, 10] {
                c_a.grow(0, mc);
                r_a.grow(0, mc);
                assert_eq!(c_a.a, pc);
                assert_eq!(r_a.a, pr);
                assert_eq!(c_a.snap(), r_a.snap(), "row12: elemsize={elemsize} mc={mc}");
                assert_eq!(c_a.snap(), before_c);
            }
            c_a.free();
            r_a.free();
        }
    }
}

#[test]
fn row13_arrgrowf_grow_free_cycles() {
    let p = libs();
    let mut rng = Rng::new(13);
    for _ in 0..1000 {
        let elemsize = [1usize, 4, 8, 16][rng.below(4)];
        let n = rng.below(40);
        let mut c_a = Arr::new(&p.c, elemsize);
        let mut r_a = Arr::new(&p.r, elemsize);
        unsafe {
            c_a.grow(n, 0);
            r_a.grow(n, 0);
            assert_eq!(c_a.snap(), r_a.snap());
            c_a.free();
            r_a.free();
        }
    }
}

// ===========================================================================
// Rows 14-15: array macro pipelines
// ===========================================================================

#[test]
fn row14_arrput_pipeline() {
    let p = libs();
    let mut rng = Rng::new(14);
    for n in [0usize, 1, 2, 3, 4, 5, 7, 8, 9, 100, 500] {
        let mut c_a = Arr::new(&p.c, 4);
        let mut r_a = Arr::new(&p.r, 4);
        unsafe {
            for _ in 0..n {
                let v = rng.bytes(4);
                c_a.put(&v);
                r_a.put(&v);
                assert_eq!(c_a.snap(), r_a.snap(), "row14: n={n}");
            }
            c_a.free();
            r_a.free();
        }
    }
}

#[test]
fn row15_array_op_stream() {
    let p = libs();
    for &elemsize in &[4usize, 16] {
        let mut rng = Rng::new(0x150000 + elemsize as u64);
        let mut c_a = Arr::new(&p.c, elemsize);
        let mut r_a = Arr::new(&p.r, elemsize);
        unsafe {
            for step in 0..3000 {
                let len = c_a.len();
                match rng.below(8) {
                    0 | 1 | 2 => {
                        let v = rng.bytes(elemsize);
                        c_a.put(&v);
                        r_a.put(&v);
                    }
                    3 => {
                        if len > 0 {
                            assert_eq!(c_a.pop(), r_a.pop(), "pop @{step}");
                        }
                    }
                    4 => {
                        let n = rng.below(5);
                        assert_eq!(c_a.addnindex(n), r_a.addnindex(n), "addn @{step}");
                        // zero the freshly added slots identically
                        let l = c_a.len();
                        for i in (l - n)..l {
                            let v = rng.bytes(elemsize);
                            std::ptr::copy_nonoverlapping(
                                v.as_ptr(),
                                c_a.a.add(i * elemsize),
                                elemsize,
                            );
                            std::ptr::copy_nonoverlapping(
                                v.as_ptr(),
                                r_a.a.add(i * elemsize),
                                elemsize,
                            );
                        }
                    }
                    5 => {
                        if len > 0 {
                            let i = rng.below(len);
                            let n = rng.range(0, len - i);
                            c_a.deln(i, n);
                            r_a.deln(i, n);
                        }
                    }
                    6 => {
                        if len > 0 {
                            let i = rng.below(len);
                            c_a.delswap(i);
                            r_a.delswap(i);
                        }
                    }
                    _ => {
                        let i = rng.below(len + 1);
                        let v = rng.bytes(elemsize);
                        c_a.ins(i, &v);
                        r_a.ins(i, &v);
                    }
                }
                assert_eq!(c_a.snap(), r_a.snap(), "row15: elemsize={elemsize} step={step}");
                if rng.below(200) == 0 {
                    let n = rng.below(64);
                    c_a.setlen(n.min(c_a.len()));
                    r_a.setlen(n.min(r_a.len()));
                    c_a.setcap(n);
                    r_a.setcap(n);
                    assert_eq!(c_a.snap(), r_a.snap(), "row15 setlen/setcap step={step}");
                }
            }
            c_a.free();
            r_a.free();
        }
    }
}

// ===========================================================================
// Row 16: arr_push
// ===========================================================================

#[test]
fn row16_arr_push() {
    let p = libs();
    for num in [0i32, 1, 2, 49, 50, 51, 99, 100, 101, 1000, 5000] {
        unsafe {
            (p.c.arr_push)(num as c_int);
            (p.r.arr_push)(num as c_int);
        }
    }
    // arr_push has no observable output; prove the process is still healthy and
    // that both libraries still behave identically afterwards.
    hash_bytes_row(&[0x3141_5926], &[8], false, 16, "row16-after");
}

// ===========================================================================
// Row 17: strkey
// ===========================================================================

#[test]
fn row17_strkey() {
    let p = libs();
    let mut rng = Rng::new(17);
    let mut vals: Vec<i32> = vec![0, 1, -1, 9, 10, 42, -42, 99999, i32::MAX, i32::MIN];
    for _ in 0..500 {
        vals.push(rng.next_u64() as i32);
    }
    for n in vals {
        let (a, b) = unsafe {
            (
                cstr((p.c.strkey)(n as c_int)),
                cstr((p.r.strkey)(n as c_int)),
            )
        };
        assert_eq!(a, b, "row17: n={n} c={:?} r={:?}",
            String::from_utf8_lossy(&a), String::from_utf8_lossy(&b));
        assert_eq!(a, format!("test_{}", n).into_bytes());
    }
}

// ===========================================================================
// Rows 18-20: stbds_hmput_default
// ===========================================================================

#[test]
fn row18_hmput_default_from_null() {
    let p = libs();
    for elemsize in [1usize, 4, 8, 16, 24, 32, 64] {
        unsafe {
            let tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            let tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            assert_eq!(
                snap_map(tc, elemsize, KeyRepr::Raw),
                snap_map(tr, elemsize, KeyRepr::Raw),
                "row18: elemsize={elemsize}"
            );
            (p.c.hmfree_func)(tc.sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)(tr.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn row19_hmput_default_idempotent() {
    let p = libs();
    for elemsize in [4usize, 8, 16, 24] {
        unsafe {
            let mut tc = (p.c.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            let mut tr = (p.r.hmput_default)(std::ptr::null_mut(), elemsize) as *mut u8;
            // write a default value like stbds_hmdefault does: (t)[-1].value = v
            *(tc.sub(elemsize) as *mut u8) = 0x7e;
            *(tr.sub(elemsize) as *mut u8) = 0x7e;
            for _ in 0..5 {
                tc = (p.c.hmput_default)(tc as *mut c_void, elemsize) as *mut u8;
                tr = (p.r.hmput_default)(tr as *mut c_void, elemsize) as *mut u8;
                assert_eq!(
                    snap_map(tc, elemsize, KeyRepr::Raw),
                    snap_map(tr, elemsize, KeyRepr::Raw),
                    "row19: elemsize={elemsize}"
                );
            }
            (p.c.hmfree_func)(tc.sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)(tr.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

#[test]
fn row20_hmput_default_zero_length() {
    let p = libs();
    for elemsize in [4usize, 8, 16] {
        unsafe {
            // build an array with capacity but length forced to 0
            let ac = (p.c.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4) as *mut u8;
            let ar = (p.r.arrgrowf)(std::ptr::null_mut(), elemsize, 0, 4) as *mut u8;
            (*header(ac)).length = 0;
            (*header(ar)).length = 0;
            let tc = (p.c.hmput_default)(ac.add(elemsize) as *mut c_void, elemsize) as *mut u8;
            let tr = (p.r.hmput_default)(ar.add(elemsize) as *mut c_void, elemsize) as *mut u8;
            assert_eq!(
                snap_map(tc, elemsize, KeyRepr::Raw),
                snap_map(tr, elemsize, KeyRepr::Raw),
                "row20: elemsize={elemsize}"
            );
            (p.c.hmfree_func)(tc.sub(elemsize) as *mut c_void, elemsize);
            (p.r.hmfree_func)(tr.sub(elemsize) as *mut c_void, elemsize);
        }
    }
}

// ===========================================================================
// Rows 21-27: binary hash map put/get across shapes
// ===========================================================================

/// Insert `n` random keys of `keysize` bytes into an `elemsize` map on both
/// libraries, then look every key up plus `n` absent keys.
fn bin_map_row(
    elemsize: usize,
    keysize: usize,
    value_off: usize,
    n: usize,
    dup_rate: usize,
    rng_seed: u64,
    mode: c_int,
    label: &str,
) {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(rng_seed);
    let mut c_m = Map::new(&p.c, elemsize, keysize, value_off);
    let mut r_m = Map::new(&p.r, elemsize, keysize, value_off);
    let mut keys: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for i in 0..n {
            let key = if dup_rate > 0 && !keys.is_empty() && rng.below(dup_rate) == 0 {
                keys[rng.below(keys.len())].clone()
            } else {
                rng.bytes(keysize)
            };
            let value = rng.bytes(elemsize - value_off);
            let ic = c_m.hmput(&key, &value, mode);
            let ir = r_m.hmput(&key, &value, mode);
            assert_eq!(ic, ir, "{label}: put idx i={i}");
            assert_eq!(
                c_m.snap(KeyRepr::Raw),
                r_m.snap(KeyRepr::Raw),
                "{label}: after put i={i} key={key:02x?}"
            );
            keys.push(key);
        }
        for (i, k) in keys.iter().enumerate() {
            assert_eq!(
                c_m.hmgeti(k, mode),
                r_m.hmgeti(k, mode),
                "{label}: get i={i} key={k:02x?}"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "{label}: after get i={i}");
        }
        for i in 0..n.max(8) {
            let k = rng.bytes(keysize);
            assert_eq!(
                c_m.hmgeti(&k, mode),
                r_m.hmgeti(&k, mode),
                "{label}: miss i={i} key={k:02x?}"
            );
        }
        assert_eq!(hm_len(c_m.t, elemsize), hm_len(r_m.t, elemsize), "{label}: hmlen");
        c_m.free();
        r_m.free();
    }
}

#[test]
fn row21_bin_single_insert() {
    bin_map_row(E_INT_INT, 4, 4, 1, 0, 21, STBDS_HM_BINARY, "row21");
}

#[test]
fn row22_bin_below_threshold() {
    for n in [1usize, 2, 6] {
        bin_map_row(E_INT_INT, 4, 4, n, 0, 22 + n as u64, STBDS_HM_BINARY, "row22");
    }
}

#[test]
fn row23_bin_crosses_growth() {
    for n in [7usize, 8, 16, 24, 40, 64] {
        bin_map_row(E_INT_INT, 4, 4, n, 0, 230 + n as u64, STBDS_HM_BINARY, "row23");
    }
}

#[test]
fn row24_bin_with_duplicates() {
    bin_map_row(E_INT_INT, 4, 4, 300, 3, 24, STBDS_HM_BINARY, "row24");
}

#[test]
fn row25_bin_keysize8_elem16() {
    bin_map_row(E_SZ_SZ, 8, 8, 80, 5, 25, STBDS_HM_BINARY, "row25");
}

#[test]
fn row26_bin_keysize8_elem24() {
    bin_map_row(24, 8, 8, 80, 5, 26, STBDS_HM_BINARY, "row26");
}

#[test]
fn row27_bin_odd_keysizes() {
    // keysize 1 has only 256 distinct keys -> heavy duplicate/update traffic
    bin_map_row(E_INT_INT, 1, 4, 200, 2, 271, STBDS_HM_BINARY, "row27-ks1");
    bin_map_row(E_INT_INT, 3, 4, 200, 4, 272, STBDS_HM_BINARY, "row27-ks3");
    bin_map_row(E_SZ_SZ, 5, 8, 200, 4, 273, STBDS_HM_BINARY, "row27-ks5");
    bin_map_row(E_SZ_SZ, 7, 8, 200, 4, 274, STBDS_HM_BINARY, "row27-ks7");
}

// ===========================================================================
// Rows 28-32: delete / tombstone / shrink / rebuild
// ===========================================================================

#[test]
fn row28_bin_insert_delete_stream() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(28);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    let mut live: Vec<Vec<u8>> = Vec::new();
    unsafe {
        for step in 0..2000 {
            match rng.below(10) {
                0..=5 => {
                    // limit the key space so collisions and updates happen
                    let mut k = rng.bytes(4);
                    k[2] = 0;
                    k[3] = 0;
                    let v = rng.bytes(4);
                    assert_eq!(
                        c_m.hmput(&k, &v, STBDS_HM_BINARY),
                        r_m.hmput(&k, &v, STBDS_HM_BINARY),
                        "row28: put step={step}"
                    );
                    if !live.contains(&k) {
                        live.push(k);
                    }
                }
                6..=8 => {
                    let k = if !live.is_empty() && rng.bool() {
                        live[rng.below(live.len())].clone()
                    } else {
                        rng.bytes(4)
                    };
                    assert_eq!(
                        c_m.hmdel(&k, STBDS_HM_BINARY, 0),
                        r_m.hmdel(&k, STBDS_HM_BINARY, 0),
                        "row28: del step={step} key={k:02x?}"
                    );
                    live.retain(|x| x != &k);
                }
                _ => {
                    let k = if !live.is_empty() && rng.bool() {
                        live[rng.below(live.len())].clone()
                    } else {
                        rng.bytes(4)
                    };
                    assert_eq!(
                        c_m.hmgeti(&k, STBDS_HM_BINARY),
                        r_m.hmgeti(&k, STBDS_HM_BINARY),
                        "row28: get step={step}"
                    );
                }
            }
            assert_eq!(
                c_m.snap(KeyRepr::Raw),
                r_m.snap(KeyRepr::Raw),
                "row28: step={step}"
            );
        }
        for k in &live {
            assert_eq!(
                c_m.hmgeti(k, STBDS_HM_BINARY),
                r_m.hmgeti(k, STBDS_HM_BINARY),
                "row28: final get {k:02x?}"
            );
        }
        c_m.free();
        r_m.free();
    }
}

#[test]
fn row29_bin_delete_last() {
    let p = libs();
    for n in [1usize, 2, 5, 9, 20] {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(290 + n as u64);
        let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
        let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for _ in 0..n {
                let k = rng.bytes(4);
                let v = rng.bytes(4);
                c_m.hmput(&k, &v, STBDS_HM_BINARY);
                r_m.hmput(&k, &v, STBDS_HM_BINARY);
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            // delete in reverse insertion order: always old_index == final_index
            for k in keys.iter().rev() {
                assert_eq!(
                    c_m.hmdel(k, STBDS_HM_BINARY, 0),
                    r_m.hmdel(k, STBDS_HM_BINARY, 0),
                    "row29: n={n}"
                );
                assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row29: n={n}");
            }
            c_m.free();
            r_m.free();
        }
    }
}

#[test]
fn row30_bin_delete_middle() {
    let p = libs();
    for n in [3usize, 8, 17, 40] {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(300 + n as u64);
        let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
        let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
        let mut keys: Vec<Vec<u8>> = Vec::new();
        unsafe {
            for _ in 0..n {
                let k = rng.bytes(4);
                let v = rng.bytes(4);
                c_m.hmput(&k, &v, STBDS_HM_BINARY);
                r_m.hmput(&k, &v, STBDS_HM_BINARY);
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
            // always delete the front key: old_index != final_index -> memmove
            while !keys.is_empty() {
                let k = keys.remove(0);
                assert_eq!(
                    c_m.hmdel(&k, STBDS_HM_BINARY, 0),
                    r_m.hmdel(&k, STBDS_HM_BINARY, 0),
                    "row30: n={n}"
                );
                assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row30: n={n}");
                for kk in &keys {
                    assert_eq!(
                        c_m.hmgeti(kk, STBDS_HM_BINARY),
                        r_m.hmgeti(kk, STBDS_HM_BINARY),
                        "row30: relookup n={n}"
                    );
                }
            }
            c_m.free();
            r_m.free();
        }
    }
}

#[test]
fn row31_bin_delete_absent() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(31);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        for _ in 0..20 {
            let mut k = rng.bytes(4);
            k[3] = 0;
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
        }
        for _ in 0..200 {
            let mut k = rng.bytes(4);
            k[3] = 0xff; // guaranteed absent (inserted keys all have k[3]==0)
            let dc = c_m.hmdel(&k, STBDS_HM_BINARY, 0);
            let dr = r_m.hmdel(&k, STBDS_HM_BINARY, 0);
            assert_eq!(dc, dr, "row31: key={k:02x?}");
            assert_eq!(dc, 0, "row31: absent key must report 0");
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row31");
        }
        c_m.free();
        r_m.free();
    }
}

#[test]
fn row32_bin_tombstone_reuse() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(32);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        let keys: Vec<Vec<u8>> = (0..40u32).map(|i| i.to_le_bytes().to_vec()).collect();
        for k in &keys {
            let v = rng.bytes(4);
            c_m.hmput(k, &v, STBDS_HM_BINARY);
            r_m.hmput(k, &v, STBDS_HM_BINARY);
        }
        // delete every other key (creates tombstones), then re-insert them
        for k in keys.iter().step_by(2) {
            c_m.hmdel(k, STBDS_HM_BINARY, 0);
            r_m.hmdel(k, STBDS_HM_BINARY, 0);
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row32 del");
        }
        for k in keys.iter().step_by(2) {
            let v = rng.bytes(4);
            assert_eq!(
                c_m.hmput(k, &v, STBDS_HM_BINARY),
                r_m.hmput(k, &v, STBDS_HM_BINARY),
                "row32 reinsert"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row32 reinsert");
        }
        for k in &keys {
            assert_eq!(
                c_m.hmgeti(k, STBDS_HM_BINARY),
                r_m.hmgeti(k, STBDS_HM_BINARY),
                "row32 final get"
            );
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// Rows 33-34: hmget_key_ts and hmget_key
// ===========================================================================

#[test]
fn row33_hmget_key_ts() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(33);
    let mut c_m = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_m = Map::new(&p.r, E_INT_INT, 4, 4);
    unsafe {
        // first call on a NULL table
        let k0 = rng.bytes(4);
        assert_eq!(
            c_m.hmgeti_ts(&k0, STBDS_HM_BINARY),
            r_m.hmgeti_ts(&k0, STBDS_HM_BINARY),
            "row33: first ts call"
        );
        assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row33: after first");
        let mut keys = Vec::new();
        for _ in 0..50 {
            let k = rng.bytes(4);
            let v = rng.bytes(4);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
            keys.push(k);
        }
        for k in &keys {
            assert_eq!(
                c_m.hmgeti_ts(k, STBDS_HM_BINARY),
                r_m.hmgeti_ts(k, STBDS_HM_BINARY),
                "row33: hit"
            );
        }
        for _ in 0..50 {
            let k = rng.bytes(4);
            assert_eq!(
                c_m.hmgeti_ts(&k, STBDS_HM_BINARY),
                r_m.hmgeti_ts(&k, STBDS_HM_BINARY),
                "row33: miss"
            );
        }
        c_m.free();
        r_m.free();
    }
}

#[test]
fn row34_hmget_key_vs_ts() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(34);
    let mut c_m = Map::new(&p.c, E_SZ_SZ, 8, 8);
    let mut r_m = Map::new(&p.r, E_SZ_SZ, 8, 8);
    unsafe {
        let mut keys = Vec::new();
        for _ in 0..64 {
            let k = rng.bytes(8);
            let v = rng.bytes(8);
            c_m.hmput(&k, &v, STBDS_HM_BINARY);
            r_m.hmput(&k, &v, STBDS_HM_BINARY);
            keys.push(k);
        }
        for k in &keys {
            let ct = c_m.hmgeti_ts(k, STBDS_HM_BINARY);
            let rt = r_m.hmgeti_ts(k, STBDS_HM_BINARY);
            let ch = c_m.hmgeti(k, STBDS_HM_BINARY);
            let rh = r_m.hmgeti(k, STBDS_HM_BINARY);
            assert_eq!((ct, ch), (rt, rh), "row34");
            assert_eq!(ct, ch, "row34: ts out-param vs header temp (C)");
            assert_eq!(rt, rh, "row34: ts out-param vs header temp (Rust)");
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// Rows 35-41: string maps in every string.mode
// ===========================================================================

/// `sh_mode`: `None` = let `hmput_key` create the table implicitly
/// (`STBDS_SH_DEFAULT`); `Some(m)` = pre-create with `stbds_shmode_func(m)`.
fn str_map_row(sh_mode: Option<c_int>, n: usize, rng_seed: u64, mode: c_int, label: &str) {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(rng_seed);
    let keys = key_pool((n.max(1) * 2).min(4096));
    let kr = KeyRepr::PtrString { off: 0 };
    let mut c_m = Map::new(&p.c, E_STR, 8, 8);
    let mut r_m = Map::new(&p.r, E_STR, 8, 8);
    unsafe {
        if let Some(m) = sh_mode {
            c_m.sh_new(m);
            r_m.sh_new(m);
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "{label}: sh_new");
        }
        let mut used: Vec<*mut c_char> = Vec::new();
        for i in 0..n {
            let k = keys[rng.below(keys.len())];
            let v = rng.bytes(8);
            assert_eq!(
                c_m.shput(k, &v, mode),
                r_m.shput(k, &v, mode),
                "{label}: put i={i}"
            );
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "{label}: after put i={i}");
            if !used.contains(&k) {
                used.push(k);
            }
        }
        for k in &used {
            assert_eq!(
                c_m.shgeti(*k, mode),
                r_m.shgeti(*k, mode),
                "{label}: get {:?}",
                String::from_utf8_lossy(&cstr(*k))
            );
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "{label}: after get");
        }
        for _ in 0..32 {
            let k = keys[rng.below(keys.len())];
            assert_eq!(c_m.shgeti(k, mode), r_m.shgeti(k, mode), "{label}: mixed get");
        }
        assert_eq!(hm_len(c_m.t, E_STR), hm_len(r_m.t, E_STR), "{label}: hmlen");
        c_m.free();
        r_m.free();
    }
}

#[test]
fn row35_str_sh_default_implicit() {
    for n in [1usize, 2, 6, 7, 16, 64] {
        str_map_row(None, n, 350 + n as u64, STBDS_HM_STRING, "row35");
    }
}

#[test]
fn row36_str_sh_strdup() {
    for n in [1usize, 6, 7, 40, 128] {
        str_map_row(Some(STBDS_SH_STRDUP), n, 360 + n as u64, STBDS_HM_STRING, "row36");
    }
}

#[test]
fn row37_str_sh_arena() {
    for n in [1usize, 6, 7, 40, 128] {
        str_map_row(Some(STBDS_SH_ARENA), n, 370 + n as u64, STBDS_HM_STRING, "row37");
    }
}

/// long keys force `stbds_stralloc`'s oversized-block path from inside the map
#[test]
fn row37b_str_sh_arena_long_keys() {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(3712);
    let kr = KeyRepr::PtrString { off: 0 };
    let owned: Vec<Vec<u8>> = (0..60usize)
        .map(|i| {
            let mut s = vec![b'a' + (i % 26) as u8; 1 + i * 25];
            s.push(0);
            s
        })
        .collect();
    let mut c_m = Map::new(&p.c, E_STR, 8, 8);
    let mut r_m = Map::new(&p.r, E_STR, 8, 8);
    unsafe {
        c_m.sh_new(STBDS_SH_ARENA);
        r_m.sh_new(STBDS_SH_ARENA);
        for (i, k) in owned.iter().enumerate() {
            let kp = k.as_ptr() as *mut c_char;
            let v = rng.bytes(8);
            assert_eq!(
                c_m.shput(kp, &v, STBDS_HM_STRING),
                r_m.shput(kp, &v, STBDS_HM_STRING)
            );
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "row37b: put i={i}");
        }
        for k in &owned {
            let kp = k.as_ptr() as *mut c_char;
            assert_eq!(
                c_m.shgeti(kp, STBDS_HM_STRING),
                r_m.shgeti(kp, STBDS_HM_STRING)
            );
        }
        c_m.free();
        r_m.free();
    }
}

fn str_del_row(sh_mode: Option<c_int>, rng_seed: u64, label: &str) {
    let p = libs();
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(rng_seed);
    let kr = KeyRepr::PtrString { off: 0 };
    let keys = key_pool(64);
    let mut c_m = Map::new(&p.c, E_STR, 8, 8);
    let mut r_m = Map::new(&p.r, E_STR, 8, 8);
    unsafe {
        if let Some(m) = sh_mode {
            c_m.sh_new(m);
            r_m.sh_new(m);
        }
        for k in &keys {
            let v = rng.bytes(8);
            c_m.shput(*k, &v, STBDS_HM_STRING);
            r_m.shput(*k, &v, STBDS_HM_STRING);
        }
        assert_eq!(c_m.snap(kr), r_m.snap(kr), "{label}: filled");
        // delete from the front: old_index != final_index -> char** re-lookup
        let mut remaining: Vec<*mut c_char> = keys.clone();
        while !remaining.is_empty() {
            let k = remaining.remove(0);
            assert_eq!(
                c_m.shdel(k, STBDS_HM_STRING, 0),
                r_m.shdel(k, STBDS_HM_STRING, 0),
                "{label}: del {:?}",
                String::from_utf8_lossy(&cstr(k))
            );
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "{label}: after del");
            assert_eq!(
                c_m.shgeti(k, STBDS_HM_STRING),
                r_m.shgeti(k, STBDS_HM_STRING),
                "{label}: deleted key must miss identically"
            );
            for r in &remaining {
                assert_eq!(
                    c_m.shgeti(*r, STBDS_HM_STRING),
                    r_m.shgeti(*r, STBDS_HM_STRING),
                    "{label}: survivor lookup"
                );
            }
        }
        c_m.free();
        r_m.free();
    }
}

#[test]
fn row38_str_strdup_delete() {
    str_del_row(Some(STBDS_SH_STRDUP), 38, "row38");
}

#[test]
fn row39_str_arena_delete() {
    str_del_row(Some(STBDS_SH_ARENA), 39, "row39");
}

#[test]
fn row40_str_default_delete() {
    str_del_row(None, 40, "row40");
}

#[test]
fn row41_str_op_stream() {
    let p = libs();
    let kr = KeyRepr::PtrString { off: 0 };
    for (i, (tag, sh)) in [
        ("default", None),
        ("strdup", Some(STBDS_SH_STRDUP)),
        ("arena", Some(STBDS_SH_ARENA)),
    ]
    .into_iter()
    .enumerate()
    {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(4100 + i as u64);
        let keys = key_pool(96);
        let mut c_m = Map::new(&p.c, E_STR, 8, 8);
        let mut r_m = Map::new(&p.r, E_STR, 8, 8);
        unsafe {
            if let Some(m) = sh {
                c_m.sh_new(m);
                r_m.sh_new(m);
            }
            for step in 0..1000 {
                let k = keys[rng.below(keys.len())];
                match rng.below(10) {
                    0..=5 => {
                        let v = rng.bytes(8);
                        assert_eq!(
                            c_m.shput(k, &v, STBDS_HM_STRING),
                            r_m.shput(k, &v, STBDS_HM_STRING),
                            "row41-{tag}: put step={step}"
                        );
                    }
                    6..=8 => {
                        assert_eq!(
                            c_m.shdel(k, STBDS_HM_STRING, 0),
                            r_m.shdel(k, STBDS_HM_STRING, 0),
                            "row41-{tag}: del step={step}"
                        );
                    }
                    _ => {
                        assert_eq!(
                            c_m.shgeti(k, STBDS_HM_STRING),
                            r_m.shgeti(k, STBDS_HM_STRING),
                            "row41-{tag}: get step={step}"
                        );
                    }
                }
                assert_eq!(c_m.snap(kr), r_m.snap(kr), "row41-{tag}: step={step}");
            }
            c_m.free();
            r_m.free();
        }
    }
}

// ===========================================================================
// Rows 42-45: mode / shmode / keyoffset axes
// ===========================================================================

#[test]
fn row42_shmode_func_all_modes() {
    let p = libs();
    for m in [0i32, 1, 2, 3, 4, 5, 127, 255, 256, 257, -1, -2, i32::MAX, i32::MIN] {
        for elemsize in [8usize, 16, 24, 32] {
            reset_seeds(p, 0x3141_5926);
            unsafe {
                let tc = (p.c.shmode_func)(elemsize, m as c_int) as *mut u8;
                let tr = (p.r.shmode_func)(elemsize, m as c_int) as *mut u8;
                assert_eq!(
                    snap_map(tc, elemsize, KeyRepr::Raw),
                    snap_map(tr, elemsize, KeyRepr::Raw),
                    "row42: mode={m} elemsize={elemsize}"
                );
                let tbl = hm_table(tc, elemsize);
                assert_eq!(
                    (*tbl).string.mode,
                    (m as u32 & 0xff) as u8,
                    "row42: (unsigned char) truncation, mode={m}"
                );
                (p.c.hmfree_func)(tc.sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)(tr.sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

/// `mode >= 2`: string hash + `strcmp` compare, but `hmdel_key`'s
/// `mode == STBDS_HM_STRING` tests are false.
#[test]
fn row43_mode_out_of_range_positive() {
    let p = libs();
    let kr = KeyRepr::PtrString { off: 0 };
    for (i, mode) in [2i32, 3, 7, 1000, i32::MAX].into_iter().enumerate() {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(4300 + i as u64);
        let keys = key_pool(24);
        let mut c_m = Map::new(&p.c, E_STR, 8, 8);
        let mut r_m = Map::new(&p.r, E_STR, 8, 8);
        unsafe {
            for k in &keys {
                let v = rng.bytes(8);
                assert_eq!(
                    c_m.shput(*k, &v, mode as c_int),
                    r_m.shput(*k, &v, mode as c_int),
                    "row43: put mode={mode}"
                );
                assert_eq!(c_m.snap(kr), r_m.snap(kr), "row43: put mode={mode}");
            }
            for k in &keys {
                assert_eq!(
                    c_m.shgeti(*k, mode as c_int),
                    r_m.shgeti(*k, mode as c_int),
                    "row43: get mode={mode}"
                );
            }
            // delete the LAST element only: old_index == final_index, so the
            // char**-vs-raw-bytes re-lookup divergence is not reached.
            let last = keys[keys.len() - 1];
            assert_eq!(
                c_m.shdel(last, mode as c_int, 0),
                r_m.shdel(last, mode as c_int, 0),
                "row43: del mode={mode}"
            );
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "row43: del mode={mode}");
            c_m.free();
            r_m.free();
        }
    }
}

/// negative / zero mode -> binary path
#[test]
fn row44_mode_negative_is_binary() {
    for (i, mode) in [0i32, -1, -2, -1000, i32::MIN].into_iter().enumerate() {
        bin_map_row(E_INT_INT, 4, 4, 60, 4, 4400 + i as u64, mode as c_int, "row44");
    }
}

#[test]
fn row45_hmdel_keyoffset_nonzero() {
    let p = libs();
    const ES: usize = 16; // { u32 key; u32 mirror; u64 value; }
    const KO: usize = 4;
    reset_seeds(p, 0x3141_5926);
    let mut rng = Rng::new(45);
    let mut c_m = Map::new(&p.c, ES, 4, 8);
    let mut r_m = Map::new(&p.r, ES, 4, 8);
    unsafe {
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for _ in 0..30 {
            let k = rng.bytes(4);
            let v = rng.bytes(8);
            let ic = c_m.hmput(&k, &v, STBDS_HM_BINARY);
            let ir = r_m.hmput(&k, &v, STBDS_HM_BINARY);
            assert_eq!(ic, ir);
            // mirror the key bytes at offset KO on both sides
            std::ptr::copy_nonoverlapping(k.as_ptr(), c_m.t.offset(ic * ES as isize).add(KO), 4);
            std::ptr::copy_nonoverlapping(k.as_ptr(), r_m.t.offset(ir * ES as isize).add(KO), 4);
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row45: filled");
        // delete the last element repeatedly with keyoffset = KO
        while let Some(k) = keys.pop() {
            assert_eq!(
                c_m.hmdel(&k, STBDS_HM_BINARY, KO),
                r_m.hmdel(&k, STBDS_HM_BINARY, KO),
                "row45: del"
            );
            assert_eq!(c_m.snap(KeyRepr::Raw), r_m.snap(KeyRepr::Raw), "row45: after del");
        }
        c_m.free();
        r_m.free();
    }
}

// ===========================================================================
// Row 46: stbds_hmfree_func in every string.mode
// ===========================================================================

#[test]
fn row46_hmfree_func_all_modes() {
    let p = libs();
    unsafe {
        // a == NULL: must be a no-op on both
        (p.c.hmfree_func)(std::ptr::null_mut(), 16);
        (p.r.hmfree_func)(std::ptr::null_mut(), 16);
    }
    for sh in [
        None,
        Some(STBDS_SH_NONE),
        Some(STBDS_SH_DEFAULT),
        Some(STBDS_SH_STRDUP),
        Some(STBDS_SH_ARENA),
    ] {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(46);
        let keys = key_pool(32);
        let mut c_m = Map::new(&p.c, E_STR, 8, 8);
        let mut r_m = Map::new(&p.r, E_STR, 8, 8);
        unsafe {
            if let Some(m) = sh {
                c_m.sh_new(m);
                r_m.sh_new(m);
            }
            let binary = sh == Some(STBDS_SH_NONE);
            let mode = if binary { STBDS_HM_BINARY } else { STBDS_HM_STRING };
            for k in &keys {
                let v = rng.bytes(8);
                if binary {
                    let kb = (*k as usize).to_ne_bytes().to_vec();
                    c_m.hmput(&kb, &v, mode);
                    r_m.hmput(&kb, &v, mode);
                } else {
                    c_m.shput(*k, &v, mode);
                    r_m.shput(*k, &v, mode);
                }
            }
            let kr = if binary { KeyRepr::Raw } else { KeyRepr::PtrString { off: 0 } };
            assert_eq!(c_m.snap(kr), r_m.snap(kr), "row46: sh={sh:?}");
            c_m.free();
            r_m.free();
            assert!(c_m.t.is_null() && r_m.t.is_null());
        }
    }
}

// ===========================================================================
// Rows 47-51: stbds_stralloc / stbds_strreset
// ===========================================================================

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
struct Arena {
    storage: usize, // stbds_string_block *
    remaining: usize,
    block: u8,
    mode: u8,
    _pad: [u8; 6],
}

#[derive(Debug, PartialEq, Eq)]
struct ArenaObs {
    ret: Vec<u8>,
    storage_null: bool,
    remaining: usize,
    block: u8,
    mode: u8,
}

impl Arena {
    /// Pointer-value-independent view (the `storage` address necessarily
    /// differs between the two libraries).
    fn norm(&self) -> (bool, usize, u8, u8) {
        (self.storage == 0, self.remaining, self.block, self.mode)
    }
}

unsafe fn stralloc_obs(lib: &Lib, ar: &mut Arena, s: &mut Vec<u8>) -> ArenaObs {
    unsafe {
        let r = (lib.stralloc)(ar as *mut Arena as *mut c_void, s.as_mut_ptr() as *mut c_char);
        ArenaObs {
            ret: cstr(r),
            storage_null: ar.storage == 0,
            remaining: ar.remaining,
            block: ar.block,
            mode: ar.mode,
        }
    }
}

fn zstr(len: usize, fill: u8) -> Vec<u8> {
    let mut v = vec![fill; len];
    v.push(0);
    v
}

#[test]
fn row47_stralloc_bump_path() {
    let p = libs();
    let mut rng = Rng::new(47);
    for trial in 0..40 {
        let mut ac = Arena::default();
        let mut ar = Arena::default();
        unsafe {
            for step in 0..60 {
                let len = rng.range(0, 64);
                let mut s = zstr(len, b'a' + (step % 26) as u8);
                let mut s2 = s.clone();
                let oc = stralloc_obs(&p.c, &mut ac, &mut s);
                let or = stralloc_obs(&p.r, &mut ar, &mut s2);
                assert_eq!(oc, or, "row47: trial={trial} step={step} len={len}");
                assert_eq!(oc.ret, s[..len], "row47: returned copy");
            }
            (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
            (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
            assert_eq!(ac, ar, "row47: after reset");
            assert_eq!(ac, Arena::default(), "row47: reset zeroes the arena");
        }
    }
}

#[test]
fn row48_stralloc_oversized_first() {
    let p = libs();
    for len in [512usize, 511, 513, 1000, 4096, 100_000] {
        let mut ac = Arena::default();
        let mut ar = Arena::default();
        let mut s = zstr(len, b'Z');
        let mut s2 = s.clone();
        unsafe {
            let oc = stralloc_obs(&p.c, &mut ac, &mut s);
            let or = stralloc_obs(&p.r, &mut ar, &mut s2);
            assert_eq!(oc, or, "row48: len={len}");
            assert_eq!(oc.ret.len(), len, "row48: content length");
            (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
            (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
            assert_eq!(ac, ar);
        }
    }
}

#[test]
fn row49_stralloc_oversized_after_block() {
    let p = libs();
    let mut ac = Arena::default();
    let mut ar = Arena::default();
    unsafe {
        // establish a normal 512-byte block first
        let mut s = zstr(8, b'x');
        let mut s2 = s.clone();
        assert_eq!(
            stralloc_obs(&p.c, &mut ac, &mut s),
            stralloc_obs(&p.r, &mut ar, &mut s2),
            "row49: seed block"
        );
        for len in [600usize, 5000, 100_000, 3] {
            let mut b = zstr(len, b'q');
            let mut b2 = b.clone();
            let oc = stralloc_obs(&p.c, &mut ac, &mut b);
            let or = stralloc_obs(&p.r, &mut ar, &mut b2);
            assert_eq!(oc, or, "row49: len={len}");
        }
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);
        assert_eq!(ac, Arena::default());
    }
}

#[test]
fn row50_stralloc_block_counter_sweep() {
    let p = libs();
    let mut rng = Rng::new(50);
    let mut skipped = 0usize;
    let mut tested = 0usize;
    for block in 0u16..=255 {
        // blocksize = 512 << ((block>>1) & 63)   (x86-64 masks the shift count)
        let k = ((block >> 1) as u32) & 63;
        let blocksize = 512usize.wrapping_shl(k);
        if blocksize > (1 << 24) {
            skipped += 1;
            continue; // would allocate gigabytes; not a behaviour difference
        }
        tested += 1;
        for len in [1usize, 7, 511, 512, 513, rng.range(1, 2000)] {
            let mut ac = Arena { block: block as u8, ..Default::default() };
            let mut ar = Arena { block: block as u8, ..Default::default() };
            let mut s = zstr(len, b'k');
            let mut s2 = s.clone();
            unsafe {
                let oc = stralloc_obs(&p.c, &mut ac, &mut s);
                let or = stralloc_obs(&p.r, &mut ar, &mut s2);
                assert_eq!(oc, or, "row50: block={block} len={len} blocksize={blocksize}");
                assert_eq!(oc.ret.len(), len);
                (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
                (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
                assert_eq!(ac, ar, "row50: reset block={block}");
            }
        }
    }
    assert!(tested >= 100, "row50: only {tested} block values tested ({skipped} skipped)");
}

#[test]
fn row51_strreset_chain_and_empty() {
    let p = libs();
    unsafe {
        // strreset on an already-zero arena
        let mut ac = Arena::default();
        let mut ar = Arena::default();
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);
        assert_eq!(ac, Arena::default());

        // build a multi-block chain (normal + oversized), reset, reuse
        for round in 0..3 {
            for len in [16usize, 700, 32, 40_000, 8] {
                let mut s = zstr(len, b'm');
                let mut s2 = s.clone();
                let oc = stralloc_obs(&p.c, &mut ac, &mut s);
                let or = stralloc_obs(&p.r, &mut ar, &mut s2);
                assert_eq!(oc, or, "row51: round={round} len={len}");
            }
            // fill the current block completely so a new one is needed
            for _ in 0..200 {
                let mut s = zstr(30, b'n');
                let mut s2 = s.clone();
                assert_eq!(
                    stralloc_obs(&p.c, &mut ac, &mut s),
                    stralloc_obs(&p.r, &mut ar, &mut s2),
                    "row51: fill round={round}"
                );
            }
            (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
            (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
            assert_eq!(ac, ar, "row51: reset round={round}");
            assert_eq!(ac, Arena::default(), "row51: fully zeroed round={round}");
        }
    }
}

// ===========================================================================
// Row 52: everything live at once
// ===========================================================================

#[test]
fn row52_mixed_stress() {
    let p = libs();
    reset_seeds(p, 0xabcd_ef01);
    let mut rng = Rng::new(52);
    let keys = key_pool(128);
    let bin_kr = KeyRepr::Raw;
    let str_kr = KeyRepr::PtrString { off: 0 };

    let mut c_bin = Map::new(&p.c, E_INT_INT, 4, 4);
    let mut r_bin = Map::new(&p.r, E_INT_INT, 4, 4);
    let mut c_str = Map::new(&p.c, E_STR, 8, 8);
    let mut r_str = Map::new(&p.r, E_STR, 8, 8);
    let mut c_arr = Arr::new(&p.c, 8);
    let mut r_arr = Arr::new(&p.r, 8);
    let mut ac = Arena::default();
    let mut ar = Arena::default();

    unsafe {
        c_str.sh_new(STBDS_SH_STRDUP);
        r_str.sh_new(STBDS_SH_STRDUP);
        for step in 0..5000 {
            match rng.below(12) {
                0 | 1 => {
                    let mut k = rng.bytes(4);
                    k[3] = 0;
                    let v = rng.bytes(4);
                    assert_eq!(
                        c_bin.hmput(&k, &v, STBDS_HM_BINARY),
                        r_bin.hmput(&k, &v, STBDS_HM_BINARY),
                        "row52: bin put step={step}"
                    );
                }
                2 => {
                    let mut k = rng.bytes(4);
                    k[3] = 0;
                    assert_eq!(
                        c_bin.hmdel(&k, STBDS_HM_BINARY, 0),
                        r_bin.hmdel(&k, STBDS_HM_BINARY, 0),
                        "row52: bin del step={step}"
                    );
                }
                3 => {
                    let mut k = rng.bytes(4);
                    k[3] = 0;
                    assert_eq!(
                        c_bin.hmgeti(&k, STBDS_HM_BINARY),
                        r_bin.hmgeti(&k, STBDS_HM_BINARY),
                        "row52: bin get step={step}"
                    );
                }
                4 | 5 => {
                    let k = keys[rng.below(keys.len())];
                    let v = rng.bytes(8);
                    assert_eq!(
                        c_str.shput(k, &v, STBDS_HM_STRING),
                        r_str.shput(k, &v, STBDS_HM_STRING),
                        "row52: str put step={step}"
                    );
                }
                6 => {
                    let k = keys[rng.below(keys.len())];
                    assert_eq!(
                        c_str.shdel(k, STBDS_HM_STRING, 0),
                        r_str.shdel(k, STBDS_HM_STRING, 0),
                        "row52: str del step={step}"
                    );
                }
                7 => {
                    let k = keys[rng.below(keys.len())];
                    assert_eq!(
                        c_str.shgeti(k, STBDS_HM_STRING),
                        r_str.shgeti(k, STBDS_HM_STRING),
                        "row52: str get step={step}"
                    );
                }
                8 | 9 => {
                    let v = rng.bytes(8);
                    c_arr.put(&v);
                    r_arr.put(&v);
                }
                10 => {
                    let l = c_arr.len();
                    if l > 0 {
                        let i = rng.below(l);
                        if rng.bool() {
                            c_arr.delswap(i);
                            r_arr.delswap(i);
                        } else {
                            let n = rng.range(0, l - i);
                            c_arr.deln(i, n);
                            r_arr.deln(i, n);
                        }
                    }
                }
                _ => {
                    let len = rng.range(0, 900);
                    let mut s = zstr(len, b's');
                    let mut s2 = s.clone();
                    assert_eq!(
                        stralloc_obs(&p.c, &mut ac, &mut s),
                        stralloc_obs(&p.r, &mut ar, &mut s2),
                        "row52: arena step={step}"
                    );
                }
            }
            assert_eq!(c_bin.snap(bin_kr), r_bin.snap(bin_kr), "row52: bin snap step={step}");
            assert_eq!(c_str.snap(str_kr), r_str.snap(str_kr), "row52: str snap step={step}");
            assert_eq!(c_arr.snap(), r_arr.snap(), "row52: arr snap step={step}");
            assert_eq!(ac.norm(), ar.norm(), "row52: arena snap step={step}");
        }
        c_bin.free();
        r_bin.free();
        c_str.free();
        r_str.free();
        c_arr.free();
        r_arr.free();
        (p.c.strreset)(&mut ac as *mut Arena as *mut c_void);
        (p.r.strreset)(&mut ar as *mut Arena as *mut c_void);
        assert_eq!(ac, ar);
        assert_eq!(ac, Arena::default());
    }
}

// ===========================================================================
// Row 35c: stbds_temp_key (written by hmput_key on the string paths)
//
// `stbds_make_hash_index` never initialises `temp_key`, so it is compared
// separately here -- only immediately after operations the C source provably
// writes it (insert with string.mode 1/2/3, and a first-scan hit with
// mode >= STBDS_HM_STRING).
// ===========================================================================

#[test]
fn row35c_temp_key() {
    let p = libs();
    for (i, sh) in [None, Some(STBDS_SH_DEFAULT), Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA)]
        .into_iter()
        .enumerate()
    {
        reset_seeds(p, 0x3141_5926);
        let mut rng = Rng::new(3500 + i as u64);
        let keys = key_pool(48);
        let mut c_m = Map::new(&p.c, E_STR, 8, 8);
        let mut r_m = Map::new(&p.r, E_STR, 8, 8);
        unsafe {
            if let Some(m) = sh {
                c_m.sh_new(m);
                r_m.sh_new(m);
            }
            // Only NEW keys: an insert always runs the
            // `switch (table->string.mode)` arm that writes temp_key.
            let mut seen: Vec<*mut c_char> = Vec::new();
            for k in &keys {
                if seen.contains(k) {
                    continue;
                }
                seen.push(*k);
                let v = rng.bytes(8);
                c_m.shput(*k, &v, STBDS_HM_STRING);
                r_m.shput(*k, &v, STBDS_HM_STRING);
                let tc = table_temp_key(c_m.t, E_STR);
                let tr = table_temp_key(r_m.t, E_STR);
                assert_eq!(tc, tr, "row35c: after insert, sh={sh:?}");
                assert_eq!(
                    tc.as_deref(),
                    Some(&cstr(*k)[..]),
                    "row35c: temp_key must name the inserted key"
                );
            }
            // NOTE: re-putting an existing key is deliberately NOT checked.
            // `stbds_hmput_key` writes temp_key on a first-scan hit but not on a
            // wrap-around-scan hit, and `stbds_make_hash_index` never
            // initialises temp_key when it rehashes, so after a grow the value
            // is indeterminate in the C original too.
            c_m.free();
            r_m.free();
        }
    }
}

// ===========================================================================
// Rows 2-7 addendum: unaligned buffers
//
// `stbds_siphash_bytes` and `stbds_hash_string` read byte-at-a-time, so the
// result must not depend on the alignment of the input pointer.
// ===========================================================================

#[test]
fn row05b_hash_unaligned_buffers() {
    let p = libs();
    let mut rng = Rng::new(52001);
    let mut backing = vec![0u8; 4096];
    for b in backing.iter_mut() {
        *b = rng.byte();
    }
    unsafe {
        for off in 0..16usize {
            for len in [0usize, 1, 3, 7, 8, 9, 15, 16, 17, 31, 64, 129] {
                let ptr = backing.as_mut_ptr().add(off) as *mut c_void;
                for &seed in &SEEDS {
                    assert_eq!(
                        (p.c.hash_bytes)(ptr, len, seed),
                        (p.r.hash_bytes)(ptr, len, seed),
                        "row05b: off={off} len={len} seed={seed:#x}"
                    );
                }
            }
        }
        // hash_string at every alignment
        let mut strs = vec![0u8; 512];
        for i in 0..400 {
            strs[i] = (rng.byte() % 94) + 33;
        }
        strs[400] = 0;
        for off in 0..16usize {
            for &seed in &SEEDS {
                let ptr = strs.as_mut_ptr().add(off) as *mut c_char;
                assert_eq!(
                    (p.c.hash_string)(ptr, seed),
                    (p.r.hash_string)(ptr, seed),
                    "row05b: hash_string off={off}"
                );
            }
        }
    }
}
