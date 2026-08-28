//! Hash-map layer: `stbds_hmput_key`, `stbds_hmget_key`, `stbds_hmget_key_ts`,
//! `stbds_hmput_default`, `stbds_shmode_func`, `stbds_hmdel_key` and
//! `stbds_hmfree_func`.
//!
//! Everything lives in a single `#[test]` because the libraries carry a mutable
//! file-static hash seed; running the scenarios sequentially keeps the two
//! seed streams in lock-step.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Re-implementations of the stb_ds macros, driven purely through .so exports
// ---------------------------------------------------------------------------

unsafe fn temp_of(t: *mut c_void, elemsize: usize) -> isize {
    (*header_of(raw_of(t, elemsize))).temp
}

unsafe fn hmlen(t: *mut c_void, elemsize: usize) -> isize {
    if t.is_null() {
        0
    } else {
        (*header_of(raw_of(t, elemsize))).length as isize - 1
    }
}

/// `stbds_hmput(t, k, v)` for an arbitrary POD element layout.
unsafe fn bin_put(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
    rest: &[u8],
) -> *mut c_void {
    assert_eq!(key.len() + rest.len(), elemsize);
    let mut k = key.to_vec();
    let t = (api.hmput_key)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        HM_BINARY,
    );
    let temp = temp_of(t, elemsize);
    let e = (t as *mut u8).add(elemsize * temp as usize);
    ptr::copy_nonoverlapping(k.as_ptr(), e, key.len());
    ptr::copy_nonoverlapping(rest.as_ptr(), e.add(key.len()), rest.len());
    t
}

/// `stbds_hmgeti(t, k)`
unsafe fn bin_geti(api: &Api, t: *mut c_void, elemsize: usize, key: &[u8]) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    let t = (api.hmget_key)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        HM_BINARY,
    );
    (t, temp_of(t, elemsize))
}

/// `stbds_hmgeti_ts(t, k, temp)`
unsafe fn bin_geti_ts(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: &[u8],
) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    let mut temp: isize = 0x5555_5555;
    let t = (api.hmget_key_ts)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        &mut temp,
        HM_BINARY,
    );
    (t, temp)
}

/// `stbds_hmdel(t, k)`
unsafe fn bin_del(api: &Api, t: *mut c_void, elemsize: usize, key: &[u8]) -> (*mut c_void, isize) {
    let mut k = key.to_vec();
    let t2 = (api.hmdel_key)(
        t,
        elemsize,
        k.as_mut_ptr() as *mut c_void,
        key.len(),
        0,
        HM_BINARY,
    );
    let r = if t2.is_null() {
        0
    } else {
        temp_of(t2, elemsize)
    };
    (t2, r)
}

const KEYPTR_SIZE: usize = std::mem::size_of::<*mut c_char>();

/// `stbds_shput(t, k, v)` — element layout `{ char *key; int value; }`
unsafe fn sh_put(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    value: c_int,
) -> *mut c_void {
    let t = (api.hmput_key)(t, elemsize, key as *mut c_void, KEYPTR_SIZE, HM_STRING);
    let temp = temp_of(t, elemsize);
    let e = (t as *mut u8).add(elemsize * temp as usize);
    *(e.add(KEYPTR_SIZE) as *mut c_int) = value;
    t
}

/// `stbds_shputs(t, s)`
unsafe fn sh_puts(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
    value: c_int,
) -> *mut c_void {
    let t = (api.hmput_key)(t, elemsize, key as *mut c_void, KEYPTR_SIZE, HM_STRING);
    let raw = raw_of(t, elemsize);
    let temp = (*header_of(raw)).temp;
    let e = (t as *mut u8).add(elemsize * temp as usize);
    *(e as *mut *mut c_char) = key;
    *(e.add(KEYPTR_SIZE) as *mut c_int) = value;
    // `(t)[temp].key = stbds_temp_key((t)-1)`
    let table = (*header_of(raw)).hash_table as *mut HashIndex;
    *(e as *mut *mut c_char) = (*table).temp_key;
    t
}

/// `stbds_shgeti(t, k)`
unsafe fn sh_geti(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
) -> (*mut c_void, isize) {
    let t = (api.hmget_key)(t, elemsize, key as *mut c_void, KEYPTR_SIZE, HM_STRING);
    (t, temp_of(t, elemsize))
}

/// `stbds_shdel(t, k)`
unsafe fn sh_del(
    api: &Api,
    t: *mut c_void,
    elemsize: usize,
    key: *mut c_char,
) -> (*mut c_void, isize) {
    let t2 = (api.hmdel_key)(
        t,
        elemsize,
        key as *mut c_void,
        KEYPTR_SIZE,
        0,
        HM_STRING,
    );
    let r = if t2.is_null() {
        0
    } else {
        temp_of(t2, elemsize)
    };
    (t2, r)
}

// ---------------------------------------------------------------------------
// Comparison helper
// ---------------------------------------------------------------------------

struct Cmp {
    string_keys: bool,
    elemsize: usize,
}

impl Cmp {
    unsafe fn check(&self, ct: *mut c_void, rt: *mut c_void, what: &str) {
        assert_eq!(
            ct.is_null(),
            rt.is_null(),
            "{what}: nullness differs (C null={}, Rust null={})",
            ct.is_null(),
            rt.is_null()
        );
        if ct.is_null() {
            return;
        }
        let craw = raw_of(ct, self.elemsize);
        let rraw = raw_of(rt, self.elemsize);
        if fingerprint(craw, self.elemsize, self.string_keys)
            != fingerprint(rraw, self.elemsize, self.string_keys)
        {
            let cd = dump_map(craw, self.elemsize, self.string_keys);
            let rd = dump_map(rraw, self.elemsize, self.string_keys);
            panic!("{what}: state mismatch\n--- C ---\n{cd}\n--- Rust ---\n{rd}");
        }
    }
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

unsafe fn reseed(p: &Pair, seed: usize) {
    (p.c.rand_seed)(seed);
    (p.r.rand_seed)(seed);
}

/// Progress marker: the C library aborts on its own internal asserts, which
/// takes the whole test process down, so the current position is logged.
fn mark(what: &str) {
    if std::env::var_os("HM_TRACE").is_some() {
        eprintln!("[hashmap] {what}");
    }
}

/// Insert / lookup / delete on a binary-keyed map of a given layout.
unsafe fn scenario_binary(p: &Pair, elemsize: usize, keysize: usize, n: usize, seed: usize) {
    let label = format!("binary(elemsize={elemsize}, keysize={keysize}, n={n})");
    mark(&label);
    reseed(p, seed);
    let cmp = Cmp {
        string_keys: false,
        elemsize,
    };

    let mk = |i: usize| -> (Vec<u8>, Vec<u8>) {
        let mut key = vec![0u8; keysize];
        let x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for (b, s) in key.iter_mut().enumerate() {
            *s = ((x >> (8 * (b % 8))) & 0xff) as u8;
        }
        let mut rest = vec![0u8; elemsize - keysize];
        for (b, s) in rest.iter_mut().enumerate() {
            *s = (i as u8).wrapping_add(b as u8).wrapping_mul(3);
        }
        (key, rest)
    };

    let mut ct: *mut c_void = ptr::null_mut();
    let mut rt: *mut c_void = ptr::null_mut();

    // --- inserts ---------------------------------------------------------
    for i in 0..n {
        let (k, v) = mk(i);
        ct = bin_put(&p.c, ct, elemsize, &k, &v);
        rt = bin_put(&p.r, rt, elemsize, &k, &v);
        cmp.check(ct, rt, &format!("{label}: after put #{i}"));
    }
    assert_eq!(hmlen(ct, elemsize), n as isize, "{label}: C length");
    assert_eq!(hmlen(rt, elemsize), n as isize, "{label}: Rust length");

    // --- overwrite existing keys ----------------------------------------
    for i in (0..n).step_by(3) {
        let (k, _) = mk(i);
        let v: Vec<u8> = (0..elemsize - keysize).map(|b| (b as u8) ^ 0xA5).collect();
        ct = bin_put(&p.c, ct, elemsize, &k, &v);
        rt = bin_put(&p.r, rt, elemsize, &k, &v);
        cmp.check(ct, rt, &format!("{label}: after overwrite #{i}"));
    }

    // --- lookups (hits and misses) --------------------------------------
    for i in 0..n * 2 {
        let (k, _) = mk(i);
        let (c2, ci) = bin_geti(&p.c, ct, elemsize, &k);
        let (r2, ri) = bin_geti(&p.r, rt, elemsize, &k);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "{label}: hmgeti(#{i})");
        cmp.check(ct, rt, &format!("{label}: after hmgeti #{i}"));

        let (c3, cts) = bin_geti_ts(&p.c, ct, elemsize, &k);
        let (r3, rts) = bin_geti_ts(&p.r, rt, elemsize, &k);
        ct = c3;
        rt = r3;
        assert_eq!(cts, rts, "{label}: hmgeti_ts(#{i})");
    }

    // --- deletions (existing, then repeated / missing) -------------------
    for i in (0..n).step_by(2) {
        let (k, _) = mk(i);
        mark(&format!("{label}: hmdel #{i}"));
        let (c2, cr) = bin_del(&p.c, ct, elemsize, &k);
        let (r2, rr) = bin_del(&p.r, rt, elemsize, &k);
        ct = c2;
        rt = r2;
        assert_eq!(cr, rr, "{label}: hmdel(#{i})");
        cmp.check(ct, rt, &format!("{label}: after hmdel #{i}"));

        // deleting again must be a no-op with the same reported result
        let (c3, cr2) = bin_del(&p.c, ct, elemsize, &k);
        let (r3, rr2) = bin_del(&p.r, rt, elemsize, &k);
        ct = c3;
        rt = r3;
        assert_eq!(cr2, rr2, "{label}: repeated hmdel(#{i})");
        cmp.check(ct, rt, &format!("{label}: after repeated hmdel #{i}"));
    }

    // --- re-insert to reuse tombstones ----------------------------------
    for i in (0..n).step_by(2) {
        let (k, _) = mk(i);
        let v: Vec<u8> = (0..elemsize - keysize).map(|b| (b as u8) ^ 0x5A).collect();
        ct = bin_put(&p.c, ct, elemsize, &k, &v);
        rt = bin_put(&p.r, rt, elemsize, &k, &v);
        cmp.check(ct, rt, &format!("{label}: after tombstone reuse #{i}"));
    }

    // --- delete everything ----------------------------------------------
    for i in 0..n {
        let (k, _) = mk(i);
        mark(&format!("{label}: drain hmdel #{i}"));
        let (c2, cr) = bin_del(&p.c, ct, elemsize, &k);
        let (r2, rr) = bin_del(&p.r, rt, elemsize, &k);
        ct = c2;
        rt = r2;
        assert_eq!(cr, rr, "{label}: drain hmdel(#{i})");
        cmp.check(ct, rt, &format!("{label}: after drain hmdel #{i}"));
    }
    assert_eq!(hmlen(ct, elemsize), 0, "{label}: C drained length");
    assert_eq!(hmlen(rt, elemsize), 0, "{label}: Rust drained length");

    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
}

/// Alternating churn that drives the tombstone-rebuild and shrink branches.
unsafe fn scenario_binary_churn(p: &Pair, seed: usize) {
    let elemsize = 8usize;
    let label = "binary-churn";
    mark(label);
    reseed(p, seed);
    let cmp = Cmp {
        string_keys: false,
        elemsize,
    };

    let mut ct: *mut c_void = ptr::null_mut();
    let mut rt: *mut c_void = ptr::null_mut();
    let mut rng = Rng::new(0x1357_9BDF);
    let mut live: Vec<u32> = Vec::new();

    for step in 0..4000usize {
        let op = rng.next_u32() % 100;
        if op < 55 || live.is_empty() {
            let k = rng.next_u32();
            let kb = k.to_le_bytes();
            let vb = k.rotate_left(7).to_le_bytes();
            ct = bin_put(&p.c, ct, elemsize, &kb, &vb);
            rt = bin_put(&p.r, rt, elemsize, &kb, &vb);
            if !live.contains(&k) {
                live.push(k);
            }
        } else {
            let idx = (rng.next_u32() as usize) % live.len();
            let k = live.swap_remove(idx);
            let kb = k.to_le_bytes();
            let (c2, cr) = bin_del(&p.c, ct, elemsize, &kb);
            let (r2, rr) = bin_del(&p.r, rt, elemsize, &kb);
            ct = c2;
            rt = r2;
            assert_eq!(cr, rr, "{label}: del result at step {step}");
        }
        if step % 7 == 0 || step > 3900 {
            cmp.check(ct, rt, &format!("{label}: step {step}"));
        }
    }
    cmp.check(ct, rt, &format!("{label}: final"));
    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
}

/// String-keyed map. `mode` selects the implicit SH_DEFAULT behaviour (None) or
/// an explicit `sh_new_strdup` / `sh_new_arena` map.
unsafe fn scenario_string(p: &Pair, mode: Option<c_int>, n: usize, seed: usize) {
    let elemsize = KEYPTR_SIZE + 8; // { char *key; int value; } with tail padding
    let label = format!("string(mode={mode:?}, n={n})");
    mark(&label);
    reseed(p, seed);
    let cmp = Cmp {
        string_keys: true,
        elemsize,
    };

    // keys must outlive the maps (SH_DEFAULT stores the caller's pointer)
    let mut keybufs: Vec<Vec<c_char>> = (0..n * 3).map(|i| cbuf(&format!("key_{i}"))).collect();

    let (mut ct, mut rt) = match mode {
        None => (ptr::null_mut::<c_void>(), ptr::null_mut::<c_void>()),
        Some(m) => (
            (p.c.shmode_func)(elemsize, m),
            (p.r.shmode_func)(elemsize, m),
        ),
    };
    if mode.is_some() {
        cmp.check(ct, rt, &format!("{label}: after shmode_func"));
    }

    for i in 0..n {
        let kp = keybufs[i].as_mut_ptr();
        ct = sh_put(&p.c, ct, elemsize, kp, i as c_int);
        rt = sh_put(&p.r, rt, elemsize, kp, i as c_int);
        cmp.check(ct, rt, &format!("{label}: after shput #{i}"));
        // temp_key is defined right after a string-mode hmput_key
        assert_eq!(
            temp_key_str(raw_of(ct, elemsize)),
            temp_key_str(raw_of(rt, elemsize)),
            "{label}: temp_key after shput #{i}"
        );
        assert_eq!(
            temp_key_str(raw_of(ct, elemsize)),
            cstr(kp),
            "{label}: C temp_key content after shput #{i}"
        );
    }
    assert_eq!(hmlen(ct, elemsize), n as isize, "{label}: C length");
    assert_eq!(hmlen(rt, elemsize), n as isize, "{label}: Rust length");

    // overwrite through shput and shputs
    for i in (0..n).step_by(4) {
        let kp = keybufs[i].as_mut_ptr();
        ct = sh_put(&p.c, ct, elemsize, kp, -(i as c_int));
        rt = sh_put(&p.r, rt, elemsize, kp, -(i as c_int));
        cmp.check(ct, rt, &format!("{label}: after shput overwrite #{i}"));
    }
    for i in (1..n).step_by(5) {
        // `shputs` must only be used for *new* keys: on an overwrite the C macro
        // reads `stbds_temp_key`, which `stbds_hmput_key` leaves stale when the
        // match happens in its wrap-around probe loop, corrupting the table.
        let j = 2 * n + i;
        let kp = keybufs[j].as_mut_ptr();
        ct = sh_puts(&p.c, ct, elemsize, kp, 1000 + i as c_int);
        rt = sh_puts(&p.r, rt, elemsize, kp, 1000 + i as c_int);
        cmp.check(ct, rt, &format!("{label}: after shputs insert #{j}"));
        assert_eq!(
            temp_key_str(raw_of(ct, elemsize)),
            temp_key_str(raw_of(rt, elemsize)),
            "{label}: temp_key after shputs #{j}"
        );
    }

    // lookups, hits and misses
    for i in 0..n * 2 {
        let kp = keybufs[i].as_mut_ptr();
        let (c2, ci) = sh_geti(&p.c, ct, elemsize, kp);
        let (r2, ri) = sh_geti(&p.r, rt, elemsize, kp);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "{label}: shgeti(#{i})");
        cmp.check(ct, rt, &format!("{label}: after shgeti #{i}"));
    }

    // deletes
    for i in (0..n).step_by(2) {
        let kp = keybufs[i].as_mut_ptr();
        mark(&format!("{label}: shdel #{i}"));
        let (c2, cr) = sh_del(&p.c, ct, elemsize, kp);
        let (r2, rr) = sh_del(&p.r, rt, elemsize, kp);
        ct = c2;
        rt = r2;
        assert_eq!(cr, rr, "{label}: shdel(#{i})");
        cmp.check(ct, rt, &format!("{label}: after shdel #{i}"));
    }

    // re-insert with fresh key buffers (distinct pointers, equal contents)
    let mut fresh: Vec<Vec<c_char>> = (0..n).map(|i| cbuf(&format!("key_{i}"))).collect();
    for i in (0..n).step_by(2) {
        let kp = fresh[i].as_mut_ptr();
        ct = sh_put(&p.c, ct, elemsize, kp, 7000 + i as c_int);
        rt = sh_put(&p.r, rt, elemsize, kp, 7000 + i as c_int);
        cmp.check(ct, rt, &format!("{label}: after reinsert #{i}"));
    }

    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
    drop(keybufs.pop());
    drop(fresh.pop());
}

/// Long string keys so the arena mode takes both the small-block and the
/// oversized-block path.
unsafe fn scenario_string_long_keys(p: &Pair, mode: c_int, seed: usize) {
    let elemsize = KEYPTR_SIZE + 8;
    let label = format!("string-longkeys(mode={mode})");
    mark(&label);
    reseed(p, seed);
    let cmp = Cmp {
        string_keys: true,
        elemsize,
    };

    let lens = [1usize, 7, 100, 511, 512, 513, 2000, 3, 9000, 5];
    let mut keybufs: Vec<Vec<c_char>> = lens
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let mut s = String::with_capacity(l + 8);
            s.push_str(&format!("{i}:"));
            while s.len() < l {
                s.push((b'a' + (s.len() % 26) as u8) as char);
            }
            cbuf(&s)
        })
        .collect();

    let mut ct = (p.c.shmode_func)(elemsize, mode);
    let mut rt = (p.r.shmode_func)(elemsize, mode);
    cmp.check(ct, rt, &format!("{label}: after shmode_func"));

    for i in 0..keybufs.len() {
        let kp = keybufs[i].as_mut_ptr();
        ct = sh_put(&p.c, ct, elemsize, kp, i as c_int);
        rt = sh_put(&p.r, rt, elemsize, kp, i as c_int);
        cmp.check(ct, rt, &format!("{label}: after shput #{i}"));
    }
    for i in 0..keybufs.len() {
        let kp = keybufs[i].as_mut_ptr();
        let (c2, ci) = sh_geti(&p.c, ct, elemsize, kp);
        let (r2, ri) = sh_geti(&p.r, rt, elemsize, kp);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "{label}: shgeti(#{i})");
    }
    for i in (0..keybufs.len()).step_by(2) {
        let kp = keybufs[i].as_mut_ptr();
        mark(&format!("{label}: shdel #{i}"));
        let (c2, cr) = sh_del(&p.c, ct, elemsize, kp);
        let (r2, rr) = sh_del(&p.r, rt, elemsize, kp);
        ct = c2;
        rt = r2;
        assert_eq!(cr, rr, "{label}: shdel(#{i})");
        cmp.check(ct, rt, &format!("{label}: after shdel #{i}"));
    }

    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
    drop(keybufs.pop());
}

unsafe fn scenario_hmput_default(p: &Pair, seed: usize) {
    mark("hmput_default");
    reseed(p, seed);
    let elemsize = 16usize;
    let cmp = Cmp {
        string_keys: false,
        elemsize,
    };

    // on a NULL map
    let mut ct = (p.c.hmput_default)(ptr::null_mut(), elemsize);
    let mut rt = (p.r.hmput_default)(ptr::null_mut(), elemsize);
    cmp.check(ct, rt, "hmput_default(NULL)");

    // `hmdefault(t, v)`: t[-1].value = v
    *((ct as *mut u8).offset(-(elemsize as isize)).add(8) as *mut u64) = 0xDEAD_BEEF;
    *((rt as *mut u8).offset(-(elemsize as isize)).add(8) as *mut u64) = 0xDEAD_BEEF;
    cmp.check(ct, rt, "hmdefault stored");

    // idempotent on a non-empty map
    let ct2 = (p.c.hmput_default)(ct, elemsize);
    let rt2 = (p.r.hmput_default)(rt, elemsize);
    assert_eq!(ct, ct2, "C hmput_default moved a non-empty map");
    assert_eq!(rt, rt2, "Rust hmput_default moved a non-empty map");
    cmp.check(ct2, rt2, "hmput_default(non-empty)");
    ct = ct2;
    rt = rt2;

    // and the default survives real inserts, plus misses return -1
    for i in 0..40u64 {
        let kb = i.to_le_bytes();
        let vb = (i * 11).to_le_bytes();
        ct = bin_put(&p.c, ct, elemsize, &kb, &vb);
        rt = bin_put(&p.r, rt, elemsize, &kb, &vb);
        cmp.check(ct, rt, &format!("hmput_default + put #{i}"));
    }
    for i in 40..80u64 {
        let kb = i.to_le_bytes();
        let (c2, ci) = bin_geti(&p.c, ct, elemsize, &kb);
        let (r2, ri) = bin_geti(&p.r, rt, elemsize, &kb);
        ct = c2;
        rt = r2;
        assert_eq!(ci, ri, "hmput_default miss #{i}");
        assert_eq!(ci, -1, "expected a miss for #{i}");
    }

    (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
    (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
}

unsafe fn scenario_null_inputs(p: &Pair, seed: usize) {
    mark("null_inputs");
    reseed(p, seed);

    for &elemsize in &[8usize, 16, 20, 64] {
        let keysize = 4usize.min(elemsize);
        let cmp = Cmp {
            string_keys: false,
            elemsize,
        };
        let mut key = vec![0x11u8; keysize];

        // hmget_key on a NULL map creates a default-only map
        let ct = (p.c.hmget_key)(
            ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        let rt = (p.r.hmget_key)(
            ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            HM_BINARY,
        );
        cmp.check(ct, rt, &format!("hmget_key(NULL, e={elemsize})"));
        assert_eq!(temp_of(ct, elemsize), -1);

        // a second lookup on the now table-less map
        let (ct, ci) = bin_geti(&p.c, ct, elemsize, &key);
        let (rt, ri) = bin_geti(&p.r, rt, elemsize, &key);
        assert_eq!(ci, ri, "hmget_key on table-less map (e={elemsize})");
        cmp.check(ct, rt, &format!("hmget_key #2 (e={elemsize})"));

        // hmdel_key on that map (table == NULL branch)
        let (ct, cr) = bin_del(&p.c, ct, elemsize, &key);
        let (rt, rr) = bin_del(&p.r, rt, elemsize, &key);
        assert_eq!(cr, rr, "hmdel_key table-less (e={elemsize})");
        cmp.check(ct, rt, &format!("hmdel_key table-less (e={elemsize})"));

        (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
        (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);

        // hmget_key_ts on NULL
        let mut ctemp: isize = 0x1234;
        let mut rtemp: isize = 0x1234;
        let ct = (p.c.hmget_key_ts)(
            ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            &mut ctemp,
            HM_BINARY,
        );
        let rt = (p.r.hmget_key_ts)(
            ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            &mut rtemp,
            HM_BINARY,
        );
        assert_eq!(ctemp, rtemp, "hmget_key_ts(NULL) temp (e={elemsize})");
        cmp.check(ct, rt, &format!("hmget_key_ts(NULL, e={elemsize})"));
        (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
        (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);

        // hmdel_key(NULL) returns NULL
        let cd = (p.c.hmdel_key)(
            ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            0,
            HM_BINARY,
        );
        let rd = (p.r.hmdel_key)(
            ptr::null_mut(),
            elemsize,
            key.as_mut_ptr() as *mut c_void,
            keysize,
            0,
            HM_BINARY,
        );
        assert!(cd.is_null(), "C hmdel_key(NULL) should be NULL");
        assert!(rd.is_null(), "Rust hmdel_key(NULL) should be NULL");

        // hmfree_func(NULL) must be a no-op
        (p.c.hmfree_func)(ptr::null_mut(), elemsize);
        (p.r.hmfree_func)(ptr::null_mut(), elemsize);
    }
}

unsafe fn scenario_shmode_modes(p: &Pair, seed: usize) {
    mark("shmode_modes");
    reseed(p, seed);
    let elemsize = 16usize;
    let cmp = Cmp {
        string_keys: false,
        elemsize,
    };
    for &m in &[SH_NONE, SH_DEFAULT, SH_STRDUP, SH_ARENA, 7] {
        let ct = (p.c.shmode_func)(elemsize, m);
        let rt = (p.r.shmode_func)(elemsize, m);
        cmp.check(ct, rt, &format!("shmode_func(mode={m})"));
        (p.c.hmfree_func)(raw_of(ct, elemsize), elemsize);
        (p.r.hmfree_func)(raw_of(rt, elemsize), elemsize);
    }
}

/// A binary map created via `sh_new_*` keeps `string.mode` set. Mixing modes
/// like that makes the C code compare a caller string against a heap pointer,
/// so the outcome depends on allocator addresses and is not comparable across
/// two independent libraries -- deliberately not exercised.

// ---------------------------------------------------------------------------

#[test]
fn hashmap_matches_c() {
    let p = load_pair();
    unsafe {
        scenario_null_inputs(&p, 0x3141_5926);
        scenario_shmode_modes(&p, 0x3141_5926);
        scenario_hmput_default(&p, 0x3141_5926);

        scenario_binary(&p, 8, 4, 1, 0x3141_5926);
        scenario_binary(&p, 8, 4, 2, 0x3141_5926);
        scenario_binary(&p, 8, 4, 6, 0x3141_5926);
        scenario_binary(&p, 8, 4, 7, 0x3141_5926);
        scenario_binary(&p, 8, 4, 8, 0x3141_5926);
        scenario_binary(&p, 8, 4, 64, 0x3141_5926);
        scenario_binary(&p, 8, 4, 300, 1);
        scenario_binary(&p, 16, 8, 200, 0);
        scenario_binary(&p, 20, 8, 150, usize::MAX);
        scenario_binary(&p, 64, 32, 90, 12345);
        scenario_binary(&p, 4, 1, 120, 777);

        scenario_binary_churn(&p, 0x3141_5926);

        scenario_string(&p, None, 120, 0x3141_5926);
        scenario_string(&p, Some(SH_DEFAULT), 120, 0x3141_5926);
        scenario_string(&p, Some(SH_STRDUP), 120, 0x3141_5926);
        scenario_string(&p, Some(SH_ARENA), 120, 0x3141_5926);

        scenario_string_long_keys(&p, SH_ARENA, 0x3141_5926);
        scenario_string_long_keys(&p, SH_STRDUP, 0x3141_5926);
    }
}
