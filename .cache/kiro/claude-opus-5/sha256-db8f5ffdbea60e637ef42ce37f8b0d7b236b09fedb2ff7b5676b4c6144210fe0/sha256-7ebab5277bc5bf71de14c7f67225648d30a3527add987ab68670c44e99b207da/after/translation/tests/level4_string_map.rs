//! Level 4: string-keyed hash maps.
//!
//! Covers all three key-ownership modes reachable from stb_ds:
//!   * `STBDS_SH_DEFAULT` - the caller's pointer is stored (a plain `shput`
//!     against a `NULL` map).
//!   * `STBDS_SH_STRDUP`  - `sh_new_strdup`, keys are `stbds_strdup`ed.
//!   * `STBDS_SH_ARENA`   - `sh_new_arena`, keys go through `stbds_stralloc`.
//!
//! Key *pointers* differ between the two libraries in the strdup/arena modes,
//! so keys are compared by contents.

mod harness;

use harness::*;
use std::ffi::{c_char, c_void};

const ELEMSIZE: usize = 16; // struct { char *key; int value; }
const VALUE_OFFSET: usize = 8;

/// Owns the NUL-terminated key buffers; in `SH_DEFAULT` mode the library stores
/// these pointers directly, so they must outlive the map.
struct Keys {
    bufs: Vec<Vec<u8>>,
}

impl Keys {
    fn new(strings: &[String]) -> Keys {
        Keys {
            bufs: strings.iter().map(|s| cstring(s)).collect(),
        }
    }
    fn ptr(&self, i: usize) -> *mut c_char {
        self.bufs[i].as_ptr() as *mut c_char
    }
}

fn assert_same(ct: *mut c_void, rt: *mut c_void, temp_key: bool, ctx: &str) {
    let (a, b) = unsafe {
        (
            snapshot_string(ct, ELEMSIZE, VALUE_OFFSET, temp_key),
            snapshot_string(rt, ELEMSIZE, VALUE_OFFSET, temp_key),
        )
    };
    if a != b {
        // print the interesting parts only; bucket arrays are large
        panic!(
            "{} diverged\n  C: len={} cap={} temp={} slots={} used={} tomb={} mode={} keys={:?} values={:?} temp_key={:?}\n RS: len={} cap={} temp={} slots={} used={} tomb={} mode={} keys={:?} values={:?} temp_key={:?}\n buckets equal: {}",
            ctx,
            a.length, a.capacity, a.temp, a.slot_count, a.used_count, a.tombstone_count, a.string_mode,
            a.keys.as_ref().map(|k| k.iter().map(|x| String::from_utf8_lossy(x).to_string()).collect::<Vec<_>>()),
            a.values, a.temp_key.as_ref().map(|x| String::from_utf8_lossy(x).to_string()),
            b.length, b.capacity, b.temp, b.slot_count, b.used_count, b.tombstone_count, b.string_mode,
            b.keys.as_ref().map(|k| k.iter().map(|x| String::from_utf8_lossy(x).to_string()).collect::<Vec<_>>()),
            b.values, b.temp_key.as_ref().map(|x| String::from_utf8_lossy(x).to_string()),
            a.buckets == b.buckets,
        );
    }
}

fn make_keys(n: usize) -> Keys {
    // a mix of lengths, shared prefixes and pure-ASCII high bytes
    let mut v = Vec::new();
    for i in 0..n {
        match i % 4 {
            0 => v.push(format!("test_{}", i)),
            1 => v.push(format!("key-with-a-longer-name-{:05}", i)),
            2 => v.push(format!("{}", i)),
            _ => v.push(format!("prefix/{}/suffix", i)),
        }
    }
    Keys::new(&v)
}

/// Creates an empty map in the requested mode on both libraries.
/// `SH_DEFAULT` is reached implicitly by putting into a `NULL` map.
unsafe fn new_maps(p: &Pair, mode: i32, seed: usize) -> (*mut c_void, *mut c_void) {
    p.c.rand_seed(seed);
    p.rs.rand_seed(seed);
    if mode == SH_DEFAULT {
        (std::ptr::null_mut(), std::ptr::null_mut())
    } else {
        (
            p.c.shmode_func(ELEMSIZE, mode),
            p.rs.shmode_func(ELEMSIZE, mode),
        )
    }
}

const MODES: &[(i32, &str)] = &[
    (SH_DEFAULT, "SH_DEFAULT"),
    (SH_STRDUP, "SH_STRDUP"),
    (SH_ARENA, "SH_ARENA"),
];

// ---------------------------------------------------------------------------
// shput
// ---------------------------------------------------------------------------

#[test]
fn shput_inserts_match() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(250);

    for &(mode, name) in MODES {
        unsafe {
            let (mut ct, mut rt) = new_maps(p, mode, 0xA1B2_C3D4);
            for i in 0..keys.bufs.len() {
                ct = shput(&p.c, ct, ELEMSIZE, keys.ptr(i), i as i32 * 3, VALUE_OFFSET);
                rt = shput(&p.rs, rt, ELEMSIZE, keys.ptr(i), i as i32 * 3, VALUE_OFFSET);
                assert_same(ct, rt, true, &format!("{} shput #{}", name, i));
            }
            // lookups
            for i in 0..keys.bufs.len() {
                let (c2, ci) = shgeti(&p.c, ct, ELEMSIZE, keys.ptr(i));
                let (r2, ri) = shgeti(&p.rs, rt, ELEMSIZE, keys.ptr(i));
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "{} shgeti({})", name, i);
                assert!(ci >= 0, "{} key {} missing", name, i);
            }
            // misses
            let missing = Keys::new(&["nope".into(), "".into(), "test_999999".into()]);
            for i in 0..missing.bufs.len() {
                let (c2, ci) = shgeti(&p.c, ct, ELEMSIZE, missing.ptr(i));
                let (r2, ri) = shgeti(&p.rs, rt, ELEMSIZE, missing.ptr(i));
                ct = c2;
                rt = r2;
                assert_eq!(ci, ri, "{} shgeti(miss {})", name, i);
                assert_eq!(ci, -1);
            }
            assert_same(ct, rt, false, &format!("{} after lookups", name));
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

#[test]
fn shput_overwrites_match() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(30);

    for &(mode, name) in MODES {
        unsafe {
            let (mut ct, mut rt) = new_maps(p, mode, 0x0BAD_F00D);
            let mut rng = Rng::new(0x1122_3344);
            for round in 0..500 {
                let i = rng.below(keys.bufs.len() as u64) as usize;
                ct = shput(&p.c, ct, ELEMSIZE, keys.ptr(i), round as i32, VALUE_OFFSET);
                rt = shput(&p.rs, rt, ELEMSIZE, keys.ptr(i), round as i32, VALUE_OFFSET);
                // temp_key is not compared here: when a *re*-insert follows a
                // table grow, the C code can leave temp_key unwritten.
                assert_same(ct, rt, false, &format!("{} overwrite round {}", name, round));
            }
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

// ---------------------------------------------------------------------------
// shputs: writes the struct then restores the key from stbds_temp_key
// ---------------------------------------------------------------------------

#[test]
fn shputs_distinct_keys_match() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(200);

    for &(mode, name) in MODES {
        unsafe {
            let (mut ct, mut rt) = new_maps(p, mode, 0x5150_5150);
            for i in 0..keys.bufs.len() {
                ct = shputs(&p.c, ct, ELEMSIZE, keys.ptr(i), 1000 - i as i32, VALUE_OFFSET);
                rt = shputs(&p.rs, rt, ELEMSIZE, keys.ptr(i), 1000 - i as i32, VALUE_OFFSET);
                assert_same(ct, rt, true, &format!("{} shputs #{}", name, i));
            }
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

// ---------------------------------------------------------------------------
// shdel
// ---------------------------------------------------------------------------

#[test]
fn shdel_forward_matches() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(200);

    for &(mode, name) in MODES {
        unsafe {
            let (mut ct, mut rt) = new_maps(p, mode, 0xFEED_BEEF);
            for i in 0..keys.bufs.len() {
                ct = shput(&p.c, ct, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
                rt = shput(&p.rs, rt, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
            }
            assert_same(ct, rt, true, &format!("{} before shdel", name));
            for i in 0..keys.bufs.len() {
                let (c2, cr) = shdel(&p.c, ct, ELEMSIZE, keys.ptr(i));
                let (r2, rr) = shdel(&p.rs, rt, ELEMSIZE, keys.ptr(i));
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "{} shdel({}) result", name, i);
                assert_same(ct, rt, false, &format!("{} shdel #{}", name, i));
            }
            assert_eq!(hmlen(ct, ELEMSIZE), 0);
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

#[test]
fn shdel_reverse_and_missing_matches() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(120);
    let missing = Keys::new(&["absent-1".into(), "absent-2".into(), "".into()]);

    for &(mode, name) in MODES {
        unsafe {
            let (mut ct, mut rt) = new_maps(p, mode, 0x1357_9BDF);
            for i in 0..keys.bufs.len() {
                ct = shput(&p.c, ct, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
                rt = shput(&p.rs, rt, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
            }
            for i in 0..missing.bufs.len() {
                let (c2, cr) = shdel(&p.c, ct, ELEMSIZE, missing.ptr(i));
                let (r2, rr) = shdel(&p.rs, rt, ELEMSIZE, missing.ptr(i));
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "{} shdel(miss)", name);
                assert_same(ct, rt, false, &format!("{} shdel miss {}", name, i));
            }
            for i in (0..keys.bufs.len()).rev() {
                let (c2, cr) = shdel(&p.c, ct, ELEMSIZE, keys.ptr(i));
                let (r2, rr) = shdel(&p.rs, rt, ELEMSIZE, keys.ptr(i));
                ct = c2;
                rt = r2;
                assert_eq!(cr, rr, "{} rev shdel({})", name, i);
                assert_same(ct, rt, false, &format!("{} rev shdel #{}", name, i));
            }
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

// ---------------------------------------------------------------------------
// randomised mixed workload
// ---------------------------------------------------------------------------

#[test]
fn randomised_string_workload_matches() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(150);

    for &(mode, name) in MODES {
        for seed in [11u64, 22] {
            unsafe {
                let (mut ct, mut rt) = new_maps(p, mode, 0x2000_0000 + seed as usize);
                let mut rng = Rng::new(seed);
                for op in 0..1500 {
                    let i = rng.below(keys.bufs.len() as u64) as usize;
                    let k = keys.ptr(i);
                    match rng.below(10) {
                        0..=4 => {
                            ct = shput(&p.c, ct, ELEMSIZE, k, op as i32, VALUE_OFFSET);
                            rt = shput(&p.rs, rt, ELEMSIZE, k, op as i32, VALUE_OFFSET);
                        }
                        5..=7 => {
                            let (c2, ci) = shgeti(&p.c, ct, ELEMSIZE, k);
                            let (r2, ri) = shgeti(&p.rs, rt, ELEMSIZE, k);
                            ct = c2;
                            rt = r2;
                            assert_eq!(ci, ri, "{} seed {} op {} shgeti", name, seed, op);
                        }
                        _ => {
                            let (c2, cr) = shdel(&p.c, ct, ELEMSIZE, k);
                            let (r2, rr) = shdel(&p.rs, rt, ELEMSIZE, k);
                            ct = c2;
                            rt = r2;
                            assert_eq!(cr, rr, "{} seed {} op {} shdel", name, seed, op);
                        }
                    }
                    assert_same(ct, rt, false, &format!("{} seed {} op {}", name, seed, op));
                }
                hmfree(&p.c, ct, ELEMSIZE);
                hmfree(&p.rs, rt, ELEMSIZE);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// hmfree_func: the STRDUP mode frees every stored key, the ARENA mode resets
// the arena. Run a churn first so there is something to release.
// ---------------------------------------------------------------------------

#[test]
fn hmfree_func_releases_keys_for_every_mode() {
    let _g = shared_lock();
    let p = pair();
    let keys = make_keys(400);

    for &(mode, name) in MODES {
        unsafe {
            let (mut ct, mut rt) = new_maps(p, mode, 0x9999_0000);
            for i in 0..keys.bufs.len() {
                ct = shput(&p.c, ct, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
                rt = shput(&p.rs, rt, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
            }
            // delete a third of them so the strdup free loop sees a hole-free
            // but shortened array
            for i in (0..keys.bufs.len()).step_by(3) {
                let (c2, _) = shdel(&p.c, ct, ELEMSIZE, keys.ptr(i));
                let (r2, _) = shdel(&p.rs, rt, ELEMSIZE, keys.ptr(i));
                ct = c2;
                rt = r2;
            }
            assert_same(ct, rt, false, &format!("{} before hmfree", name));
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

/// `stbds_hmfree_func` on a table-less array and on a fresh `sh_new_*` map.
#[test]
fn hmfree_func_edge_cases() {
    let _g = shared_lock();
    let p = pair();
    unsafe {
        // NULL is a no-op
        p.c.hmfree_func(std::ptr::null_mut(), ELEMSIZE);
        p.rs.hmfree_func(std::ptr::null_mut(), ELEMSIZE);

        // array with no hash table (hmput_default only)
        let ct = p.c.hmput_default(std::ptr::null_mut(), ELEMSIZE);
        let rt = p.rs.hmput_default(std::ptr::null_mut(), ELEMSIZE);
        assert_same(ct, rt, false, "hmput_default before free");
        hmfree(&p.c, ct, ELEMSIZE);
        hmfree(&p.rs, rt, ELEMSIZE);

        // freshly created maps in every mode
        for &(mode, name) in MODES {
            if mode == SH_DEFAULT {
                continue;
            }
            let (ct, rt) = new_maps(p, mode, 0x4242);
            assert_same(ct, rt, false, &format!("fresh {}", name));
            hmfree(&p.c, ct, ELEMSIZE);
            hmfree(&p.rs, rt, ELEMSIZE);
        }
    }
}

/// Long keys drive the arena's oversized-block path from inside `hmput_key`.
#[test]
fn arena_mode_with_long_keys_matches() {
    let _g = shared_lock();
    let p = pair();
    let mut strings = Vec::new();
    for i in 0..40 {
        strings.push("x".repeat(400 + i * 40) + &format!("-{}", i));
    }
    let keys = Keys::new(&strings);
    unsafe {
        let (mut ct, mut rt) = new_maps(p, SH_ARENA, 0xABCD_0000);
        for i in 0..keys.bufs.len() {
            ct = shput(&p.c, ct, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
            rt = shput(&p.rs, rt, ELEMSIZE, keys.ptr(i), i as i32, VALUE_OFFSET);
            assert_same(ct, rt, true, &format!("arena long key #{}", i));
        }
        hmfree(&p.c, ct, ELEMSIZE);
        hmfree(&p.rs, rt, ELEMSIZE);
    }
}
