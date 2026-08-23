//! Phase B — value API differential tests. CONFIGS.md rows 224-273.
//!
//! Every assertion drives BOTH the C `.so` and the Rust `.so` through their
//! exported symbols only (never a direct Rust call) and compares the observable
//! results: return codes, `json_t.type`, `json_t.refcount`, container sizes,
//! iteration order, raw string bytes and full `json_dumps` output.
//!
//! The C library is ground truth. Nothing here is weakened to accommodate the
//! Rust port.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_double, c_int, c_void};

// ---------------------------------------------------------------- extra fn types

type FnNew = unsafe extern "C" fn() -> *mut json_t;
type FnObjIterAt = unsafe extern "C" fn(*mut json_t, *const c_char) -> *mut c_void;
type FnKeyToIter = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnIterSetNew = unsafe extern "C" fn(*mut json_t, *mut c_void, *mut json_t) -> c_int;
type FnArrInsertNew = unsafe extern "C" fn(*mut json_t, usize, *mut json_t) -> c_int;
type FnClear = unsafe extern "C" fn(*mut json_t) -> c_int;
type FnStrSet = unsafe extern "C" fn(*mut json_t, *const c_char) -> c_int;
type FnStrSetN = unsafe extern "C" fn(*mut json_t, *const c_char, usize) -> c_int;
type FnIntSet = unsafe extern "C" fn(*mut json_t, json_int_t) -> c_int;
type FnRealSet = unsafe extern "C" fn(*mut json_t, c_double) -> c_int;

// ---------------------------------------------------------------- constructors

unsafe fn n_object(lib: &Library) -> *mut json_t {
    let f: Symbol<FnNew> = sym(lib, "json_object");
    f()
}
unsafe fn n_array(lib: &Library) -> *mut json_t {
    let f: Symbol<FnNew> = sym(lib, "json_array");
    f()
}
unsafe fn n_int(lib: &Library, v: i64) -> *mut json_t {
    let f: Symbol<FnInt> = sym(lib, "json_integer");
    f(v)
}
unsafe fn n_real(lib: &Library, v: f64) -> *mut json_t {
    let f: Symbol<FnReal> = sym(lib, "json_real");
    f(v)
}
unsafe fn n_str(lib: &Library, s: &str) -> *mut json_t {
    let f: Symbol<FnStr> = sym(lib, "json_string");
    f(cs(s).as_ptr())
}
unsafe fn n_strn(lib: &Library, b: &[u8]) -> *mut json_t {
    let f: Symbol<FnStrN> = sym(lib, "json_stringn");
    f(b.as_ptr() as *const c_char, b.len())
}
unsafe fn n_strn_nocheck(lib: &Library, b: &[u8]) -> *mut json_t {
    let f: Symbol<FnStrN> = sym(lib, "json_stringn_nocheck");
    f(b.as_ptr() as *const c_char, b.len())
}
unsafe fn s_true(lib: &Library) -> *mut json_t {
    let f: Symbol<FnNew> = sym(lib, "json_true");
    f()
}
unsafe fn s_false(lib: &Library) -> *mut json_t {
    let f: Symbol<FnNew> = sym(lib, "json_false");
    f()
}
unsafe fn s_null(lib: &Library) -> *mut json_t {
    let f: Symbol<FnNew> = sym(lib, "json_null");
    f()
}

// ---------------------------------------------------------------- mutators

unsafe fn oset(lib: &Library, o: *mut json_t, k: &str, v: *mut json_t) -> c_int {
    let f: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
    f(o, cs(k).as_ptr(), v)
}
unsafe fn osetn(lib: &Library, o: *mut json_t, k: &[u8], klen: usize, v: *mut json_t) -> c_int {
    let f: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
    f(o, k.as_ptr() as *const c_char, klen, v)
}
unsafe fn osetn_nocheck(
    lib: &Library,
    o: *mut json_t,
    k: &[u8],
    klen: usize,
    v: *mut json_t,
) -> c_int {
    let f: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new_nocheck");
    f(o, k.as_ptr() as *const c_char, klen, v)
}
unsafe fn oget(lib: &Library, o: *mut json_t, k: &str) -> *mut json_t {
    let f: Symbol<FnObjGet> = sym(lib, "json_object_get");
    f(o, cs(k).as_ptr())
}
unsafe fn ogetn(lib: &Library, o: *mut json_t, k: &[u8], klen: usize) -> *mut json_t {
    let f: Symbol<FnObjGetN> = sym(lib, "json_object_getn");
    f(o, k.as_ptr() as *const c_char, klen)
}
unsafe fn odel(lib: &Library, o: *mut json_t, k: &str) -> c_int {
    let f: Symbol<FnObjDel> = sym(lib, "json_object_del");
    f(o, cs(k).as_ptr())
}
unsafe fn odeln(lib: &Library, o: *mut json_t, k: &[u8], klen: usize) -> c_int {
    let f: Symbol<FnObjDelN> = sym(lib, "json_object_deln");
    f(o, k.as_ptr() as *const c_char, klen)
}
unsafe fn aappend(lib: &Library, a: *mut json_t, v: *mut json_t) -> c_int {
    let f: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
    f(a, v)
}
unsafe fn ainsert(lib: &Library, a: *mut json_t, i: usize, v: *mut json_t) -> c_int {
    let f: Symbol<FnArrInsertNew> = sym(lib, "json_array_insert_new");
    f(a, i, v)
}
unsafe fn aset(lib: &Library, a: *mut json_t, i: usize, v: *mut json_t) -> c_int {
    let f: Symbol<FnArrSetNew> = sym(lib, "json_array_set_new");
    f(a, i, v)
}
unsafe fn aremove(lib: &Library, a: *mut json_t, i: usize) -> c_int {
    let f: Symbol<FnArrRemove> = sym(lib, "json_array_remove");
    f(a, i)
}
unsafe fn aget(lib: &Library, a: *mut json_t, i: usize) -> *mut json_t {
    let f: Symbol<FnArrGet> = sym(lib, "json_array_get");
    f(a, i)
}
unsafe fn osize(lib: &Library, j: *const json_t) -> usize {
    let f: Symbol<FnSize> = sym(lib, "json_object_size");
    f(j)
}
unsafe fn asize(lib: &Library, j: *const json_t) -> usize {
    let f: Symbol<FnSize> = sym(lib, "json_array_size");
    f(j)
}

// ---------------------------------------------------------------- observation

/// Everything about a value that is comparable across the two libraries.
/// `rc == -1` means "the pointer was NULL", `rc == -2` means "refcount was
/// `(size_t)-1`", i.e. a singleton.
#[derive(PartialEq, Debug)]
struct V {
    ty: c_int,
    rc: i64,
    osz: usize,
    asz: usize,
    plain: Option<String>,
    sorted: Option<String>,
}

unsafe fn view(lib: &Library, j: *const json_t) -> V {
    if j.is_null() {
        return V { ty: -1, rc: -1, osz: 0, asz: 0, plain: None, sorted: None };
    }
    let rc = (*j).refcount;
    V {
        ty: (*j).type_,
        rc: if rc == usize::MAX { -2 } else { rc as i64 },
        osz: osize(lib, j),
        asz: asize(lib, j),
        plain: dumps_to_string(lib, j, JSON_ENCODE_ANY),
        sorted: dumps_to_string(lib, j, JSON_ENCODE_ANY | JSON_SORT_KEYS),
    }
}

/// `refcount` only (`-1` NULL, `-2` singleton).
unsafe fn rc(j: *const json_t) -> i64 {
    if j.is_null() {
        return -1;
    }
    let r = (*j).refcount;
    if r == usize::MAX {
        -2
    } else {
        r as i64
    }
}

/// Full insertion-ordered (key bytes, key_len, dumped value) list of an object,
/// walked through the exported iterator family.
unsafe fn key_order(lib: &Library, o: *mut json_t) -> Vec<(Vec<u8>, usize, Option<String>)> {
    let it: Symbol<FnIter> = sym(lib, "json_object_iter");
    let nx: Symbol<FnIterNext> = sym(lib, "json_object_iter_next");
    let ky: Symbol<FnIterKey> = sym(lib, "json_object_iter_key");
    let kl: Symbol<FnIterKeyLen> = sym(lib, "json_object_iter_key_len");
    let vl: Symbol<FnIterValue> = sym(lib, "json_object_iter_value");
    let mut out = Vec::new();
    let mut iter = it(o);
    while !iter.is_null() {
        let k = ky(iter);
        let n = kl(iter);
        let bytes = if k.is_null() {
            Vec::new()
        } else {
            std::slice::from_raw_parts(k as *const u8, n).to_vec()
        };
        out.push((bytes, n, dumps_to_string(lib, vl(iter), JSON_ENCODE_ANY)));
        iter = nx(o, iter);
    }
    out
}

/// `(bytes-by-length, length, NUL-terminated view)` of a JSON string.
unsafe fn str_bytes(lib: &Library, j: *const json_t) -> (Option<Vec<u8>>, usize, String) {
    let sv: Symbol<FnStrVal> = sym(lib, "json_string_value");
    let sl: Symbol<FnSize> = sym(lib, "json_string_length");
    let p = sv(j);
    let n = sl(j);
    if p.is_null() {
        return (None, n, "<null>".to_string());
    }
    (
        Some(std::slice::from_raw_parts(p as *const u8, n).to_vec()),
        n,
        cstr_to_string(p),
    )
}

unsafe fn del(lib: &Library, j: *mut json_t) {
    let f: Symbol<FnDelete> = sym(lib, "json_delete");
    f(j)
}

/// Invalid-UTF-8 byte strings used by the string / key rejection rows.
const BAD_UTF8: &[&[u8]] = &[
    b"\xff",
    b"\x80",
    b"\xc0\x80",             // overlong NUL
    b"\xc1\xbf",             // overlong
    b"\xe0\x80\xa9",         // overlong
    b"\xed\xa0\x80",         // UTF-16 surrogate half
    b"\xf4\x90\x80\x80",     // > 0x10FFFF
    b"\xf5\x80\x80\x80",     // restricted lead byte
    b"a\xe2\x82",            // truncated 3-byte
    b"ok\xffend",
];

/// Deterministic ASCII key of exactly `n` bytes.
fn key_of_len(rng: &mut Rng, n: usize) -> Vec<u8> {
    const AL: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-";
    (0..n).map(|_| AL[rng.below(AL.len() as u64) as usize]).collect()
}

// ================================================================ row 224

#[test]
fn row224_fresh_empty_object() {
    diff("row224/fresh object", |lib: &Library| unsafe {
        let o = n_object(lib);
        let v = view(lib, o);
        let s = osize(lib, o);
        // documented state of a brand-new object
        assert_eq!(v.ty, JSON_OBJECT, "type");
        assert_eq!(v.rc, 1, "refcount");
        assert_eq!(s, 0, "json_object_size");
        assert_eq!(v.plain.as_deref(), Some("{}"), "dump");
        decref(lib, o);
        (v, s)
    });

    // json_object_size on every non-object -> 0
    diff("row224/size of non-objects", |lib: &Library| unsafe {
        let a = n_array(lib);
        let i = n_int(lib, 1);
        let out = (
            osize(lib, a),
            osize(lib, i),
            osize(lib, s_true(lib)),
            osize(lib, std::ptr::null()),
            asize(lib, std::ptr::null()),
        );
        decref(lib, a);
        decref(lib, i);
        out
    });
}

// ================================================================ rows 225, 226

#[test]
fn row225_key_length_and_alignment_paths() {
    // Fixed, hand-picked key lengths that exercise every `hashlittle` tail case.
    const LENS: &[usize] = &[0, 1, 2, 3, 4, 5, 7, 8, 11, 12, 13, 16, 24, 25, 33];

    diff("row225/fixed key lengths", |lib: &Library| unsafe {
        let mut rng = Rng::new(0x2251);
        let o = n_object(lib);
        let mut rets = Vec::new();
        for (i, &n) in LENS.iter().enumerate() {
            let mut k = key_of_len(&mut rng, n);
            // make every key distinct even for n == 0/1 by suffixing nothing:
            // instead give each length its own object slot via a distinguishing
            // prefix byte where there is room.
            if n > 0 {
                k[0] = b'A' + (i as u8);
            }
            rets.push(osetn(lib, o, &k, k.len(), n_int(lib, i as i64)));
        }
        let out = (rets, view(lib, o), key_order(lib, o));
        decref(lib, o);
        out
    });

    // Randomized keys of the same lengths, inserted at four different pointer
    // alignments (hashlittle has separate aligned/unaligned read paths).
    diff_n("row225/randomized keys x alignment", 400, |lib: &Library, it: u64| unsafe {
        let mut rng = Rng::new(0x9000 ^ it.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let o = n_object(lib);
        let mut rets = Vec::new();
        for &n in LENS {
            for off in 0..4usize {
                let k = key_of_len(&mut rng, n);
                // Place the key at byte offset `off` inside a heap buffer so the
                // pointer's low bits vary.
                let mut buf = vec![0u8; off + n + 8];
                buf[off..off + n].copy_from_slice(&k);
                let base = buf.as_ptr().add(off);
                let f: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
                let r = f(o, base as *const c_char, n, n_int(lib, (n * 4 + off) as i64));
                // read it straight back at a *different* alignment
                let mut buf2 = vec![0u8; 4 + n];
                buf2[3..3 + n].copy_from_slice(&k);
                let g = ogetn(lib, o, &buf2[3..], n);
                rets.push((r, view(lib, g).plain));
            }
        }
        let out = (rets, view(lib, o), key_order(lib, o));
        decref(lib, o);
        out
    });

    // row 226: explicit key_len shorter than strlen(key)
    diff("row226/key_len < strlen", |lib: &Library| unsafe {
        let o = n_object(lib);
        let full = b"abcdefghij";
        let r0 = osetn(lib, o, full, 3, n_int(lib, 1)); // key "abc"
        let r1 = osetn(lib, o, full, 10, n_int(lib, 2)); // key "abcdefghij"
        let r2 = osetn(lib, o, full, 0, n_int(lib, 3)); // key ""
        let g0 = view(lib, ogetn(lib, o, full, 3));
        let g1 = view(lib, ogetn(lib, o, full, 10));
        let g2 = view(lib, ogetn(lib, o, full, 0));
        let g3 = view(lib, ogetn(lib, o, full, 4)); // "abcd" was never inserted
        let out = ((r0, r1, r2), g0, g1, g2, g3, view(lib, o), key_order(lib, o));
        decref(lib, o);
        out
    });
}

// ================================================================ rows 227-229

#[test]
fn rows227_229_key_utf8_and_nul() {
    // row 227: embedded NUL in the key is ACCEPTED (NUL passes utf8_check_string)
    diff("row227/embedded NUL key", |lib: &Library| unsafe {
        let o = n_object(lib);
        let k = b"a\0b";
        let r = osetn(lib, o, k, 3, n_int(lib, 7));
        let hit = view(lib, ogetn(lib, o, k, 3));
        let miss = view(lib, ogetn(lib, o, k, 1)); // "a" alone
        let via_get = view(lib, oget(lib, o, "a")); // json_object_get uses strlen -> "a"
        let out = (r, hit, miss, via_get, view(lib, o), key_order(lib, o));
        decref(lib, o);
        out
    });

    // row 228: *_nocheck accepts invalid UTF-8 keys
    diff("row228/nocheck invalid-UTF8 keys", |lib: &Library| unsafe {
        let mut rets = Vec::new();
        for (i, bad) in BAD_UTF8.iter().enumerate() {
            let o = n_object(lib);
            let r = osetn_nocheck(lib, o, bad, bad.len(), n_int(lib, i as i64));
            let g = view(lib, ogetn(lib, o, bad, bad.len()));
            // json_object_set_new_nocheck (strlen flavour)
            let f: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new_nocheck");
            let z = cs_bytes(bad);
            let r2 = f(o, z.as_ptr() as *const c_char, n_int(lib, 100 + i as i64));
            rets.push((r, r2, g, view(lib, o), key_order(lib, o)));
            decref(lib, o);
        }
        rets
    });

    // row 229: checked setters reject invalid UTF-8 keys and decref the value
    diff("row229/checked invalid-UTF8 keys rejected", |lib: &Library| unsafe {
        let mut rets = Vec::new();
        for bad in BAD_UTF8 {
            let o = n_object(lib);
            // sentinel value we still own a reference to, so we can watch the decref
            let val = n_int(lib, 42);
            incref(val);
            let r = osetn(lib, o, bad, bad.len(), val);
            let after = rc(val);
            // strlen flavour
            let z = cs_bytes(bad);
            let val2 = n_int(lib, 43);
            incref(val2);
            let f: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
            let r2 = f(o, z.as_ptr() as *const c_char, val2);
            let after2 = rc(val2);
            rets.push((r, after, r2, after2, osize(lib, o), view(lib, o).plain));
            decref(lib, val);
            decref(lib, val2);
            decref(lib, o);
        }
        rets
    });

    // NULL key pointer
    diff("row229/NULL key pointer", |lib: &Library| unsafe {
        let o = n_object(lib);
        let set: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let setn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
        let get: Symbol<FnObjGet> = sym(lib, "json_object_get");
        let getn: Symbol<FnObjGetN> = sym(lib, "json_object_getn");
        let d: Symbol<FnObjDel> = sym(lib, "json_object_del");
        let dn: Symbol<FnObjDelN> = sym(lib, "json_object_deln");
        let v1 = n_int(lib, 1);
        incref(v1);
        let r1 = set(o, std::ptr::null(), v1);
        let v2 = n_int(lib, 2);
        incref(v2);
        let r2 = setn(o, std::ptr::null(), 0, v2);
        let out = (
            r1,
            rc(v1),
            r2,
            rc(v2),
            get(o, std::ptr::null()).is_null(),
            getn(o, std::ptr::null(), 0).is_null(),
            d(o, std::ptr::null()),
            dn(o, std::ptr::null(), 0),
            osize(lib, o),
        );
        decref(lib, v1);
        decref(lib, v2);
        decref(lib, o);
        out
    });
}

// ================================================================ rows 230-232

#[test]
fn rows230_232_self_insert_replace_rehash() {
    // row 230: value == container is rejected for objects and arrays.
    diff("row230/self insert rejected", |lib: &Library| unsafe {
        let o = n_object(lib);
        incref(o); // emulate json_object_set(): the setter owns one reference
        let r = oset(lib, o, "self", o);
        let after = rc(o);
        let a = n_array(lib);
        incref(a);
        let ra = aappend(lib, a, a);
        let a_after = rc(a);
        incref(a);
        let ri = ainsert(lib, a, 0, a);
        incref(a);
        let rs = aset(lib, a, 0, a);
        let out = (r, after, view(lib, o), ra, ri, rs, a_after, rc(a), view(lib, a));
        decref(lib, o);
        decref(lib, a);
        out
    });

    // row 231: re-setting an existing key replaces the value but keeps the slot.
    diff("row231/replace keeps ordinal", |lib: &Library| unsafe {
        let o = n_object(lib);
        for (i, k) in ["k0", "k1", "k2", "k3"].iter().enumerate() {
            oset(lib, o, k, n_int(lib, i as i64));
        }
        let before = dumps_to_string(lib, o, 0);
        let old = oget(lib, o, "k1");
        incref(old); // watch the old value get decref'd
        let r = oset(lib, o, "k1", n_str(lib, "replaced"));
        let out = (
            r,
            before,
            rc(old),
            view(lib, o),
            key_order(lib, o),
            osize(lib, o),
        );
        decref(lib, old);
        decref(lib, o);
        out
    });

    // row 232: 8th then 9th distinct key crosses the rehash threshold
    // (size >= hashsize(order) == 8).
    diff("row232/rehash at 9th key", |lib: &Library| unsafe {
        let o = n_object(lib);
        let mut snaps = Vec::new();
        for i in 0..20 {
            let k = format!("key{:02}", i);
            let r = oset(lib, o, &k, n_int(lib, i));
            snaps.push((i, r, osize(lib, o), dumps_to_string(lib, o, 0)));
        }
        let out = (snaps, key_order(lib, o), view(lib, o));
        decref(lib, o);
        out
    });
}

// ================================================================ rows 233-235

#[test]
fn rows233_235_get_del_clear() {
    // row 233
    diff("row233/get and getn", |lib: &Library| unsafe {
        let o = n_object(lib);
        osetn(lib, o, b"abcdef", 6, n_int(lib, 1));
        osetn(lib, o, b"a\0z", 3, n_int(lib, 2));
        osetn(lib, o, b"", 0, n_int(lib, 3));
        let out = (
            view(lib, oget(lib, o, "abcdef")),
            view(lib, oget(lib, o, "abcde")),
            view(lib, oget(lib, o, "abcdefg")),
            view(lib, oget(lib, o, "nope")),
            view(lib, ogetn(lib, o, b"abcdefXX", 6)), // key_len boundary
            view(lib, ogetn(lib, o, b"abcdefXX", 7)),
            view(lib, ogetn(lib, o, b"a\0z", 3)),
            view(lib, ogetn(lib, o, b"a\0z", 2)),
            view(lib, ogetn(lib, o, b"", 0)),
            view(lib, oget(lib, o, "")),
            // non-object receivers
            view(lib, oget(lib, n_array(lib), "x")),
            osize(lib, o),
        );
        decref(lib, o);
        out
    });

    // row 234
    diff("row234/del and deln", |lib: &Library| unsafe {
        let o = n_object(lib);
        for k in ["a", "bb", "ccc", "dddd"] {
            oset(lib, o, k, n_int(lib, k.len() as i64));
        }
        osetn(lib, o, b"n\0l", 3, n_int(lib, 9));
        let r_hit = odel(lib, o, "bb");
        let r_again = odel(lib, o, "bb");
        let r_miss = odel(lib, o, "zzz");
        let r_nul_strlen = odel(lib, o, "n"); // strlen stops at NUL -> key "n" absent
        let r_nul_exact = odeln(lib, o, b"n\0l", 3);
        let r_nul_again = odeln(lib, o, b"n\0l", 3);
        let r_empty = odeln(lib, o, b"", 0);
        // non-object receiver
        let a = n_array(lib);
        let r_bad = odel(lib, a, "a");
        let out = (
            r_hit,
            r_again,
            r_miss,
            r_nul_strlen,
            r_nul_exact,
            r_nul_again,
            r_empty,
            r_bad,
            view(lib, o),
            key_order(lib, o),
        );
        decref(lib, a);
        decref(lib, o);
        out
    });

    // row 235
    diff("row235/clear then reuse", |lib: &Library| unsafe {
        let clear: Symbol<FnClear> = sym(lib, "json_object_clear");
        let o = n_object(lib);
        for i in 0..12 {
            oset(lib, o, &format!("k{}", i), n_int(lib, i));
        }
        let child = n_object(lib);
        incref(child);
        oset(lib, o, "kept", child);
        let before = (osize(lib, o), dumps_to_string(lib, o, 0));
        let r = clear(o);
        let mid = (r, osize(lib, o), dumps_to_string(lib, o, 0), rc(child));
        for i in 0..10 {
            oset(lib, o, &format!("n{}", i), n_int(lib, 100 + i));
        }
        let after = (osize(lib, o), key_order(lib, o), view(lib, o));
        let r_bad = clear(n_int(lib, 1));
        decref(lib, child);
        decref(lib, o);
        (before, mid, after, r_bad)
    });
}

// ================================================================ rows 236-242

#[test]
fn rows236_242_object_update_family() {
    let variants: &[(&str, &str)] = &[
        ("row236", "json_object_update"),
        ("row237", "json_object_update_existing"),
        ("row238", "json_object_update_missing"),
        ("row239", "json_object_update_recursive"),
    ];

    for (row, name) in variants {
        let name = *name;
        // flat: {"a":1,"b":2} + {"b":9,"c":3}
        diff(&format!("{}/{} flat", row, name), move |lib: &Library| unsafe {
            let f: Symbol<FnTwoJson> = sym(lib, name);
            let o = n_object(lib);
            oset(lib, o, "a", n_int(lib, 1));
            oset(lib, o, "b", n_int(lib, 2));
            let other = n_object(lib);
            oset(lib, other, "b", n_int(lib, 9));
            oset(lib, other, "c", n_int(lib, 3));
            let r = f(o, other);
            let out = (r, view(lib, o), key_order(lib, o), view(lib, other));
            decref(lib, other);
            decref(lib, o);
            out
        });

        // nested objects on both sides
        diff(&format!("{}/{} nested", row, name), move |lib: &Library| unsafe {
            let f: Symbol<FnTwoJson> = sym(lib, name);
            let o = n_object(lib);
            let n1 = n_object(lib);
            oset(lib, n1, "x", n_int(lib, 1));
            oset(lib, n1, "y", n_int(lib, 2));
            oset(lib, o, "n", n1);
            oset(lib, o, "keep", n_int(lib, 7));
            let other = n_object(lib);
            let n2 = n_object(lib);
            oset(lib, n2, "y", n_int(lib, 20));
            oset(lib, n2, "z", n_int(lib, 30));
            oset(lib, other, "n", n2);
            oset(lib, other, "new", n_int(lib, 8));
            let r = f(o, other);
            let out = (r, view(lib, o), key_order(lib, o), view(lib, other));
            decref(lib, other);
            decref(lib, o);
            out
        });

        // row 240 shape: object vs non-object value mismatch, both directions
        diff(&format!("{}/{} mismatch", row, name), move |lib: &Library| unsafe {
            let f: Symbol<FnTwoJson> = sym(lib, name);
            // target nested object, other scalar
            let o = n_object(lib);
            let n1 = n_object(lib);
            oset(lib, n1, "x", n_int(lib, 1));
            oset(lib, o, "k", n1);
            let other = n_object(lib);
            oset(lib, other, "k", n_int(lib, 5));
            let r1 = f(o, other);
            let v1 = (r1, view(lib, o), key_order(lib, o));
            decref(lib, other);
            decref(lib, o);
            // target scalar, other nested object
            let o2 = n_object(lib);
            oset(lib, o2, "k", n_int(lib, 5));
            let other2 = n_object(lib);
            let n2 = n_object(lib);
            oset(lib, n2, "x", n_int(lib, 1));
            oset(lib, other2, "k", n2);
            let r2 = f(o2, other2);
            let v2 = (r2, view(lib, o2), key_order(lib, o2), rc(n2));
            decref(lib, other2);
            decref(lib, o2);
            // target array vs other object
            let o3 = n_object(lib);
            let a3 = n_array(lib);
            aappend(lib, a3, n_int(lib, 1));
            oset(lib, o3, "k", a3);
            let other3 = n_object(lib);
            let n3 = n_object(lib);
            oset(lib, n3, "x", n_int(lib, 1));
            oset(lib, other3, "k", n3);
            let r3 = f(o3, other3);
            let v3 = (r3, view(lib, o3));
            decref(lib, other3);
            decref(lib, o3);
            (v1, v2, v3)
        });

        // row 242: non-object arguments, and empty inputs
        diff(&format!("{}/{} bad args", row, name), move |lib: &Library| unsafe {
            let f: Symbol<FnTwoJson> = sym(lib, name);
            let o = n_object(lib);
            oset(lib, o, "a", n_int(lib, 1));
            let a = n_array(lib);
            let i = n_int(lib, 3);
            let empty = n_object(lib);
            let out = (
                f(o, a),
                f(a, o),
                f(o, i),
                f(i, o),
                f(o, std::ptr::null_mut()),
                f(std::ptr::null_mut(), o),
                f(o, empty),
                f(empty, o),
                view(lib, o),
                view(lib, empty),
            );
            decref(lib, empty);
            decref(lib, i);
            decref(lib, a);
            decref(lib, o);
            out
        });
    }

    // row 241: self-referential `other` -> loop check rejects.
    diff("row241/update_recursive self-referential", |lib: &Library| unsafe {
        let f: Symbol<FnTwoJson> = sym(lib, "json_object_update_recursive");
        // a = {"b": b}, b = {"a": a}  (indirect cycle)
        let a = n_object(lib);
        let b = n_object(lib);
        incref(b);
        oset(lib, a, "b", b);
        incref(a);
        oset(lib, b, "a", a);
        let r_self = f(a, a);
        // fresh target with the same shape
        let t = n_object(lib);
        let tb = n_object(lib);
        let tba = n_object(lib);
        oset(lib, tb, "a", tba);
        oset(lib, t, "b", tb);
        let r_target = f(t, a);
        let out = (r_self, r_target, osize(lib, a), osize(lib, b), view(lib, t));
        // break the cycle before dropping our references
        odel(lib, b, "a");
        odel(lib, a, "b");
        decref(lib, t);
        decref(lib, b);
        decref(lib, a);
        out
    });

    // json_object_update_recursive on a deep, acyclic tree
    diff("row239/update_recursive deep", |lib: &Library| unsafe {
        let f: Symbol<FnTwoJson> = sym(lib, "json_object_update_recursive");
        let build = |lib: &Library, leaf: i64| -> *mut json_t {
            let l3 = n_object(lib);
            oset(lib, l3, "leaf", n_int(lib, leaf));
            let l2 = n_object(lib);
            oset(lib, l2, "l3", l3);
            let l1 = n_object(lib);
            oset(lib, l1, "l2", l2);
            l1
        };
        let o = build(lib, 1);
        let other = build(lib, 2);
        oset(lib, other, "extra", n_str(lib, "e"));
        let r = f(o, other);
        let out = (r, view(lib, o), view(lib, other));
        decref(lib, other);
        decref(lib, o);
        out
    });
}

// ================================================================ rows 243-247

#[test]
fn rows243_247_object_iterators() {
    for n in [0usize, 1, 2, 8, 9, 17, 40] {
        diff(&format!("row243/iterate {} keys", n), move |lib: &Library| unsafe {
            let it: Symbol<FnIter> = sym(lib, "json_object_iter");
            let nx: Symbol<FnIterNext> = sym(lib, "json_object_iter_next");
            let o = n_object(lib);
            for i in 0..n {
                oset(lib, o, &format!("k{:03}", i), n_int(lib, i as i64));
            }
            let first_null = it(o).is_null();
            let order = key_order(lib, o);
            // exhaustion: advancing past the last element yields NULL
            let mut iter = it(o);
            let mut steps = 0usize;
            while !iter.is_null() {
                iter = nx(o, iter);
                steps += 1;
            }
            let out = (first_null, steps, order, osize(lib, o));
            decref(lib, o);
            out
        });
    }

    // row 244 + 245 + 246
    diff("rows244_246/iter_at, key_to_iter, iter_set_new", |lib: &Library| unsafe {
        let at: Symbol<FnObjIterAt> = sym(lib, "json_object_iter_at");
        let k2i: Symbol<FnKeyToIter> = sym(lib, "json_object_key_to_iter");
        let ky: Symbol<FnIterKey> = sym(lib, "json_object_iter_key");
        let kl: Symbol<FnIterKeyLen> = sym(lib, "json_object_iter_key_len");
        let vl: Symbol<FnIterValue> = sym(lib, "json_object_iter_value");
        let setn: Symbol<FnIterSetNew> = sym(lib, "json_object_iter_set_new");
        let it: Symbol<FnIter> = sym(lib, "json_object_iter");
        let nx: Symbol<FnIterNext> = sym(lib, "json_object_iter_next");

        let o = n_object(lib);
        for i in 0..10 {
            oset(lib, o, &format!("key{}", i), n_int(lib, i));
        }
        osetn(lib, o, b"z\0z", 3, n_int(lib, 99));

        // iter_at hit / miss
        let hit = at(o, cs("key3").as_ptr());
        let hit_v = view(lib, vl(hit));
        let hit_k = cstr_to_string(ky(hit));
        let hit_kl = kl(hit);
        let miss = at(o, cs("absent").as_ptr()).is_null();
        let nul_key = at(o, cs("z").as_ptr()).is_null(); // strlen -> "z", not "z\0z"
        let null_arg = at(o, std::ptr::null()).is_null();
        let wrong_type = at(n_int(lib, 1), cs("k").as_ptr()).is_null();

        // key_to_iter round trip: iter_key -> key_to_iter -> iter_value
        let mut roundtrip = Vec::new();
        let mut iter = it(o);
        while !iter.is_null() {
            let kp = ky(iter);
            let back = k2i(kp);
            let same = back == iter;
            roundtrip.push((
                same,
                kl(back),
                std::slice::from_raw_parts(ky(back) as *const u8, kl(back)).to_vec(),
                view(lib, vl(back)).plain,
            ));
            iter = nx(o, iter);
        }
        let k2i_null = k2i(std::ptr::null()).is_null();

        // iter_set_new replaces the value in place
        let target = at(o, cs("key5").as_ptr());
        let old = vl(target);
        incref(old);
        let r_set = setn(o, target, n_str(lib, "SET"));
        let old_rc = rc(old);
        // rejected variants (value is decref'd on failure)
        let v1 = n_int(lib, 1);
        incref(v1);
        let r_null_iter = setn(o, std::ptr::null_mut(), v1);
        let v2 = n_int(lib, 2);
        incref(v2);
        let r_null_val = setn(o, target, std::ptr::null_mut());
        let r_bad_obj = setn(n_int(lib, 5), target, v2);

        let out = (
            (hit_v, hit_k, hit_kl, miss, nul_key, null_arg, wrong_type),
            (roundtrip, k2i_null),
            (r_set, old_rc, r_null_iter, rc(v1), r_null_val, r_bad_obj, rc(v2)),
            (view(lib, o), key_order(lib, o)),
        );
        decref(lib, old);
        decref(lib, v1);
        decref(lib, v2);
        decref(lib, o);
        out
    });

    // row 247: every iterator accessor with iter == NULL
    diff("row247/iter == NULL", |lib: &Library| unsafe {
        let nx: Symbol<FnIterNext> = sym(lib, "json_object_iter_next");
        let ky: Symbol<FnIterKey> = sym(lib, "json_object_iter_key");
        let kl: Symbol<FnIterKeyLen> = sym(lib, "json_object_iter_key_len");
        let vl: Symbol<FnIterValue> = sym(lib, "json_object_iter_value");
        let it: Symbol<FnIter> = sym(lib, "json_object_iter");
        let o = n_object(lib);
        oset(lib, o, "a", n_int(lib, 1));
        let out = (
            nx(o, std::ptr::null_mut()).is_null(),
            ky(std::ptr::null_mut()).is_null(),
            kl(std::ptr::null_mut()),
            vl(std::ptr::null_mut()).is_null(),
            it(std::ptr::null_mut()).is_null(),
            it(n_int(lib, 3)).is_null(),
            nx(n_int(lib, 3), std::ptr::null_mut()).is_null(),
        );
        decref(lib, o);
        out
    });
}

// ================================================================ rows 248-256

#[test]
fn rows248_256_arrays() {
    // row 248
    diff("row248/fresh array", |lib: &Library| unsafe {
        let a = n_array(lib);
        let v = view(lib, a);
        assert_eq!(v.ty, JSON_ARRAY, "type");
        assert_eq!(v.rc, 1, "refcount");
        assert_eq!(asize(lib, a), 0, "json_array_size");
        assert_eq!(v.plain.as_deref(), Some("[]"), "dump");
        let out = (v, asize(lib, a), asize(lib, n_object(lib)));
        decref(lib, a);
        out
    });

    // row 249: append 1..8 (in-capacity) then 9th (grow to 16) and beyond
    diff("row249/append growth", |lib: &Library| unsafe {
        let a = n_array(lib);
        let mut steps = Vec::new();
        for i in 0..40 {
            let r = aappend(lib, a, n_int(lib, i));
            steps.push((i, r, asize(lib, a), dumps_to_string(lib, a, 0)));
        }
        // NULL value and wrong receiver
        let r_null = aappend(lib, a, std::ptr::null_mut());
        let o = n_object(lib);
        let v = n_int(lib, 1);
        incref(v);
        let r_obj = aappend(lib, o, v);
        let out = (steps, r_null, r_obj, rc(v), view(lib, a));
        decref(lib, v);
        decref(lib, o);
        decref(lib, a);
        out
    });

    // row 250
    diff("row250/insert_new", |lib: &Library| unsafe {
        let a = n_array(lib);
        for i in 0..4 {
            aappend(lib, a, n_int(lib, i));
        }
        let r0 = ainsert(lib, a, 0, n_str(lib, "front"));
        let d0 = dumps_to_string(lib, a, 0);
        let rm = ainsert(lib, a, 2, n_str(lib, "mid"));
        let dm = dumps_to_string(lib, a, 0);
        let n = asize(lib, a);
        let re = ainsert(lib, a, n, n_str(lib, "end")); // index == entries, no memmove
        let de = dumps_to_string(lib, a, 0);
        let v = n_int(lib, 5);
        incref(v);
        let rx = ainsert(lib, a, asize(lib, a) + 1, v); // index > entries
        let rh = ainsert(lib, a, usize::MAX, n_int(lib, 6));
        let rn = ainsert(lib, a, 0, std::ptr::null_mut());
        let o = n_object(lib);
        let ro = ainsert(lib, o, 0, n_int(lib, 1));
        // insert into an empty array at index 0 (grow path with entries == 0)
        let b = n_array(lib);
        let rb = ainsert(lib, b, 0, n_int(lib, 1));
        let rb2 = ainsert(lib, b, 5, n_int(lib, 2));
        let out = (
            (r0, d0, rm, dm, re, de),
            (rx, rc(v), rh, rn, ro, rb, rb2),
            (view(lib, a), view(lib, b)),
        );
        decref(lib, v);
        decref(lib, b);
        decref(lib, o);
        decref(lib, a);
        out
    });

    // row 251
    diff("row251/set_new", |lib: &Library| unsafe {
        let a = n_array(lib);
        for i in 0..5 {
            aappend(lib, a, n_int(lib, i));
        }
        let old = aget(lib, a, 2);
        incref(old);
        let r_mid = aset(lib, a, 2, n_str(lib, "X"));
        let r_first = aset(lib, a, 0, n_str(lib, "F"));
        let r_last = aset(lib, a, 4, n_str(lib, "L"));
        let v = n_int(lib, 9);
        incref(v);
        let r_eq = aset(lib, a, 5, v); // index == entries
        let r_huge = aset(lib, a, usize::MAX, n_int(lib, 9));
        let r_null = aset(lib, a, 0, std::ptr::null_mut());
        let empty = n_array(lib);
        let r_empty = aset(lib, empty, 0, n_int(lib, 1));
        let out = (
            r_mid,
            r_first,
            r_last,
            r_eq,
            rc(v),
            r_huge,
            r_null,
            r_empty,
            rc(old),
            view(lib, a),
        );
        decref(lib, v);
        decref(lib, old);
        decref(lib, empty);
        decref(lib, a);
        out
    });

    // row 252
    diff("row252/remove", |lib: &Library| unsafe {
        let a = n_array(lib);
        for i in 0..6 {
            aappend(lib, a, n_int(lib, i));
        }
        let kept = aget(lib, a, 3);
        incref(kept);
        let r_first = aremove(lib, a, 0);
        let d1 = dumps_to_string(lib, a, 0);
        let r_mid = aremove(lib, a, 2);
        let d2 = dumps_to_string(lib, a, 0);
        let last = asize(lib, a) - 1;
        let r_last = aremove(lib, a, last); // no memmove branch
        let d3 = dumps_to_string(lib, a, 0);
        let r_oob = aremove(lib, a, asize(lib, a));
        let r_huge = aremove(lib, a, usize::MAX);
        let o = n_object(lib);
        let r_obj = aremove(lib, o, 0);
        let empty = n_array(lib);
        let r_empty = aremove(lib, empty, 0);
        let out = (
            r_first, d1, r_mid, d2, r_last, d3, r_oob, r_huge, r_obj, r_empty, rc(kept),
            view(lib, a),
        );
        decref(lib, kept);
        decref(lib, empty);
        decref(lib, o);
        decref(lib, a);
        out
    });

    // row 253
    diff("row253/clear", |lib: &Library| unsafe {
        let clear: Symbol<FnClear> = sym(lib, "json_array_clear");
        let a = n_array(lib);
        for i in 0..20 {
            aappend(lib, a, n_int(lib, i));
        }
        let child = n_int(lib, 777);
        incref(child);
        aappend(lib, a, child);
        let r = clear(a);
        let mid = (r, asize(lib, a), dumps_to_string(lib, a, 0), rc(child));
        // capacity kept: refilling 21 entries must not change observable output
        for i in 0..21 {
            aappend(lib, a, n_int(lib, 100 + i));
        }
        let r_bad = clear(n_object(lib));
        let out = (mid, view(lib, a), r_bad, clear(std::ptr::null_mut()));
        decref(lib, child);
        decref(lib, a);
        out
    });

    // rows 254, 255
    diff("rows254_255/extend", |lib: &Library| unsafe {
        let ext: Symbol<FnTwoJson> = sym(lib, "json_array_extend");
        // other larger than 2x capacity: 8 -> max(8+20, 16) == 28
        let a = n_array(lib);
        let other = n_array(lib);
        for i in 0..20 {
            aappend(lib, other, n_int(lib, i));
        }
        let r_big = ext(a, other);
        let big = (r_big, asize(lib, a), dumps_to_string(lib, a, 0), rc(aget(lib, other, 0)));
        // empty other
        let empty = n_array(lib);
        let r_empty = ext(a, empty);
        let r_empty2 = ext(empty, a);
        // non-array arguments
        let o = n_object(lib);
        let r_o1 = ext(a, o);
        let r_o2 = ext(o, a);
        let r_n1 = ext(a, std::ptr::null_mut());
        let r_n2 = ext(std::ptr::null_mut(), a);
        // self-extend (allowed: array_copy from itself)
        let s = n_array(lib);
        for i in 0..3 {
            aappend(lib, s, n_int(lib, i));
        }
        let r_self = ext(s, s);
        let out = (
            big,
            r_empty,
            r_empty2,
            r_o1,
            r_o2,
            r_n1,
            r_n2,
            r_self,
            view(lib, s),
            view(lib, a),
            view(lib, empty),
        );
        decref(lib, s);
        decref(lib, o);
        decref(lib, empty);
        decref(lib, other);
        decref(lib, a);
        out
    });

    // row 256
    diff("row256/get", |lib: &Library| unsafe {
        let a = n_array(lib);
        for i in 0..3 {
            aappend(lib, a, n_int(lib, i * 10));
        }
        let out = (
            view(lib, aget(lib, a, 0)),
            view(lib, aget(lib, a, 2)),
            aget(lib, a, 3).is_null(),
            aget(lib, a, 8).is_null(),
            aget(lib, a, usize::MAX).is_null(),
            aget(lib, n_object(lib), 0).is_null(),
            aget(lib, std::ptr::null_mut(), 0).is_null(),
        );
        decref(lib, a);
        out
    });
}

// ================================================================ rows 257-262

#[test]
fn rows257_262_strings() {
    // row 257: json_string / json_stringn over the whole UTF-8 width range
    let good: &[&[u8]] = &[
        b"",
        b"a",
        b"ascii text 123",
        "\u{e9}".as_bytes(),       // 2-byte
        "\u{20ac}".as_bytes(),     // 3-byte
        "\u{10348}".as_bytes(),    // 4-byte
        "mix \u{e9}\u{20ac}\u{10348} end".as_bytes(),
        b"a\0b",           // embedded NUL (json_stringn only)
        b"\0",
        b"\x7f\x01\x1f",
    ];
    diff("row257/json_string+stringn valid", move |lib: &Library| unsafe {
        let mut out = Vec::new();
        for b in good {
            let sn = n_strn(lib, b);
            let a = (view(lib, sn), str_bytes(lib, sn));
            decref(lib, sn);
            let z = cs_bytes(b);
            let f: Symbol<FnStr> = sym(lib, "json_string");
            let s = f(z.as_ptr() as *const c_char);
            let c = (view(lib, s), str_bytes(lib, s));
            decref(lib, s);
            out.push((a, c));
        }
        // NULL input
        let f: Symbol<FnStr> = sym(lib, "json_string");
        let fn_: Symbol<FnStrN> = sym(lib, "json_stringn");
        let fc: Symbol<FnStr> = sym(lib, "json_string_nocheck");
        let fnc: Symbol<FnStrN> = sym(lib, "json_stringn_nocheck");
        let nulls = (
            f(std::ptr::null()).is_null(),
            fn_(std::ptr::null(), 0).is_null(),
            fc(std::ptr::null()).is_null(),
            fnc(std::ptr::null(), 0).is_null(),
        );
        (out, nulls)
    });

    // rows 258, 259 + dumping an invalid-UTF-8 string
    diff("rows258_259/invalid UTF-8", |lib: &Library| unsafe {
        let mut out = Vec::new();
        for bad in BAD_UTF8 {
            // checked: rejected
            let rejected = n_strn(lib, bad).is_null();
            let z = cs_bytes(bad);
            let f: Symbol<FnStr> = sym(lib, "json_string");
            let rejected2 = f(z.as_ptr() as *const c_char).is_null();
            // nocheck: accepted
            let s = n_strn_nocheck(lib, bad);
            let accepted = view(lib, s);
            let raw = str_bytes(lib, s);
            // dumping it must fail (dump_string returns -1 -> json_dumps NULL)
            let d_any = dumps_to_string(lib, s, JSON_ENCODE_ANY);
            let d_ascii = dumps_to_string(lib, s, JSON_ENCODE_ANY | JSON_ENSURE_ASCII);
            // and inside a container
            let a = n_array(lib);
            incref(s);
            aappend(lib, a, s);
            let d_arr = dumps_to_string(lib, a, 0);
            let dumpb: Symbol<FnDumpb> = sym(lib, "json_dumpb");
            let need = dumpb(a, std::ptr::null_mut(), 0, 0);
            decref(lib, a);
            decref(lib, s);
            out.push((rejected, rejected2, accepted, raw, d_any, d_ascii, d_arr, need));
        }
        out
    });

    // rows 260, 261
    diff("rows260_261/string_set family", |lib: &Library| unsafe {
        let set: Symbol<FnStrSet> = sym(lib, "json_string_set");
        let setn: Symbol<FnStrSetN> = sym(lib, "json_string_setn");
        let setc: Symbol<FnStrSet> = sym(lib, "json_string_set_nocheck");
        let setnc: Symbol<FnStrSetN> = sym(lib, "json_string_setn_nocheck");

        let s = n_str(lib, "original value");
        let mut steps = Vec::new();
        // shorter
        steps.push((set(s, cs("hi").as_ptr()), str_bytes(lib, s)));
        // longer
        steps.push((
            set(s, cs("a considerably longer replacement string").as_ptr()),
            str_bytes(lib, s),
        ));
        // empty
        steps.push((set(s, cs("").as_ptr()), str_bytes(lib, s)));
        // explicit length shorter than strlen
        steps.push((setn(s, b"abcdef".as_ptr() as *const c_char, 3), str_bytes(lib, s)));
        // embedded NUL
        steps.push((setn(s, b"x\0y".as_ptr() as *const c_char, 3), str_bytes(lib, s)));
        // multi-byte
        steps.push((set(s, cs("\u{10348}\u{20ac}").as_ptr()), str_bytes(lib, s)));
        // invalid UTF-8: rejected, value untouched
        for bad in BAD_UTF8 {
            let r = setn(s, bad.as_ptr() as *const c_char, bad.len());
            steps.push((r, str_bytes(lib, s)));
            let z = cs_bytes(bad);
            let r2 = set(s, z.as_ptr() as *const c_char);
            steps.push((r2, str_bytes(lib, s)));
        }
        // nocheck: accepted
        let mut nc = Vec::new();
        for bad in BAD_UTF8 {
            let t = n_str(lib, "seed");
            let r = setnc(t, bad.as_ptr() as *const c_char, bad.len());
            nc.push((r, str_bytes(lib, t), dumps_to_string(lib, t, JSON_ENCODE_ANY)));
            let z = cs_bytes(bad);
            let r2 = setc(t, z.as_ptr() as *const c_char);
            nc.push((r2, str_bytes(lib, t), dumps_to_string(lib, t, JSON_ENCODE_ANY)));
            decref(lib, t);
        }
        // NULL value pointer and wrong receiver type
        let i = n_int(lib, 1);
        let bad_args = (
            set(s, std::ptr::null()),
            setn(s, std::ptr::null(), 0),
            setc(s, std::ptr::null()),
            setnc(s, std::ptr::null(), 0),
            set(i, cs("x").as_ptr()),
            setn(i, cs("x").as_ptr(), 1),
            setc(i, cs("x").as_ptr()),
            setnc(i, cs("x").as_ptr(), 1),
            set(std::ptr::null_mut(), cs("x").as_ptr()),
        );
        let out = (steps, nc, bad_args, view(lib, s));
        decref(lib, i);
        decref(lib, s);
        out
    });

    // row 262
    diff("row262/string_value+length", |lib: &Library| unsafe {
        let sv: Symbol<FnStrVal> = sym(lib, "json_string_value");
        let sl: Symbol<FnSize> = sym(lib, "json_string_length");
        let s = n_strn(lib, b"ab\0cd");
        let i = n_int(lib, 1);
        let a = n_array(lib);
        let o = n_object(lib);
        let out = (
            str_bytes(lib, s),
            (sv(i).is_null(), sl(i)),
            (sv(a).is_null(), sl(a)),
            (sv(o).is_null(), sl(o)),
            (sv(s_true(lib)).is_null(), sl(s_true(lib))),
            (sv(s_null(lib)).is_null(), sl(s_null(lib))),
            (sv(std::ptr::null()).is_null(), sl(std::ptr::null())),
        );
        decref(lib, o);
        decref(lib, a);
        decref(lib, i);
        decref(lib, s);
        out
    });
}

// ================================================================ rows 263-266

#[test]
fn rows263_266_numbers() {
    // row 263
    diff("row263/integer", |lib: &Library| unsafe {
        let iv: Symbol<FnIntVal> = sym(lib, "json_integer_value");
        let is: Symbol<FnIntSet> = sym(lib, "json_integer_set");
        let vals: &[i64] = &[0, 1, -1, i64::MIN, i64::MAX, 42, -9007199254740993];
        let mut out = Vec::new();
        for (idx, &v) in vals.iter().enumerate() {
            let j = n_int(lib, v);
            let before = (view(lib, j), iv(j));
            // re-set to the next value in the list, so LLONG_MIN/MAX are also
            // exercised through json_integer_set
            let r = is(j, vals[(idx + 1) % vals.len()]);
            let after = (r, view(lib, j), iv(j));
            out.push((before, after));
            decref(lib, j);
        }
        // wrong types
        let s = n_str(lib, "x");
        let r = n_real(lib, 1.0);
        let a = n_array(lib);
        let bad = (
            iv(s),
            iv(r),
            iv(a),
            iv(s_true(lib)),
            iv(std::ptr::null()),
            is(s, 5),
            is(r, 5),
            is(a, 5),
            is(s_true(lib), 5),
            is(std::ptr::null_mut(), 5),
        );
        decref(lib, a);
        decref(lib, r);
        decref(lib, s);
        (out, bad)
    });

    // rows 264, 265
    diff("rows264_265/real", |lib: &Library| unsafe {
        let rv: Symbol<FnRealVal> = sym(lib, "json_real_value");
        let rs: Symbol<FnRealSet> = sym(lib, "json_real_set");
        let good: &[f64] = &[
            0.0,
            -0.0,
            5e-324,
            f64::MIN_POSITIVE,
            f64::MAX,
            -f64::MAX,
            1.0,
            -1.5,
            3.141592653589793,
            1e300,
            1e-300,
        ];
        let mut out = Vec::new();
        for &v in good {
            let j = n_real(lib, v);
            let before = (view(lib, j), rv(j).to_bits());
            let r = rs(j, -v);
            let after = (r, view(lib, j), rv(j).to_bits());
            out.push((before, after));
            decref(lib, j);
        }
        // NaN / +-Inf rejected by both the constructor and the setter
        let bad_vals: &[f64] = &[f64::NAN, -f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
        let mut bad = Vec::new();
        for &v in bad_vals {
            let cons = n_real(lib, v).is_null();
            let j = n_real(lib, 1.25);
            let r = rs(j, v);
            bad.push((cons, r, rv(j).to_bits(), view(lib, j).plain));
            decref(lib, j);
        }
        // wrong types
        let s = n_str(lib, "x");
        let i = n_int(lib, 1);
        let wrong = (
            rv(s).to_bits(),
            rv(i).to_bits(),
            rv(s_null(lib)).to_bits(),
            rv(std::ptr::null()).to_bits(),
            rs(s, 1.0),
            rs(i, 1.0),
            rs(std::ptr::null_mut(), 1.0),
        );
        decref(lib, i);
        decref(lib, s);
        (out, bad, wrong)
    });

    // row 266
    diff("row266/number_value", |lib: &Library| unsafe {
        let nv: Symbol<FnRealVal> = sym(lib, "json_number_value");
        let i = n_int(lib, -7);
        let big = n_int(lib, i64::MAX);
        let r = n_real(lib, 2.5);
        let neg0 = n_real(lib, -0.0);
        let s = n_str(lib, "3");
        let a = n_array(lib);
        let out = (
            nv(i).to_bits(),
            nv(big).to_bits(),
            nv(r).to_bits(),
            nv(neg0).to_bits(),
            nv(s).to_bits(),
            nv(a).to_bits(),
            nv(s_true(lib)).to_bits(),
            nv(s_false(lib)).to_bits(),
            nv(s_null(lib)).to_bits(),
            nv(std::ptr::null()).to_bits(),
        );
        decref(lib, a);
        decref(lib, s);
        decref(lib, neg0);
        decref(lib, r);
        decref(lib, big);
        decref(lib, i);
        out
    });
}

// ================================================================ row 267

#[test]
fn row267_singletons() {
    diff("row267/singletons", |lib: &Library| unsafe {
        let t = s_true(lib);
        let f = s_false(lib);
        let n = s_null(lib);
        // refcount is literally (size_t)-1
        assert_eq!((*t).refcount, usize::MAX, "json_true refcount");
        assert_eq!((*f).refcount, usize::MAX, "json_false refcount");
        assert_eq!((*n).refcount, usize::MAX, "json_null refcount");
        // stable identity
        let stable = (t == s_true(lib), f == s_false(lib), n == s_null(lib));
        // incref / decref are no-ops
        incref(t);
        incref(n);
        decref(lib, t);
        decref(lib, f);
        decref(lib, n);
        decref(lib, n);
        // json_delete on a singleton falls into `default: return` and frees nothing
        del(lib, t);
        del(lib, f);
        del(lib, n);
        (
            stable,
            view(lib, t),
            view(lib, f),
            view(lib, n),
            (*t).refcount == usize::MAX,
            (*f).refcount == usize::MAX,
            (*n).refcount == usize::MAX,
            // still usable after all of the above
            dumps_to_string(lib, s_true(lib), JSON_ENCODE_ANY),
            dumps_to_string(lib, s_false(lib), JSON_ENCODE_ANY),
            dumps_to_string(lib, s_null(lib), JSON_ENCODE_ANY),
        )
    });
}

// ================================================================ rows 268-269

#[test]
fn rows268_269_equal() {
    diff("rows268_269/json_equal", |lib: &Library| unsafe {
        let eq: Symbol<FnEqual> = sym(lib, "json_equal");
        let mut out = Vec::new();

        // same pointer, NULL args, type mismatch
        let i1 = n_int(lib, 1);
        let r1 = n_real(lib, 1.0);
        out.push(("same-ptr", eq(i1, i1)));
        out.push(("null-left", eq(std::ptr::null(), i1)));
        out.push(("null-right", eq(i1, std::ptr::null())));
        out.push(("null-both", eq(std::ptr::null(), std::ptr::null())));
        out.push(("int-vs-real", eq(i1, r1)));
        out.push(("real-vs-int", eq(r1, i1)));

        // integers
        let i2 = n_int(lib, 1);
        let i3 = n_int(lib, 2);
        let imin = n_int(lib, i64::MIN);
        let imin2 = n_int(lib, i64::MIN);
        out.push(("int-eq", eq(i1, i2)));
        out.push(("int-ne", eq(i1, i3)));
        out.push(("intmin-eq", eq(imin, imin2)));

        // reals, incl. 0.0 vs -0.0 (IEEE ==) and equal magnitudes
        let z = n_real(lib, 0.0);
        let nz = n_real(lib, -0.0);
        let h = n_real(lib, 0.5);
        let h2 = n_real(lib, 0.5);
        out.push(("real-zero-negzero", eq(z, nz)));
        out.push(("real-eq", eq(h, h2)));
        out.push(("real-ne", eq(h, z)));

        // strings, NUL-safe
        let s1 = n_strn(lib, b"a\0b");
        let s2 = n_strn(lib, b"a\0b");
        let s3 = n_strn(lib, b"a\0c");
        let s4 = n_strn(lib, b"a");
        out.push(("str-nul-eq", eq(s1, s2)));
        out.push(("str-nul-ne", eq(s1, s3)));
        out.push(("str-prefix-ne", eq(s1, s4)));
        out.push(("str-empty", eq(n_strn(lib, b""), n_strn(lib, b""))));

        // singletons
        out.push(("true-true", eq(s_true(lib), s_true(lib))));
        out.push(("true-false", eq(s_true(lib), s_false(lib))));
        out.push(("null-null-singleton", eq(s_null(lib), s_null(lib))));

        // arrays: order sensitive
        let a1 = n_array(lib);
        let a2 = n_array(lib);
        let a3 = n_array(lib);
        for v in [1i64, 2, 3] {
            aappend(lib, a1, n_int(lib, v));
            aappend(lib, a2, n_int(lib, v));
        }
        for v in [3i64, 2, 1] {
            aappend(lib, a3, n_int(lib, v));
        }
        let a4 = n_array(lib);
        aappend(lib, a4, n_int(lib, 1));
        out.push(("arr-eq", eq(a1, a2)));
        out.push(("arr-order", eq(a1, a3)));
        out.push(("arr-size", eq(a1, a4)));
        out.push(("arr-empty", eq(n_array(lib), n_array(lib))));

        // objects: size then per-key, order INSENSITIVE
        let o1 = n_object(lib);
        let o2 = n_object(lib);
        let o3 = n_object(lib);
        let o4 = n_object(lib);
        oset(lib, o1, "a", n_int(lib, 1));
        oset(lib, o1, "b", n_str(lib, "x"));
        oset(lib, o2, "b", n_str(lib, "x"));
        oset(lib, o2, "a", n_int(lib, 1));
        oset(lib, o3, "a", n_int(lib, 1));
        oset(lib, o3, "b", n_str(lib, "y"));
        oset(lib, o4, "a", n_int(lib, 1));
        out.push(("obj-eq-reordered", eq(o1, o2)));
        out.push(("obj-value-ne", eq(o1, o3)));
        out.push(("obj-size-ne", eq(o1, o4)));
        out.push(("obj-empty", eq(n_object(lib), n_object(lib))));

        // nested
        let n1 = n_object(lib);
        let n2 = n_object(lib);
        for o in [n1, n2] {
            let inner = n_array(lib);
            aappend(lib, inner, n_int(lib, 1));
            let io = n_object(lib);
            oset(lib, io, "deep", n_str(lib, "v"));
            aappend(lib, inner, io);
            oset(lib, o, "arr", inner);
        }
        out.push(("nested-eq", eq(n1, n2)));
        oset(lib, n2, "extra", s_null(lib));
        out.push(("nested-ne", eq(n1, n2)));

        // objects with NUL keys
        let k1 = n_object(lib);
        let k2 = n_object(lib);
        osetn(lib, k1, b"a\0b", 3, n_int(lib, 1));
        osetn(lib, k2, b"a\0b", 3, n_int(lib, 1));
        out.push(("objkey-nul-eq", eq(k1, k2)));
        let k3 = n_object(lib);
        osetn(lib, k3, b"a\0c", 3, n_int(lib, 1));
        out.push(("objkey-nul-ne", eq(k1, k3)));

        out
    });
}

// ================================================================ rows 270-272

#[test]
fn rows270_272_copy_and_deep_copy() {
    // row 270: shallow copy shares children
    diff("row270/json_copy", |lib: &Library| unsafe {
        let cp: Symbol<FnCopy> = sym(lib, "json_copy");

        // object
        let o = n_object(lib);
        let child = n_array(lib);
        aappend(lib, child, n_int(lib, 1));
        oset(lib, o, "c", child);
        osetn(lib, o, b"k\0k", 3, n_str(lib, "nul-key"));
        for i in 0..10 {
            oset(lib, o, &format!("f{}", i), n_int(lib, i));
        }
        let child_rc_before = rc(child);
        let oc = cp(o);
        let obj_case = (
            view(lib, o),
            view(lib, oc),
            key_order(lib, o),
            key_order(lib, oc),
            child_rc_before,
            rc(child),
            // SAME pointer => shared child
            oget(lib, oc, "c") == oget(lib, o, "c"),
            oc == o,
        );
        decref(lib, oc);
        let after_copy_dropped = rc(child);
        decref(lib, o);

        // array
        let a = n_array(lib);
        let ac_child = n_object(lib);
        oset(lib, ac_child, "x", n_int(lib, 1));
        aappend(lib, a, ac_child);
        aappend(lib, a, n_str(lib, "s"));
        let ac = cp(a);
        let arr_case = (
            view(lib, a),
            view(lib, ac),
            rc(ac_child),
            aget(lib, ac, 0) == aget(lib, a, 0),
            ac == a,
        );
        decref(lib, ac);
        decref(lib, a);

        // scalars: fresh objects with the same content
        let s = n_strn(lib, b"a\0b");
        let sc = cp(s);
        let i = n_int(lib, i64::MIN);
        let ic = cp(i);
        let r = n_real(lib, -0.0);
        let rc_ = cp(r);
        let scalars = (
            (view(lib, sc), str_bytes(lib, sc), sc == s),
            (view(lib, ic), ic == i),
            (view(lib, rc_), rc_ == r),
        );
        decref(lib, sc);
        decref(lib, s);
        decref(lib, ic);
        decref(lib, i);
        decref(lib, rc_);
        decref(lib, r);

        // singletons return the SAME pointer, NULL returns NULL
        let singles = (
            cp(s_true(lib)) == s_true(lib),
            cp(s_false(lib)) == s_false(lib),
            cp(s_null(lib)) == s_null(lib),
            rc(cp(s_true(lib))),
            cp(std::ptr::null_mut()).is_null(),
        );

        (obj_case, after_copy_dropped, arr_case, scalars, singles)
    });

    // row 271: deep copy does not share children
    diff("row271/json_deep_copy", |lib: &Library| unsafe {
        let dc: Symbol<FnDeepCopy> = sym(lib, "json_deep_copy");
        let eq: Symbol<FnEqual> = sym(lib, "json_equal");

        let root = n_object(lib);
        let arr = n_array(lib);
        for i in 0..3 {
            let inner = n_object(lib);
            oset(lib, inner, "i", n_int(lib, i));
            oset(lib, inner, "s", n_strn(lib, b"x\0y"));
            aappend(lib, arr, inner);
        }
        aappend(lib, arr, s_true(lib));
        aappend(lib, arr, n_real(lib, -0.0));
        oset(lib, root, "arr", arr);
        osetn(lib, root, b"n\0k", 3, n_str(lib, "v"));
        for i in 0..9 {
            oset(lib, root, &format!("p{}", i), n_int(lib, i));
        }
        // a value shared twice in the source becomes two distinct copies
        let shared = n_str(lib, "shared");
        incref(shared);
        oset(lib, root, "s1", shared);
        incref(shared);
        oset(lib, root, "s2", shared);

        let copy = dc(root);
        let out = (
            (view(lib, root), view(lib, copy)),
            (key_order(lib, root), key_order(lib, copy)),
            (
                eq(root, copy),
                copy == root,
                oget(lib, copy, "arr") == oget(lib, root, "arr"),
                oget(lib, copy, "s1") == oget(lib, root, "s1"),
                oget(lib, copy, "s1") == oget(lib, copy, "s2"),
            ),
            (
                rc(shared),
                rc(oget(lib, copy, "s1")),
                rc(oget(lib, copy, "arr")),
            ),
            // scalars and singletons
            (
                dc(s_true(lib)) == s_true(lib),
                dc(s_null(lib)) == s_null(lib),
                dc(std::ptr::null()).is_null(),
            ),
        );
        decref(lib, copy);
        decref(lib, shared);
        decref(lib, root);
        out
    });

    // row 272: indirect cycle rejected
    diff("row272/deep_copy cycle", |lib: &Library| unsafe {
        let dc: Symbol<FnDeepCopy> = sym(lib, "json_deep_copy");
        // object -> array -> object
        let o = n_object(lib);
        let a = n_array(lib);
        incref(a);
        oset(lib, o, "a", a);
        incref(o);
        aappend(lib, a, o);
        let r_o = dc(o).is_null();
        let r_a = dc(a).is_null();

        // object -> object -> object
        let p = n_object(lib);
        let q = n_object(lib);
        incref(q);
        oset(lib, p, "q", q);
        incref(p);
        oset(lib, q, "p", p);
        let r_p = dc(p).is_null();

        // a DAG (shared but acyclic) must still copy fine
        let d = n_object(lib);
        let leaf = n_array(lib);
        aappend(lib, leaf, n_int(lib, 1));
        incref(leaf);
        oset(lib, d, "l1", leaf);
        incref(leaf);
        oset(lib, d, "l2", leaf);
        let dcopy = dc(d);
        let dag = (view(lib, dcopy), dcopy.is_null());

        let out = (r_o, r_a, r_p, dag);
        decref(lib, dcopy);
        decref(lib, leaf);
        decref(lib, d);
        // break both cycles
        aremove(lib, a, 0);
        odel(lib, o, "a");
        odel(lib, q, "p");
        odel(lib, p, "q");
        decref(lib, a);
        decref(lib, o);
        decref(lib, q);
        decref(lib, p);
        out
    });
}

// ================================================================ row 273

#[test]
fn row273_delete_and_decref() {
    diff("row273/json_delete on each heap type", |lib: &Library| unsafe {
        // Build one of each heap type with refcount 1 and free it via json_delete
        // directly. Any double free / missing free shows up as a crash or as an
        // ASAN-style abort, so surviving is itself the observation.
        let o = n_object(lib);
        for i in 0..12 {
            oset(lib, o, &format!("k{}", i), n_int(lib, i));
        }
        del(lib, o);
        let a = n_array(lib);
        for _ in 0..12 {
            aappend(lib, a, n_str(lib, "v"));
        }
        del(lib, a);
        del(lib, n_strn(lib, b"a\0b"));
        del(lib, n_int(lib, 5));
        del(lib, n_real(lib, 1.5));
        // NULL is a no-op
        del(lib, std::ptr::null_mut());
        1u8
    });

    diff("row273/decref chain", |lib: &Library| unsafe {
        // refcount bookkeeping through a shared subtree
        let leaf = n_int(lib, 1);
        let a = n_array(lib);
        incref(leaf);
        aappend(lib, a, leaf);
        let b = n_array(lib);
        incref(leaf);
        aappend(lib, b, leaf);
        let rc0 = rc(leaf);
        decref(lib, a);
        let rc1 = rc(leaf);
        decref(lib, b);
        let rc2 = rc(leaf);
        let ty = (*leaf).type_;
        decref(lib, leaf);
        (rc0, rc1, rc2, ty)
    });
}

// ================================================================ randomized sequence

/// Op codes recorded in the randomized-mutation log.
const OPS: usize = 13;

#[test]
fn rows224_256_randomized_mutation_sequence() {
    // Keys chosen to cover hashlittle's tail cases (len 0,1,2,4,11,12,13,25)
    // and to collide often enough that deletes actually hit.
    const KEYS: &[&str] = &[
        "",
        "a",
        "bb",
        "abcd",
        "0123456789a",
        "0123456789ab",
        "0123456789abc",
        "0123456789abcdef012345678",
        "k1",
        "k2",
        "zz",
        "dup",
    ];

    diff_n("rows224-256/randomized mutation sequence", 320, |lib: &Library, it: u64| unsafe {
        let mut rng = Rng::new(0xB10C_0000 ^ it.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let clear_o: Symbol<FnClear> = sym(lib, "json_object_clear");
        let clear_a: Symbol<FnClear> = sym(lib, "json_array_clear");
        let ext: Symbol<FnTwoJson> = sym(lib, "json_array_extend");
        let upd: Symbol<FnTwoJson> = sym(lib, "json_object_update");
        let upd_e: Symbol<FnTwoJson> = sym(lib, "json_object_update_existing");
        let upd_m: Symbol<FnTwoJson> = sym(lib, "json_object_update_missing");
        let upd_r: Symbol<FnTwoJson> = sym(lib, "json_object_update_recursive");

        let obj = n_object(lib);
        let arr = n_array(lib);
        let mut log: Vec<(u64, i64, usize, usize, Option<String>, Option<String>, Option<String>)> =
            Vec::new();

        for step in 0..26u64 {
            let op = rng.below(OPS as u64);
            let ret: i64 = match op {
                0 => {
                    // object set_new, key from the pool
                    let k = KEYS[rng.below(KEYS.len() as u64) as usize];
                    oset(lib, obj, k, n_int(lib, step as i64)) as i64
                }
                1 => {
                    // object set_new, randomized key, randomized scalar value
                    let n = rng.below(30) as usize;
                    let k = key_of_len(&mut rng, n);
                    let v = match rng.below(4) {
                        0 => n_int(lib, rng.i64()),
                        1 => n_str(lib, &rng.ascii_string(6)),
                        2 => s_null(lib),
                        _ => n_real(lib, 1.0 / (1.0 + (rng.below(1000) as f64))),
                    };
                    osetn(lib, obj, &k, k.len(), v) as i64
                }
                2 => {
                    let k = KEYS[rng.below(KEYS.len() as u64) as usize];
                    odel(lib, obj, k) as i64
                }
                3 => clear_o(obj) as i64,
                4 => aappend(lib, arr, n_int(lib, step as i64)) as i64,
                5 => {
                    let n = asize(lib, arr);
                    let i = rng.below(n as u64 + 3) as usize;
                    ainsert(lib, arr, i, n_str(lib, &rng.ascii_string(4))) as i64
                }
                6 => {
                    let n = asize(lib, arr);
                    let i = rng.below(n as u64 + 3) as usize;
                    aremove(lib, arr, i) as i64
                }
                7 => {
                    let n = asize(lib, arr);
                    let i = rng.below(n as u64 + 3) as usize;
                    aset(lib, arr, i, n_int(lib, -(step as i64))) as i64
                }
                8 => clear_a(arr) as i64,
                9 => {
                    // extend with a freshly built other array
                    let other = n_array(lib);
                    let m = rng.below(22);
                    for j in 0..m {
                        aappend(lib, other, n_int(lib, 1000 + j as i64));
                    }
                    let r = ext(arr, other) as i64;
                    decref(lib, other);
                    r
                }
                10 => {
                    // one of the four update flavours with a small other object
                    let other = n_object(lib);
                    let m = rng.below(5) + 1;
                    for _ in 0..m {
                        let k = KEYS[rng.below(KEYS.len() as u64) as usize];
                        oset(lib, other, k, n_int(lib, rng.i64() & 0xffff));
                    }
                    if rng.below(2) == 0 {
                        let nested = n_object(lib);
                        oset(lib, nested, "in", n_int(lib, step as i64));
                        oset(lib, other, "dup", nested);
                    }
                    let r = match rng.below(4) {
                        0 => upd(obj, other),
                        1 => upd_e(obj, other),
                        2 => upd_m(obj, other),
                        _ => upd_r(obj, other),
                    } as i64;
                    decref(lib, other);
                    r
                }
                11 => {
                    // nest a fresh object/array under a pool key
                    let k = KEYS[rng.below(KEYS.len() as u64) as usize];
                    let v = if rng.below(2) == 0 {
                        let nested = n_object(lib);
                        oset(lib, nested, "x", n_int(lib, step as i64));
                        nested
                    } else {
                        let nested = n_array(lib);
                        aappend(lib, nested, n_int(lib, step as i64));
                        nested
                    };
                    oset(lib, obj, k, v) as i64
                }
                _ => {
                    // put a shared reference to `arr` in the object (no cycle)
                    incref(arr);
                    oset(lib, obj, "arrref", arr) as i64
                }
            };

            log.push((
                op,
                ret,
                osize(lib, obj),
                asize(lib, arr),
                dumps_to_string(lib, obj, 0),
                dumps_to_string(lib, obj, JSON_SORT_KEYS),
                dumps_to_string(lib, arr, JSON_COMPACT),
            ));
        }

        let tail = (key_order(lib, obj), view(lib, obj), view(lib, arr));
        decref(lib, arr);
        decref(lib, obj);
        (log, tail)
    });
}
