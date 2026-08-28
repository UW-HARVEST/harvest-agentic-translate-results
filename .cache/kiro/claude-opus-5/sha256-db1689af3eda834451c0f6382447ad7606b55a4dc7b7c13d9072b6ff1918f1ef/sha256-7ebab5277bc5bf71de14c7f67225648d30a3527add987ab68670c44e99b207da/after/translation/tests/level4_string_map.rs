//! Level 4: string-keyed maps — `stbds_shmode_func` plus `stbds_hmput_key` /
//! `stbds_hmget_key` / `stbds_hmget_key_ts` / `stbds_hmdel_key` /
//! `stbds_hmfree_func` in `STBDS_HM_STRING` mode, across all three key-storage
//! modes (`SH_DEFAULT`, `SH_STRDUP`, `SH_ARENA`).
mod harness;

use harness::*;
use std::ffi::{CString, c_void};
use std::os::raw::{c_char, c_int};

const ELEM: usize = std::mem::size_of::<StrEntry>(); // 16
const KEYSIZE: usize = std::mem::size_of::<*mut c_char>(); // 8

struct SMap<'a> {
    im: &'a Impl,
    t: *mut c_void,
}

impl<'a> SMap<'a> {
    /// `strmap = NULL` — the first `shput` implicitly selects `SH_DEFAULT`.
    fn new_default(im: &'a Impl) -> Self {
        SMap {
            im,
            t: std::ptr::null_mut(),
        }
    }

    /// `sh_new_strdup(t)` / `sh_new_arena(t)`
    fn new_mode(im: &'a Impl, mode: c_int) -> Self {
        let t = unsafe { (im.shmode_func)(ELEM, mode) };
        SMap { im, t }
    }

    fn raw(&self) -> *mut c_void {
        unsafe { (self.t as *mut u8).sub(ELEM) as *mut c_void }
    }

    fn temp(&self) -> isize {
        unsafe { header(self.raw()).temp }
    }

    /// `*(char **) stbds_header(t-1)->hash_table` — `stbds_temp_key`
    unsafe fn temp_key(&self) -> *mut c_char {
        unsafe { *(header(self.raw()).hash_table as *mut *mut c_char) }
    }

    /// `stbds_shput(t, k, v)`. Returns `(index, inserted_new_element)`.
    fn put(&mut self, k: *const c_char, v: c_int) -> (isize, bool) {
        let before = self.len();
        self.t = unsafe {
            (self.im.hmput_key)(self.t, ELEM, k as *mut c_void, KEYSIZE, HM_STRING)
        };
        let idx = self.temp();
        unsafe {
            (*(self.t as *mut StrEntry).offset(idx)).value = v;
        }
        (idx, self.len() == before + 1)
    }

    /// `stbds_shputs(t, s)` — assigns the whole struct then restores the key
    /// from `stbds_temp_key`, exactly as the macro does.
    ///
    /// Only valid when the put inserts a *new* element: `stbds_hmput_key`
    /// writes `temp_key` on the insert path, but a fresh `stbds_hash_index`
    /// leaves it uninitialised, so `shputs` on an already-present key can read
    /// garbage. `str_dups` only ever uses it on a fresh insert.
    fn puts(&mut self, k: *const c_char, v: c_int) -> isize {
        let before = self.len();
        self.t = unsafe {
            (self.im.hmput_key)(self.t, ELEM, k as *mut c_void, KEYSIZE, HM_STRING)
        };
        unsafe {
            let idx = self.temp();
            let e = (self.t as *mut StrEntry).offset(idx);
            (*e).key = k as *mut c_char;
            (*e).value = v;
            let idx = self.temp();
            let e = (self.t as *mut StrEntry).offset(idx);
            assert!(
                self.len() == before + 1,
                "shputs driver used on an already-present key"
            );
            (*e).key = self.temp_key();
            idx
        }
    }

    /// Content of `stbds_temp_key`. Only safe directly after an insert.
    fn temp_key_str(&self) -> Option<Vec<u8>> {
        unsafe { cstr(self.temp_key()) }
    }

    /// `stbds_shgeti(t, k)`
    fn geti(&mut self, k: *const c_char) -> isize {
        self.t = unsafe {
            (self.im.hmget_key)(self.t, ELEM, k as *mut c_void, KEYSIZE, HM_STRING)
        };
        self.temp()
    }

    /// `stbds_shgeti` via the `_ts` entry point
    fn geti_ts(&mut self, k: *const c_char) -> isize {
        let mut tmp: isize = 0;
        self.t = unsafe {
            (self.im.hmget_key_ts)(
                self.t,
                ELEM,
                k as *mut c_void,
                KEYSIZE,
                &raw mut tmp,
                HM_STRING,
            )
        };
        tmp
    }

    /// `stbds_shdel(t, k)`
    fn del(&mut self, k: *const c_char) -> isize {
        self.t = unsafe {
            (self.im.hmdel_key)(self.t, ELEM, k as *mut c_void, KEYSIZE, 0, HM_STRING)
        };
        if self.t.is_null() { 0 } else { self.temp() }
    }

    /// `stbds_shlen(t)`
    fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            unsafe { header(self.raw()).length as isize - 1 }
        }
    }

    fn snap(&self) -> Option<StrMapSnap> {
        if self.t.is_null() {
            None
        } else {
            Some(unsafe { str_map_snap(self.t) })
        }
    }

    fn free(&mut self) {
        if !self.t.is_null() {
            unsafe { (self.im.hmfree_func)(self.raw(), ELEM) };
            self.t = std::ptr::null_mut();
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Put(usize, c_int),
    Puts(usize, c_int),
    Geti(usize),
    GetiTs(usize),
    Del(usize),
}

/// How the map is created.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Default,
    Strdup,
    Arena,
}

fn make<'a>(im: &'a Impl, kind: Kind) -> SMap<'a> {
    match kind {
        Kind::Default => SMap::new_default(im),
        Kind::Strdup => SMap::new_mode(im, SH_STRDUP),
        Kind::Arena => SMap::new_mode(im, SH_ARENA),
    }
}

fn run(label: &str, kind: Kind, keys: &[CString], ops: &[Op], seed: usize) {
    let _g = global_lock();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(seed);
        (p.rs.rand_seed)(seed);
    }
    let mut cm = make(&p.c, kind);
    let mut rm = make(&p.rs, kind);

    // Freshly created maps must already agree.
    match (cm.snap(), rm.snap()) {
        (None, None) => {}
        (Some(a), Some(b)) => {
            if let Some(d) = diff_str_map(&a, &b) {
                panic!("{label}/{kind:?}: fresh map: {d}");
            }
        }
        _ => panic!("{label}/{kind:?}: fresh map nullness mismatch"),
    }

    for (i, op) in ops.iter().enumerate() {
        let key = |n: usize| keys[n % keys.len()].as_ptr();
        let (cv, rv) = match *op {
            Op::Put(n, v) => {
                let (ci, cnew) = cm.put(key(n), v);
                let (ri, rnew) = rm.put(key(n), v);
                assert_eq!(
                    cnew, rnew,
                    "{label}/{kind:?} op {i} ({op:?}): insertion flag mismatch"
                );
                if cnew {
                    // The insert path is the only one guaranteed to have
                    // written temp_key, so this is where it can be compared.
                    assert_eq!(
                        cm.temp_key_str(),
                        rm.temp_key_str(),
                        "{label}/{kind:?} op {i} ({op:?}): temp_key mismatch"
                    );
                }
                (ci, ri)
            }
            Op::Puts(n, v) => {
                let ci = cm.puts(key(n), v);
                let ri = rm.puts(key(n), v);
                assert_eq!(
                    cm.temp_key_str(),
                    rm.temp_key_str(),
                    "{label}/{kind:?} op {i} ({op:?}): temp_key mismatch"
                );
                (ci, ri)
            }
            Op::Geti(n) => (cm.geti(key(n)), rm.geti(key(n))),
            Op::GetiTs(n) => (cm.geti_ts(key(n)), rm.geti_ts(key(n))),
            Op::Del(n) => (cm.del(key(n)), rm.del(key(n))),
        };
        assert_eq!(
            cv, rv,
            "{label}/{kind:?} op {i} ({op:?}) key={:?}: return mismatch",
            keys[match *op {
                Op::Put(n, _) | Op::Puts(n, _) | Op::Geti(n) | Op::GetiTs(n) | Op::Del(n) => n,
            } % keys.len()]
        );
        assert_eq!(
            cm.len(),
            rm.len(),
            "{label}/{kind:?} op {i} ({op:?}): shlen mismatch"
        );
        match (cm.snap(), rm.snap()) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                if let Some(d) = diff_str_map(&a, &b) {
                    panic!("{label}/{kind:?} op {i} ({op:?}): {d}");
                }
            }
            (a, b) => panic!(
                "{label}/{kind:?} op {i} ({op:?}): nullness mismatch C={} Rust={}",
                a.is_some(),
                b.is_some()
            ),
        }
    }
    cm.free();
    rm.free();
}

fn all_kinds(label: &str, keys: &[CString], ops: &[Op]) {
    for kind in [Kind::Default, Kind::Strdup, Kind::Arena] {
        run(label, kind, keys, ops, 0x31415926);
    }
}

fn make_keys(n: usize) -> Vec<CString> {
    (0..n)
        .map(|i| CString::new(format!("test_{i}")).unwrap())
        .collect()
}

#[test]
fn shmode_func_fresh_maps_match() {
    let _g = global_lock();
    let p = pair();
    for mode in [SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA] {
        unsafe {
            (p.c.rand_seed)(0x31415926);
            (p.rs.rand_seed)(0x31415926);
        }
        let cm = SMap::new_mode(&p.c, mode);
        let rm = SMap::new_mode(&p.rs, mode);
        let (a, b) = (cm.snap().unwrap(), rm.snap().unwrap());
        if let Some(d) = diff_str_map(&a, &b) {
            panic!("shmode_func(mode={mode}): {d}");
        }
        let mut cm = cm;
        let mut rm = rm;
        cm.free();
        rm.free();
    }
}

#[test]
fn empty_string_map_lookups() {
    let keys = make_keys(4);
    all_kinds(
        "empty",
        &keys,
        &[Op::Geti(0), Op::GetiTs(1), Op::Del(2), Op::Geti(0)],
    );
}

#[test]
fn small_string_sequences() {
    let keys = make_keys(16);
    let mut ops = Vec::new();
    for i in 0..16 {
        ops.push(Op::Put(i, i as c_int * 7));
        ops.push(Op::Geti(i));
        ops.push(Op::GetiTs(i));
    }
    all_kinds("small", &keys, &ops);
}

#[test]
fn shputs_restores_key_from_temp_key() {
    let keys = make_keys(40);
    let mut ops = Vec::new();
    // Each key is inserted exactly once, which is the only case where the
    // `shputs` macro's `stbds_temp_key` read is well defined.
    for i in 0..40 {
        ops.push(Op::Puts(i, 1000 + i as c_int));
        ops.push(Op::Geti(i));
    }
    // Re-inserting after a delete goes through the insert path again.
    for i in 0..40 {
        ops.push(Op::Del(i));
        ops.push(Op::Puts(i, 2000 + i as c_int));
    }
    all_kinds("shputs", &keys, &ops);
}

#[test]
fn string_map_growth() {
    let keys = make_keys(500);
    let mut ops = Vec::new();
    for i in 0..500 {
        ops.push(Op::Put(i, i as c_int));
    }
    for i in 0..500 {
        ops.push(Op::Geti(i));
    }
    all_kinds("growth", &keys, &ops);
}

#[test]
fn string_map_deletion() {
    let keys = make_keys(300);
    let mut ops = Vec::new();
    for i in 0..300 {
        ops.push(Op::Put(i, i as c_int));
    }
    for i in (0..300).step_by(2) {
        ops.push(Op::Del(i));
        ops.push(Op::Geti(i));
        ops.push(Op::Geti(i + 1));
    }
    for i in (1..300).step_by(2) {
        ops.push(Op::Del(i));
    }
    for i in 0..50 {
        ops.push(Op::Put(i, -(i as c_int)));
    }
    all_kinds("deletion", &keys, &ops);
}

/// Keys chosen to stress `stbds_hash_string`: shared prefixes, very long keys,
/// the empty key, and high-bit bytes.
#[test]
fn tricky_keys() {
    let mut keys: Vec<CString> = Vec::new();
    keys.push(CString::new("").unwrap());
    keys.push(CString::new("a").unwrap());
    keys.push(CString::new("A").unwrap());
    keys.push(CString::new("aa").unwrap());
    for i in 0..20 {
        keys.push(CString::new(format!("prefix_{}", "x".repeat(i))).unwrap());
    }
    keys.push(CString::new(vec![0x80u8; 40]).unwrap());
    keys.push(CString::new(vec![0xffu8; 40]).unwrap());
    keys.push(CString::new("z".repeat(2000)).unwrap());
    keys.push(CString::new("z".repeat(2001)).unwrap());

    let n = keys.len();
    let mut ops = Vec::new();
    for i in 0..n {
        ops.push(Op::Put(i, i as c_int));
    }
    for i in 0..n {
        ops.push(Op::Geti(i));
        ops.push(Op::GetiTs(i));
    }
    for i in 0..n {
        ops.push(Op::Del(i));
        ops.push(Op::Geti(i));
    }
    all_kinds("tricky", &keys, &ops);
}

/// `SH_ARENA` keys must land in the index's own string arena — this drives it
/// past the 512-byte first block and into the oversized-string path.
#[test]
fn arena_mode_long_keys() {
    let mut keys: Vec<CString> = Vec::new();
    for i in 0..60 {
        keys.push(CString::new(format!("{:0>width$}", i, width = 30 + i * 25)).unwrap());
    }
    keys.push(CString::new("Q".repeat(5000)).unwrap());
    keys.push(CString::new("R".repeat(70000)).unwrap());
    let n = keys.len();
    let mut ops = Vec::new();
    for i in 0..n {
        ops.push(Op::Put(i, i as c_int));
    }
    for i in 0..n {
        ops.push(Op::Geti(i));
    }
    run("arena-long", Kind::Arena, &keys, &ops, 0x31415926);
    run("strdup-long", Kind::Strdup, &keys, &ops, 0x31415926);
    run("default-long", Kind::Default, &keys, &ops, 0x31415926);
}

#[test]
fn randomized_string_operations() {
    let keys = make_keys(200);
    let mut state: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    let mut ops = Vec::new();
    for _ in 0..2500 {
        let r = next();
        let k = (r % 200) as usize;
        match (r / 200) % 7 {
            0 | 1 | 2 => ops.push(Op::Put(k, (r % 5000) as c_int)),
            3 | 4 => ops.push(Op::Geti(k)),
            5 => ops.push(Op::GetiTs(k)),
            _ => ops.push(Op::Del(k)),
        }
    }
    all_kinds("random", &keys, &ops);
}

#[test]
fn string_maps_under_varied_seeds() {
    let keys = make_keys(120);
    let mut ops = Vec::new();
    for i in 0..120 {
        ops.push(Op::Put(i, i as c_int));
    }
    for i in (0..120).step_by(3) {
        ops.push(Op::Del(i));
    }
    for i in 0..120 {
        ops.push(Op::Geti(i));
    }
    for seed in [0usize, 1, 0xffff_ffff_ffff_ffff, 0x5555_5555, 1 << 55] {
        for kind in [Kind::Default, Kind::Strdup, Kind::Arena] {
            run(&format!("seed-{seed:#x}"), kind, &keys, &ops, seed);
        }
    }
}
