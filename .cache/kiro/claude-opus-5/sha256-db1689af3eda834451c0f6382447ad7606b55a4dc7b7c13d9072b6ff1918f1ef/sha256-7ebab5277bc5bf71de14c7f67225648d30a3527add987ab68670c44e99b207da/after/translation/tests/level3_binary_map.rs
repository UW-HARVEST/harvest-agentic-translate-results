//! Level 3: the binary-keyed hash map —
//! `stbds_hmput_default`, `stbds_hmput_key`, `stbds_hmget_key`,
//! `stbds_hmget_key_ts`, `stbds_hmdel_key`, `stbds_hmfree_func`.
//!
//! Every operation mirrors what the `stbds_hmput` / `stbds_hmgeti` /
//! `stbds_hmdel` / `stbds_hmdefault` macros in `lib.c` expand to.
mod harness;

use harness::*;
use std::ffi::c_void;
use std::os::raw::c_int;

const ELEM: usize = std::mem::size_of::<IntEntry>(); // 8
const KEYSIZE: usize = std::mem::size_of::<c_int>(); // 4

/// One map under test, driven purely through exported symbols.
struct Map<'a> {
    im: &'a Impl,
    /// the "hash pointer" the macros keep in the user's variable (`t`)
    t: *mut c_void,
}

impl<'a> Map<'a> {
    fn new(im: &'a Impl) -> Self {
        Map {
            im,
            t: std::ptr::null_mut(),
        }
    }

    fn raw(&self) -> *mut c_void {
        unsafe { (self.t as *mut u8).sub(ELEM) as *mut c_void }
    }

    fn temp(&self) -> isize {
        unsafe { header(self.raw()).temp }
    }

    /// `stbds_hmput(t, k, v)`
    fn put(&mut self, k: c_int, v: c_int) -> isize {
        let mut key = k;
        self.t = unsafe {
            (self.im.hmput_key)(
                self.t,
                ELEM,
                (&raw mut key) as *mut c_void,
                KEYSIZE,
                HM_BINARY,
            )
        };
        let idx = self.temp();
        unsafe {
            let e = (self.t as *mut IntEntry).offset(idx);
            (*e).key = k;
            (*e).value = v;
        }
        idx
    }

    /// `stbds_hmgeti(t, k)`
    fn geti(&mut self, k: c_int) -> isize {
        let mut key = k;
        self.t = unsafe {
            (self.im.hmget_key)(
                self.t,
                ELEM,
                (&raw mut key) as *mut c_void,
                KEYSIZE,
                HM_BINARY,
            )
        };
        self.temp()
    }

    /// `stbds_hmgeti_ts(t, k, temp)`
    fn geti_ts(&mut self, k: c_int) -> isize {
        let mut key = k;
        let mut tmp: isize = 0;
        self.t = unsafe {
            (self.im.hmget_key_ts)(
                self.t,
                ELEM,
                (&raw mut key) as *mut c_void,
                KEYSIZE,
                &raw mut tmp,
                HM_BINARY,
            )
        };
        tmp
    }

    /// `stbds_hmdel(t, k)`
    fn del(&mut self, k: c_int) -> isize {
        let mut key = k;
        self.t = unsafe {
            (self.im.hmdel_key)(
                self.t,
                ELEM,
                (&raw mut key) as *mut c_void,
                KEYSIZE,
                0, // STBDS_OFFSETOF(t, key) == 0
                HM_BINARY,
            )
        };
        if self.t.is_null() { 0 } else { self.temp() }
    }

    /// `stbds_hmdefault(t, v)`
    fn set_default(&mut self, v: c_int) {
        self.t = unsafe { (self.im.hmput_default)(self.t, ELEM) };
        unsafe {
            (*(self.t as *mut IntEntry).offset(-1)).value = v;
        }
    }

    /// `stbds_hmlen(t)`
    fn len(&self) -> isize {
        if self.t.is_null() {
            0
        } else {
            unsafe { header(self.raw()).length as isize - 1 }
        }
    }

    fn snap(&self) -> Option<IntMapSnap> {
        if self.t.is_null() {
            None
        } else {
            Some(unsafe { int_map_snap(self.t) })
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
    Put(c_int, c_int),
    Geti(c_int),
    GetiTs(c_int),
    Del(c_int),
    Default(c_int),
}

fn run_ops(label: &str, ops: &[Op]) {
    let _g = global_lock();
    let p = pair();
    unsafe {
        (p.c.rand_seed)(0x31415926);
        (p.rs.rand_seed)(0x31415926);
    }
    let mut cm = Map::new(&p.c);
    let mut rm = Map::new(&p.rs);

    for (i, op) in ops.iter().enumerate() {
        let (cv, rv) = match *op {
            Op::Put(k, v) => (cm.put(k, v), rm.put(k, v)),
            Op::Geti(k) => (cm.geti(k), rm.geti(k)),
            Op::GetiTs(k) => (cm.geti_ts(k), rm.geti_ts(k)),
            Op::Del(k) => (cm.del(k), rm.del(k)),
            Op::Default(v) => {
                cm.set_default(v);
                rm.set_default(v);
                (0, 0)
            }
        };
        assert_eq!(cv, rv, "{label} op {i} ({op:?}): return value mismatch");
        assert_eq!(cm.len(), rm.len(), "{label} op {i} ({op:?}): hmlen mismatch");
        match (cm.snap(), rm.snap()) {
            (None, None) => {}
            (Some(a), Some(b)) => {
                if let Some(d) = diff_int_map(&a, &b) {
                    panic!("{label} op {i} ({op:?}): {d}");
                }
            }
            (a, b) => panic!(
                "{label} op {i} ({op:?}): nullness mismatch C={} Rust={}",
                a.is_some(),
                b.is_some()
            ),
        }
    }
    cm.free();
    rm.free();
}

#[test]
fn hmput_default_on_null_and_existing() {
    run_ops(
        "hmdefault",
        &[
            Op::Default(-7),
            Op::Default(11),
            Op::Geti(1),
            Op::Put(1, 100),
            Op::Default(42),
            Op::Geti(1),
            Op::Geti(2),
        ],
    );
}

#[test]
fn get_from_empty_map() {
    // hmgeti on a NULL map must allocate the sentinel and report -1.
    run_ops("empty-get", &[Op::Geti(5), Op::Geti(5), Op::GetiTs(5)]);
    run_ops("empty-get-ts", &[Op::GetiTs(9), Op::Geti(9)]);
    // hmdel on a NULL map returns NULL
    run_ops("empty-del", &[Op::Del(3)]);
    run_ops("empty-del-then-get", &[Op::Del(3), Op::Geti(3)]);
}

#[test]
fn small_put_get_sequences() {
    let mut ops = Vec::new();
    for i in 0..12 {
        ops.push(Op::Put(i, i * 10));
        ops.push(Op::Geti(i));
        ops.push(Op::Geti(i + 100));
    }
    run_ops("small", &ops);
}

/// Grows the table repeatedly (used_count_threshold) and reads everything back.
#[test]
fn growth_and_rehash() {
    let mut ops = Vec::new();
    for i in 0..600 {
        ops.push(Op::Put(i, i ^ 0x5a5a));
    }
    for i in 0..600 {
        ops.push(Op::Geti(i));
    }
    for i in 600..640 {
        ops.push(Op::Geti(i));
    }
    // overwrite existing keys — must reuse slots, not grow
    for i in 0..600 {
        ops.push(Op::Put(i, i + 1));
    }
    run_ops("growth", &ops);
}

/// Deletions: tombstone accumulation, the tombstone rebuild threshold, the
/// swap-with-last-element move and the shrink threshold.
#[test]
fn deletion_paths() {
    let mut ops = Vec::new();
    for i in 0..400 {
        ops.push(Op::Put(i, i * 3));
    }
    // delete every other key -> tombstones, rebuilds, then a shrink
    for i in (0..400).step_by(2) {
        ops.push(Op::Del(i));
        ops.push(Op::Geti(i));
        ops.push(Op::Geti(i + 1));
    }
    // delete the rest, walking down to an (almost) empty table
    for i in (1..400).step_by(2) {
        ops.push(Op::Del(i));
    }
    // deleting absent keys
    for i in 0..20 {
        ops.push(Op::Del(i));
    }
    // reinsert after everything was removed
    for i in 0..60 {
        ops.push(Op::Put(i + 1000, i));
    }
    for i in 0..60 {
        ops.push(Op::Geti(i + 1000));
    }
    run_ops("deletion", &ops);
}

/// Delete-the-last-element case (`old_index == final_index`) plus deleting the
/// element that was just moved.
#[test]
fn delete_last_element_paths() {
    run_ops(
        "del-last",
        &[
            Op::Put(1, 1),
            Op::Del(1),
            Op::Put(2, 2),
            Op::Put(3, 3),
            Op::Del(3),
            Op::Del(2),
            Op::Put(4, 4),
            Op::Put(5, 5),
            Op::Put(6, 6),
            Op::Del(4),
            Op::Geti(6),
            Op::Geti(5),
            Op::Del(6),
            Op::Del(5),
        ],
    );
}

/// A long deterministic pseudo-random mix of every operation.
#[test]
fn randomized_mixed_operations() {
    let mut state: u64 = 0xc0ffee_1234_5678;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 33) as u32
    };
    let mut ops = Vec::new();
    for _ in 0..4000 {
        let r = next();
        let k = (r % 250) as c_int;
        match (r / 250) % 8 {
            0 | 1 | 2 => ops.push(Op::Put(k, (r % 10_000) as c_int)),
            3 | 4 => ops.push(Op::Geti(k)),
            5 => ops.push(Op::GetiTs(k)),
            6 => ops.push(Op::Del(k)),
            _ => ops.push(Op::Default((r % 97) as c_int)),
        }
    }
    run_ops("random", &ops);
}

/// Negative and extreme key values (exercises the 4-byte memcmp key compare).
#[test]
fn extreme_keys() {
    let keys: Vec<c_int> = vec![
        0,
        -1,
        1,
        i32::MIN,
        i32::MAX,
        -2,
        0x7f7f_7f7f,
        -0x7f7f_7f7f,
        0x0080_0000,
        -128,
        255,
        256,
        65535,
        65536,
    ];
    let mut ops = Vec::new();
    for (i, &k) in keys.iter().enumerate() {
        ops.push(Op::Put(k, i as c_int));
    }
    for &k in &keys {
        ops.push(Op::Geti(k));
        ops.push(Op::GetiTs(k));
    }
    for &k in &keys {
        ops.push(Op::Del(k));
        ops.push(Op::Geti(k));
    }
    run_ops("extreme", &ops);
}

/// `stbds_rand_seed` changes the table seed, which changes every probe
/// position — the two implementations must still agree slot for slot.
#[test]
fn varied_seeds() {
    let _g = global_lock();
    let p = pair();
    for seed in [
        0usize,
        1,
        2,
        0x31415926,
        0xffff_ffff_ffff_ffff,
        0xaaaa_aaaa_aaaa_aaaa,
        1 << 40,
    ] {
        unsafe {
            (p.c.rand_seed)(seed);
            (p.rs.rand_seed)(seed);
        }
        let mut cm = Map::new(&p.c);
        let mut rm = Map::new(&p.rs);
        for i in 0..150 {
            assert_eq!(cm.put(i, i * 2), rm.put(i, i * 2), "seed {seed:#x} put {i}");
        }
        for i in (0..150).step_by(3) {
            assert_eq!(cm.del(i), rm.del(i), "seed {seed:#x} del {i}");
        }
        for i in 0..150 {
            assert_eq!(cm.geti(i), rm.geti(i), "seed {seed:#x} get {i}");
        }
        let (a, b) = (cm.snap().unwrap(), rm.snap().unwrap());
        if let Some(d) = diff_int_map(&a, &b) {
            panic!("seed {seed:#x}: {d}");
        }
        cm.free();
        rm.free();
    }
}
