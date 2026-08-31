//! Differential tests for src/value.c — the value model itself.
//!
//! Coverage target: CONFIGS.md rows 164-226.
//!
//! Every comparison uses a *full* observable snapshot of the value (see `Snap`):
//! type tag, `refcount`, container sizes, the internal capacity/order fields,
//! scalar payloads (integers exactly, reals as bit patterns so `-0.0` stays
//! distinct), both canonical dumps (`JSON_SORT_KEYS|JSON_ENCODE_ANY` and the
//! insertion-order `JSON_ENCODE_ANY`) and the complete ordered child list
//! (object: key bytes + key_len + per-child dump; array: element list).
//!
//! The object iteration order is the fingerprint of the hashtable behaviour, so
//! it is compared explicitly everywhere rather than only via the sorted dump.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Raw private layouts (jansson_private.h) so tests can observe the capacity /
// hashtable order fields that are part of the documented growth rules.
// ---------------------------------------------------------------------------

#[repr(C)]
struct json_array_raw {
    json: json_t,
    size: size_t,
    entries: size_t,
    table: *mut *mut json_t,
}

#[repr(C)]
struct json_object_raw {
    json: json_t,
    hashtable: hashtable_t,
}

unsafe fn arr_cap(j: *const json_t) -> size_t {
    if j.is_null() || typeof_(j) != JSON_ARRAY {
        return 0;
    }
    (*(j as *const json_array_raw)).size
}

unsafe fn obj_order(j: *const json_t) -> size_t {
    if j.is_null() || typeof_(j) != JSON_OBJECT {
        return 0;
    }
    (*(j as *const json_object_raw)).hashtable.order
}

// ---------------------------------------------------------------------------
// Byte strings that print readably in divergence messages.
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone)]
struct B(Vec<u8>);

impl std::fmt::Debug for B {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "b\"")?;
        for &x in &self.0 {
            match x {
                b'"' => write!(f, "\\\"")?,
                b'\\' => write!(f, "\\\\")?,
                0x20..=0x7e => write!(f, "{}", x as char)?,
                _ => write!(f, "\\x{x:02x}")?,
            }
        }
        write!(f, "\"")
    }
}

const CANON: size_t = JSON_SORT_KEYS | JSON_ENCODE_ANY;
const RAWD: size_t = JSON_ENCODE_ANY;

/// `json_dumps` as raw bytes, freed with the matching allocator. `None` means
/// the dump failed (NULL), which is itself an observable to compare.
unsafe fn dump(api: &Api, j: *const json_t, flags: size_t) -> Option<B> {
    if j.is_null() {
        return None;
    }
    let p = (api.json_dumps)(j, flags);
    let b = cbytes(p).map(B);
    jfree(api, p as *mut c_void);
    b
}

/// Both canonical forms: sorted keys (structural) and raw insertion order.
unsafe fn canon(api: &Api, j: *const json_t) -> (Option<B>, Option<B>) {
    (dump(api, j, CANON), dump(api, j, RAWD))
}

/// One child of a container. The dumps are the structural fingerprint, but they
/// are NULL whenever the subtree holds invalid UTF-8, so the scalar payloads are
/// recorded separately and compared byte-exactly as well.
#[derive(Debug, PartialEq, Clone)]
struct Kid {
    key: Option<B>,
    key_len: size_t,
    ty: c_int,
    refcount: size_t,
    osize: size_t,
    asize: size_t,
    slen: size_t,
    sbytes: Option<B>,
    ival: i64,
    rbits: u64,
    sorted: Option<B>,
    raw: Option<B>,
}

unsafe fn kid(api: &Api, key: Option<B>, klen: size_t, v: *mut json_t) -> Kid {
    if v.is_null() {
        return Kid {
            key,
            key_len: klen,
            ty: -1,
            refcount: 0,
            osize: 0,
            asize: 0,
            slen: 0,
            sbytes: None,
            ival: 0,
            rbits: 0,
            sorted: None,
            raw: None,
        };
    }
    let (s, w) = canon(api, v);
    let slen = (api.json_string_length)(v);
    let sp = (api.json_string_value)(v);
    Kid {
        key,
        key_len: klen,
        ty: typeof_(v),
        refcount: (*v).refcount,
        osize: (api.json_object_size)(v),
        asize: (api.json_array_size)(v),
        slen,
        sbytes: if sp.is_null() {
            None
        } else {
            Some(B((0..slen).map(|i| *(sp as *const u8).add(i)).collect()))
        },
        ival: (api.json_integer_value)(v),
        rbits: (api.json_real_value)(v).to_bits(),
        sorted: s,
        raw: w,
    }
}

/// Everything observable about a value, one level deep (children summarised by
/// their own dumps, so cyclic graphs cannot blow the stack — `json_dumps`
/// itself rejects cycles and returns NULL in both libraries).
#[derive(Debug, PartialEq, Clone)]
struct Snap {
    ty: c_int,
    refcount: size_t,
    osize: size_t,
    asize: size_t,
    order: size_t,
    cap: size_t,
    slen: size_t,
    sbytes: Option<B>,
    ival: i64,
    rbits: u64,
    nbits: u64,
    sorted: Option<B>,
    raw: Option<B>,
    kids: Vec<Kid>,
}

unsafe fn snap(api: &Api, j: *mut json_t) -> Snap {
    let ty = if j.is_null() { -1 } else { typeof_(j) };
    let refcount = if j.is_null() { 0 } else { (*j).refcount };
    let slen = (api.json_string_length)(j);
    let sp = (api.json_string_value)(j);
    let sbytes = if sp.is_null() {
        None
    } else {
        Some(B((0..slen).map(|i| *(sp as *const u8).add(i)).collect()))
    };
    let (sorted, raw) = canon(api, j);

    let mut kids: Vec<Kid> = Vec::new();
    if ty == JSON_OBJECT {
        let mut it = (api.json_object_iter)(j);
        while !it.is_null() {
            let kp = (api.json_object_iter_key)(it);
            let kl = (api.json_object_iter_key_len)(it);
            let key = if kp.is_null() {
                None
            } else {
                Some(B((0..kl).map(|i| *(kp as *const u8).add(i)).collect()))
            };
            let v = (api.json_object_iter_value)(it);
            kids.push(kid(api, key, kl, v));
            it = (api.json_object_iter_next)(j, it);
        }
    } else if ty == JSON_ARRAY {
        let n = (api.json_array_size)(j);
        for i in 0..n {
            kids.push(kid(api, None, i, (api.json_array_get)(j, i)));
        }
    }

    Snap {
        ty,
        refcount,
        osize: (api.json_object_size)(j),
        asize: (api.json_array_size)(j),
        order: obj_order(j),
        cap: arr_cap(j),
        slen,
        sbytes,
        ival: (api.json_integer_value)(j),
        rbits: (api.json_real_value)(j).to_bits(),
        nbits: (api.json_number_value)(j).to_bits(),
        sorted,
        raw,
        kids,
    }
}

/// Compare two values produced by the two libraries.
unsafe fn cmp(c: &Api, r: &Api, cj: *mut json_t, rj: *mut json_t, ctx: &str) {
    diff_eq!(cj.is_null(), rj.is_null(), "{}: NULL-ness of result", ctx);
    diff_eq!(snap(c, cj), snap(r, rj), "{}", ctx);
}

/// Compare, then release both values.
unsafe fn cmp_free(c: &Api, r: &Api, cj: *mut json_t, rj: *mut json_t, ctx: &str) {
    cmp(c, r, cj, rj, ctx);
    decref(c, cj);
    decref(r, rj);
}

/// Count pointer identities shared between two trees, ignoring the three
/// singletons (which are *always* shared, by design).
unsafe fn count_shared(api: &Api, a: *mut json_t, b: *mut json_t) -> usize {
    if a.is_null() || b.is_null() {
        return 0;
    }
    let t = typeof_(a);
    let mut n = 0usize;
    if a == b && t != JSON_TRUE && t != JSON_FALSE && t != JSON_NULL {
        n += 1;
    }
    if t == JSON_OBJECT && typeof_(b) == JSON_OBJECT {
        let mut it = (api.json_object_iter)(a);
        while !it.is_null() {
            let kp = (api.json_object_iter_key)(it);
            let kl = (api.json_object_iter_key_len)(it);
            let av = (api.json_object_iter_value)(it);
            let bv = (api.json_object_getn)(b, kp, kl);
            n += count_shared(api, av, bv);
            it = (api.json_object_iter_next)(a, it);
        }
    } else if t == JSON_ARRAY && typeof_(b) == JSON_ARRAY {
        let m = (api.json_array_size)(a).min((api.json_array_size)(b));
        for i in 0..m {
            n += count_shared(api, (api.json_array_get)(a, i), (api.json_array_get)(b, i));
        }
    }
    n
}

// ---------------------------------------------------------------------------
// Value "recipes": build byte-identical trees in both libraries.
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
enum V {
    Obj(Vec<(Vec<u8>, V)>),
    Arr(Vec<V>),
    Str(Vec<u8>),
    Int(i64),
    Real(f64),
    True,
    False,
    Null,
}

unsafe fn build(api: &Api, v: &V) -> *mut json_t {
    match v {
        V::Obj(kids) => {
            let o = (api.json_object)();
            for (k, sub) in kids {
                let child = build(api, sub);
                (api.json_object_setn_new_nocheck)(o, k.as_ptr() as *const c_char, k.len(), child);
            }
            o
        }
        V::Arr(kids) => {
            let a = (api.json_array)();
            for sub in kids {
                (api.json_array_append_new)(a, build(api, sub));
            }
            a
        }
        V::Str(b) => (api.json_stringn_nocheck)(b.as_ptr() as *const c_char, b.len()),
        V::Int(n) => (api.json_integer)(*n),
        V::Real(x) => (api.json_real)(*x),
        V::True => (api.json_true)(),
        V::False => (api.json_false)(),
        V::Null => (api.json_null)(),
    }
}

/// A random scalar recipe (always dumpable: keys/strings are valid UTF-8).
fn rand_scalar(rng: &mut Rng) -> V {
    match rng.below(6) {
        0 => V::Str(rng.utf8_string(12).into_bytes()),
        1 => V::Int(rng.json_int()),
        2 => V::Real(rng.real()),
        3 => V::True,
        4 => V::False,
        _ => V::Null,
    }
}

fn rand_key(rng: &mut Rng) -> Vec<u8> {
    match rng.below(4) {
        0 => format!("k{}", rng.below(10)).into_bytes(),
        1 => rng.utf8_string(6).into_bytes(),
        2 => Vec::new(),
        _ => rng.ascii_string(5).into_bytes(),
    }
}

fn rand_value(rng: &mut Rng, depth: usize) -> V {
    if depth == 0 || rng.below(3) == 0 {
        return rand_scalar(rng);
    }
    if rng.bool() {
        let n = rng.below(6);
        V::Obj((0..n).map(|_| (rand_key(rng), rand_value(rng, depth - 1))).collect())
    } else {
        let n = rng.below(6);
        V::Arr((0..n).map(|_| rand_value(rng, depth - 1)).collect())
    }
}

/// All eight types, in type-tag order.
fn all_eight() -> Vec<(&'static str, V)> {
    vec![
        ("object", V::Obj(vec![(b"in".to_vec(), V::Int(1))])),
        ("array", V::Arr(vec![V::Int(1), V::Str(b"x".to_vec())])),
        ("string", V::Str(b"str".to_vec())),
        ("integer", V::Int(-42)),
        ("real", V::Real(1.5)),
        ("true", V::True),
        ("false", V::False),
        ("null", V::Null),
    ]
}

// A caller-allocated hashtable_t, for the direct `jsonp_loop_check` /
// `do_*_recursive` entry points.
struct Ht<'a> {
    api: &'a Api,
    t: Box<hashtable_t>,
}

impl<'a> Ht<'a> {
    unsafe fn new(api: &'a Api) -> Ht<'a> {
        let mut t = Box::new(hashtable_t::zeroed());
        assert_eq!((api.hashtable_init)(&mut *t), 0);
        Ht { api, t }
    }
    fn p(&mut self) -> *mut hashtable_t {
        &mut *self.t
    }
}

impl Drop for Ht<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.hashtable_close)(&mut *self.t) }
    }
}

// ===========================================================================
// Row 164 / 165 / 166 — construction and initial state
// ===========================================================================

#[test]
fn row164_json_object_fresh_state() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for i in 0..8 {
            let cj = (c.json_object)();
            let rj = (r.json_object)();
            assert!(!cj.is_null() && !rj.is_null());
            // C ground truth: type tag, refcount 1, size 0, order 3 (8 buckets).
            assert_eq!(typeof_(cj), JSON_OBJECT, "C: fresh object type");
            assert_eq!((*cj).refcount, 1, "C: fresh object refcount");
            assert_eq!((c.json_object_size)(cj), 0, "C: fresh object size");
            assert_eq!(obj_order(cj), 3, "C: INITIAL_HASHTABLE_ORDER");
            cmp_free(c, r, cj, rj, &format!("json_object() #{i}"));
        }
    }
}

#[test]
fn row165_seed_is_deterministic_and_one_shot() {
    let _g = global_state_lock();
    let (c, r) = both();
    // `both()` already installed FIXED_SEED, so the autoseed branch in
    // json_object() (`if (!hashtable_seed) json_object_seed(0)`) is skipped;
    // both libraries must agree on the seed and hence on iteration order.
    diff_eq!(c.hashtable_seed(), r.hashtable_seed(), "hashtable_seed");
    assert_ne!(c.hashtable_seed(), 0, "C: seed installed, autoseed skipped");
    unsafe {
        // A later json_object_seed() is a no-op (seed already non-zero).
        (c.json_object_seed)(12345);
        (r.json_object_seed)(12345);
        diff_eq!(c.hashtable_seed(), r.hashtable_seed(), "seed after re-seed");
        assert_eq!(c.hashtable_seed(), FIXED_SEED as u32, "C: seed unchanged");

        // The observable consequence: same keys -> same iteration order.
        let mut rng = Rng::new(0x0165_0001);
        for trial in 0..30 {
            let n = 1 + rng.below(20);
            let keys: Vec<Vec<u8>> = (0..n).map(|_| rand_key(&mut rng)).collect();
            let cj = (c.json_object)();
            let rj = (r.json_object)();
            for (i, k) in keys.iter().enumerate() {
                (c.json_object_setn_new_nocheck)(
                    cj,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    (c.json_integer)(i as i64),
                );
                (r.json_object_setn_new_nocheck)(
                    rj,
                    k.as_ptr() as *const c_char,
                    k.len(),
                    (r.json_integer)(i as i64),
                );
            }
            cmp_free(c, r, cj, rj, &format!("seeded iteration order trial {trial}"));
        }
    }
}

#[test]
fn row166_json_array_fresh_state() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for i in 0..8 {
            let cj = (c.json_array)();
            let rj = (r.json_array)();
            assert_eq!(typeof_(cj), JSON_ARRAY, "C: fresh array type");
            assert_eq!((*cj).refcount, 1, "C: fresh array refcount");
            assert_eq!((c.json_array_size)(cj), 0, "C: fresh array entries");
            assert_eq!(arr_cap(cj), 8, "C: initial capacity is 8");
            cmp_free(c, r, cj, rj, &format!("json_array() #{i}"));
        }
    }
}

// ===========================================================================
// Rows 167-171 — strings
// ===========================================================================

/// Invalid UTF-8 byte strings (row 168 / row 170's "must succeed" list).
fn bad_utf8() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("lone continuation", vec![0x80]),
        ("truncated 2-byte", vec![0xC3]),
        ("overlong C0 AF", vec![0xC0, 0xAF]),
        ("overlong C0 80", vec![0xC0, 0x80]),
        ("surrogate ED A0 80", vec![0xED, 0xA0, 0x80]),
        ("5-byte F8", vec![0xF8, 0x88, 0x80, 0x80, 0x80]),
        ("0xFF", vec![0xFF]),
        ("bad continuation C3 41", vec![0xC3, 0x41]),
        ("truncated 3-byte E2 82", vec![0xE2, 0x82]),
        ("above U+10FFFF F5 90 80 80", vec![0xF5, 0x90, 0x80, 0x80]),
    ]
}

#[test]
fn row167_json_string_valid_inputs() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0167_0001);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"a".to_vec(),
            b"hello".to_vec(),
            vec![b'x'; 1024],
            "héllo".as_bytes().to_vec(),
            "日本語".as_bytes().to_vec(),
            vec![0xF0, 0x9F, 0x98, 0x80],
            "\u{7f}\u{80}\u{7ff}\u{800}\u{ffff}\u{10000}\u{10ffff}".as_bytes().to_vec(),
        ];
        // Many randomised valid UTF-8 inputs.
        for _ in 0..250 {
            cases.push(rng.utf8_string(40).into_bytes());
        }

        for (i, bytes) in cases.iter().enumerate() {
            let buf = cs_bytes(bytes);
            // json_string() uses strlen(), so a NUL-free buffer is required for
            // the length to equal bytes.len(); rng.utf8_string can emit NULs, so
            // compute the strlen-visible prefix for the assertion.
            let vis = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let cj = (c.json_string)(buf.as_ptr());
            let rj = (r.json_string)(buf.as_ptr());
            let ctx = format!("json_string(case {i}, {:?})", B(bytes.clone()));
            cmp(c, r, cj, rj, &ctx);
            if !cj.is_null() {
                assert_eq!(
                    (c.json_string_length)(cj),
                    vis,
                    "C: json_string_length == strlen for case {i}"
                );
            }
            decref(c, cj);
            decref(r, rj);

            // json_stringn() with the true length also succeeds (NULs are valid
            // UTF-8), and preserves the full byte string.
            let cj = (c.json_stringn)(buf.as_ptr(), bytes.len());
            let rj = (r.json_stringn)(buf.as_ptr(), bytes.len());
            cmp_free(c, r, cj, rj, &format!("json_stringn(case {i}, full len)"));
        }
    }
}

#[test]
fn row168_json_string_rejects_invalid_utf8() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for (name, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            let cj = (c.json_string)(buf.as_ptr());
            let rj = (r.json_string)(buf.as_ptr());
            diff_eq!(cj.is_null(), rj.is_null(), "json_string({name})");
            assert!(cj.is_null(), "C: json_string must reject {name}");
            decref(c, cj);
            decref(r, rj);

            let cj = (c.json_stringn)(buf.as_ptr(), bytes.len());
            let rj = (r.json_stringn)(buf.as_ptr(), bytes.len());
            diff_eq!(cj.is_null(), rj.is_null(), "json_stringn({name})");
            assert!(cj.is_null(), "C: json_stringn must reject {name}");
            decref(c, cj);
            decref(r, rj);
        }
        // value == NULL
        let cj = (c.json_string)(std::ptr::null());
        let rj = (r.json_string)(std::ptr::null());
        diff_eq!(cj.is_null(), rj.is_null(), "json_string(NULL)");
        assert!(cj.is_null(), "C: json_string(NULL) == NULL");
        let cj = (c.json_stringn)(std::ptr::null(), 0);
        let rj = (r.json_stringn)(std::ptr::null(), 0);
        diff_eq!(cj.is_null(), rj.is_null(), "json_stringn(NULL, 0)");
        let cj = (c.json_stringn)(std::ptr::null(), 5);
        let rj = (r.json_stringn)(std::ptr::null(), 5);
        diff_eq!(cj.is_null(), rj.is_null(), "json_stringn(NULL, 5)");
    }
}

#[test]
fn row169_json_stringn_length_variants() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0169_0001);
    unsafe {
        // (buffer, len) pairs covering truncation, len==0, embedded NUL and
        // lengths that cut a multi-byte sequence in half.
        let mut cases: Vec<(Vec<u8>, size_t)> = vec![
            (b"abcdef".to_vec(), 3),
            (b"abcdef".to_vec(), 0),
            (b"a\0b".to_vec(), 3),
            (b"a\0b".to_vec(), 2),
            (b"\0".to_vec(), 1),
            ("héllo".as_bytes().to_vec(), 2), // cuts U+00E9 in half
            ("日本語".as_bytes().to_vec(), 1),
            ("日本語".as_bytes().to_vec(), 2),
            ("日本語".as_bytes().to_vec(), 4),
            (vec![0xF0, 0x9F, 0x98, 0x80], 1),
            (vec![0xF0, 0x9F, 0x98, 0x80], 3),
            (vec![0xF0, 0x9F, 0x98, 0x80], 4),
        ];
        // Randomised: a valid UTF-8 string truncated at every possible offset.
        for _ in 0..120 {
            let s = rng.utf8_string(16).into_bytes();
            let cut = rng.below(s.len() + 1);
            cases.push((s, cut));
        }

        for (i, (bytes, len)) in cases.iter().enumerate() {
            let buf = cs_bytes(bytes);
            let ctx = format!("json_stringn(case {i}, {:?}, len={len})", B(bytes.clone()));
            let cj = (c.json_stringn)(buf.as_ptr(), *len);
            let rj = (r.json_stringn)(buf.as_ptr(), *len);
            cmp_free(c, r, cj, rj, &ctx);
            // The _nocheck sibling must accept every one of these.
            let cj = (c.json_stringn_nocheck)(buf.as_ptr(), *len);
            let rj = (r.json_stringn_nocheck)(buf.as_ptr(), *len);
            assert!(!cj.is_null(), "C: json_stringn_nocheck accepts case {i}");
            cmp_free(c, r, cj, rj, &format!("json_stringn_nocheck(case {i}, len={len})"));
        }
    }
}

#[test]
fn row170_string_nocheck_accepts_everything() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0170_0001);
    unsafe {
        for (name, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            let cj = (c.json_string_nocheck)(buf.as_ptr());
            let rj = (r.json_string_nocheck)(buf.as_ptr());
            assert!(!cj.is_null(), "C: json_string_nocheck accepts {name}");
            cmp_free(c, r, cj, rj, &format!("json_string_nocheck({name})"));

            let cj = (c.json_stringn_nocheck)(buf.as_ptr(), bytes.len());
            let rj = (r.json_stringn_nocheck)(buf.as_ptr(), bytes.len());
            assert!(!cj.is_null(), "C: json_stringn_nocheck accepts {name}");
            cmp_free(c, r, cj, rj, &format!("json_stringn_nocheck({name})"));
        }

        // "" / embedded NUL with len == 3 / len == 0
        for (bytes, len) in [
            (b"".to_vec(), 0usize),
            (b"a\0b".to_vec(), 3),
            (b"abc".to_vec(), 0),
        ] {
            let buf = cs_bytes(&bytes);
            let cj = (c.json_stringn_nocheck)(buf.as_ptr(), len);
            let rj = (r.json_stringn_nocheck)(buf.as_ptr(), len);
            cmp_free(
                c,
                r,
                cj,
                rj,
                &format!("json_stringn_nocheck({:?}, {len})", B(bytes.clone())),
            );
        }

        // Randomised arbitrary byte strings (mostly invalid UTF-8).
        for i in 0..250 {
            let n = rng.below(24);
            let bytes: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
            let buf = cs_bytes(&bytes);
            let cj = (c.json_stringn_nocheck)(buf.as_ptr(), bytes.len());
            let rj = (r.json_stringn_nocheck)(buf.as_ptr(), bytes.len());
            cmp_free(
                c,
                r,
                cj,
                rj,
                &format!("json_stringn_nocheck(random #{i} {:?})", B(bytes.clone())),
            );
            let cj = (c.json_string_nocheck)(buf.as_ptr());
            let rj = (r.json_string_nocheck)(buf.as_ptr());
            cmp_free(c, r, cj, rj, &format!("json_string_nocheck(random #{i})"));
        }

        // value == NULL -> NULL for both entry points.
        let cj = (c.json_string_nocheck)(std::ptr::null());
        let rj = (r.json_string_nocheck)(std::ptr::null());
        diff_eq!(cj.is_null(), rj.is_null(), "json_string_nocheck(NULL)");
        assert!(cj.is_null(), "C: json_string_nocheck(NULL) == NULL");
        let cj = (c.json_stringn_nocheck)(std::ptr::null(), 4);
        let rj = (r.json_stringn_nocheck)(std::ptr::null(), 4);
        diff_eq!(cj.is_null(), rj.is_null(), "json_stringn_nocheck(NULL, 4)");
        assert!(cj.is_null(), "C: json_stringn_nocheck(NULL, 4) == NULL");
    }
}

#[test]
fn row171_jsonp_stringn_nocheck_own_takes_ownership() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0171_0001);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"owned".to_vec(),
            b"a\0b".to_vec(),
            vec![0xFF, 0xFE],
            vec![0xC3],
            vec![b'z'; 300],
        ];
        for _ in 0..120 {
            let n = rng.below(20);
            cases.push((0..n).map(|_| rng.next_u32() as u8).collect());
        }

        for (i, bytes) in cases.iter().enumerate() {
            let src = cs_bytes(bytes);
            // Each library must own a buffer from ITS OWN allocator, because
            // json_decref -> json_delete_string -> jsonp_free will release it.
            let cbuf = (c.jsonp_strndup)(src.as_ptr(), bytes.len());
            let rbuf = (r.jsonp_strndup)(src.as_ptr(), bytes.len());
            assert!(!cbuf.is_null() && !rbuf.is_null());
            let cj = (c.jsonp_stringn_nocheck_own)(cbuf, bytes.len());
            let rj = (r.jsonp_stringn_nocheck_own)(rbuf, bytes.len());
            // own == 1 means the pointer is adopted verbatim, no strndup.
            assert_eq!(
                (c.json_string_value)(cj) as *const c_char,
                cbuf as *const c_char,
                "C: own=1 adopts the caller's buffer"
            );
            assert_eq!(
                (r.json_string_value)(rj) as *const c_char,
                rbuf as *const c_char,
                "Rust: own=1 must adopt the caller's buffer"
            );
            cmp_free(
                c,
                r,
                cj,
                rj,
                &format!("jsonp_stringn_nocheck_own(case {i}, {:?})", B(bytes.clone())),
            );
        }

        // value == NULL -> NULL (checked before any allocation).
        let cj = (c.jsonp_stringn_nocheck_own)(std::ptr::null(), 0);
        let rj = (r.jsonp_stringn_nocheck_own)(std::ptr::null(), 0);
        diff_eq!(cj.is_null(), rj.is_null(), "jsonp_stringn_nocheck_own(NULL, 0)");
        assert!(cj.is_null(), "C: own(NULL) == NULL");
        let cj = (c.jsonp_stringn_nocheck_own)(std::ptr::null(), 7);
        let rj = (r.jsonp_stringn_nocheck_own)(std::ptr::null(), 7);
        diff_eq!(cj.is_null(), rj.is_null(), "jsonp_stringn_nocheck_own(NULL, 7)");
    }
}

// ===========================================================================
// Rows 172-177 — numbers, singletons, deletion
// ===========================================================================

#[test]
fn row172_json_integer_values() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0172_0001);
    unsafe {
        let mut vals: Vec<i64> = vec![
            0,
            1,
            -1,
            i32::MAX as i64,
            i32::MIN as i64,
            i64::MAX,
            i64::MIN,
        ];
        for _ in 0..300 {
            vals.push(rng.json_int());
        }
        for v in vals {
            let cj = (c.json_integer)(v);
            let rj = (r.json_integer)(v);
            assert_eq!(typeof_(cj), JSON_INTEGER, "C: json_integer type");
            assert_eq!((*cj).refcount, 1, "C: json_integer refcount");
            cmp_free(c, r, cj, rj, &format!("json_integer({v})"));
        }
    }
}

#[test]
fn row173_json_real_finite_values() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0173_0001);
    unsafe {
        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.5,
            f64::MIN_POSITIVE,
            f64::MAX,
            -f64::MAX,
            1e-300,
            -1e300,
            5e-324,
        ];
        for _ in 0..300 {
            vals.push(rng.real());
        }
        for v in vals {
            let cj = (c.json_real)(v);
            let rj = (r.json_real)(v);
            assert!(!cj.is_null(), "C: json_real({v:e}) must succeed");
            assert_eq!(typeof_(cj), JSON_REAL, "C: json_real type");
            assert_eq!((*cj).refcount, 1, "C: json_real refcount");
            cmp_free(c, r, cj, rj, &format!("json_real({v:e} bits={:#x})", v.to_bits()));
        }
    }
}

#[test]
fn row174_json_real_rejects_nan_and_inf() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let zero = 0.0f64;
        let bad: Vec<(&str, f64)> = vec![
            ("NAN", f64::NAN),
            ("-NAN", -f64::NAN),
            ("INFINITY", f64::INFINITY),
            ("-INFINITY", f64::NEG_INFINITY),
            ("0.0/0.0", zero / zero),
            ("1.0/0.0", 1.0 / zero),
            ("signalling nan", f64::from_bits(0x7ff0_0000_0000_0001)),
            ("quiet nan payload", f64::from_bits(0xfff8_0000_dead_beef)),
        ];
        for (name, v) in bad {
            let cj = (c.json_real)(v);
            let rj = (r.json_real)(v);
            diff_eq!(cj.is_null(), rj.is_null(), "json_real({name})");
            assert!(cj.is_null(), "C: json_real({name}) must be NULL");
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn row175_singletons_are_static_and_never_freed() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for (name, cf, rf) in [
            ("json_true", c.json_true, r.json_true),
            ("json_false", c.json_false, r.json_false),
            ("json_null", c.json_null, r.json_null),
        ] {
            let c1 = cf();
            let c2 = cf();
            let r1 = rf();
            let r2 = rf();
            // Pointer identity across calls (static storage).
            assert_eq!(c1, c2, "C: {name}() is a singleton");
            diff_eq!(c1 == c2, r1 == r2, "{name}() pointer identity");
            // refcount == (size_t)-1
            assert_eq!((*c1).refcount, usize::MAX, "C: {name} refcount is (size_t)-1");
            diff_eq!((*c1).refcount, (*r1).refcount, "{name} refcount");
            diff_eq!(typeof_(c1), typeof_(r1), "{name} type tag");

            // incref/decref must leave it alone and never free it.
            for _ in 0..5 {
                incref(c1);
                incref(r1);
                decref(c, c1);
                decref(r, r1);
            }
            diff_eq!((*c1).refcount, (*r1).refcount, "{name} refcount after in/decref");
            assert_eq!((*c1).refcount, usize::MAX, "C: {name} refcount still (size_t)-1");
            cmp(c, r, c1, r1, &format!("{name} after in/decref churn"));
        }
        // The three singletons are distinct from each other.
        assert_ne!((c.json_true)(), (c.json_false)());
        assert_ne!((c.json_true)(), (c.json_null)());
        assert_ne!((c.json_false)(), (c.json_null)());
        diff_eq!(
            ((r.json_true)() != (r.json_false)(), (r.json_true)() != (r.json_null)()),
            (true, true),
            "Rust singletons are distinct objects"
        );
    }
}

#[test]
fn row176_json_delete_every_type() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Heap types: json_delete frees them (and recursively decrefs children).
        // Nothing is observable afterwards; what is checked is that both
        // libraries take the same branch and neither crashes / double-frees.
        for (name, v) in all_eight() {
            match v {
                V::True | V::False | V::Null => continue,
                _ => {}
            }
            let cj = build(c, &v);
            let rj = build(r, &v);
            cmp(c, r, cj, rj, &format!("before json_delete({name})"));
            (c.json_delete)(cj);
            (r.json_delete)(rj);
        }
        // Empty object / empty array explicitly.
        for (name, v) in [("empty object", V::Obj(vec![])), ("empty array", V::Arr(vec![]))] {
            let cj = build(c, &v);
            let rj = build(r, &v);
            cmp(c, r, cj, rj, &format!("before json_delete({name})"));
            (c.json_delete)(cj);
            (r.json_delete)(rj);
        }
        // Deeply nested children, so the recursive decref path runs.
        let mut rng = Rng::new(0x0176_0001);
        for i in 0..40 {
            let recipe = rand_value(&mut rng, 4);
            let cj = build(c, &recipe);
            let rj = build(r, &recipe);
            cmp(c, r, cj, rj, &format!("before json_delete(random tree #{i})"));
            (c.json_delete)(cj);
            (r.json_delete)(rj);
        }
        // true/false/null hit `default: return` — no free, state untouched.
        for (name, cf, rf) in [
            ("json_true", c.json_true, r.json_true),
            ("json_false", c.json_false, r.json_false),
            ("json_null", c.json_null, r.json_null),
        ] {
            let cj = cf();
            let rj = rf();
            (c.json_delete)(cj);
            (r.json_delete)(rj);
            assert_eq!((*cj).refcount, usize::MAX, "C: {name} survives json_delete");
            cmp(c, r, cj, rj, &format!("{name} after json_delete"));
        }
    }
}

#[test]
fn row177_json_delete_null_is_a_noop() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for _ in 0..10 {
            (c.json_delete)(std::ptr::null_mut());
            (r.json_delete)(std::ptr::null_mut());
        }
        // Reaching here in both means both took the early return.
        diff_eq!(true, true, "json_delete(NULL) returned without crashing");
    }
}

// ===========================================================================
// Rows 178-187 — object accessors / mutators
// ===========================================================================

/// Build `n` keys "key0000".."keyNNNN" into a fresh object of each library and
/// return the pair.
unsafe fn objs_with_n(c: &Api, r: &Api, n: usize) -> (*mut json_t, *mut json_t) {
    let cj = (c.json_object)();
    let rj = (r.json_object)();
    for i in 0..n {
        let k = cs(&format!("key{i:04}"));
        (c.json_object_set_new)(cj, k.as_ptr(), (c.json_integer)(i as i64));
        (r.json_object_set_new)(rj, k.as_ptr(), (r.json_integer)(i as i64));
    }
    (cj, rj)
}

/// One value of every non-object type, plus NULL, for the "wrong type" branches.
unsafe fn non_objects(api: &Api) -> Vec<(&'static str, *mut json_t)> {
    vec![
        ("array", (api.json_array)()),
        ("string", (api.json_string)(b"s\0".as_ptr() as *const c_char)),
        ("integer", (api.json_integer)(7)),
        ("real", (api.json_real)(2.5)),
        ("true", (api.json_true)()),
        ("false", (api.json_false)()),
        ("null", (api.json_null)()),
        ("NULL", std::ptr::null_mut()),
    ]
}

unsafe fn non_arrays(api: &Api) -> Vec<(&'static str, *mut json_t)> {
    vec![
        ("object", (api.json_object)()),
        ("string", (api.json_string)(b"s\0".as_ptr() as *const c_char)),
        ("integer", (api.json_integer)(7)),
        ("real", (api.json_real)(2.5)),
        ("true", (api.json_true)()),
        ("false", (api.json_false)()),
        ("null", (api.json_null)()),
        ("NULL", std::ptr::null_mut()),
    ]
}

#[test]
fn row178_json_object_size() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [0usize, 1, 8, 9, 16, 17, 33] {
            let (cj, rj) = objs_with_n(c, r, n);
            diff_eq!(
                (c.json_object_size)(cj),
                (r.json_object_size)(rj),
                "json_object_size with {n} keys"
            );
            assert_eq!((c.json_object_size)(cj), n, "C: size == {n}");
            cmp_free(c, r, cj, rj, &format!("object with {n} keys"));
        }
        // Every non-object type and NULL -> 0.
        let cs_ = non_objects(c);
        let rs_ = non_objects(r);
        for i in 0..cs_.len() {
            diff_eq!(
                (c.json_object_size)(cs_[i].1),
                (r.json_object_size)(rs_[i].1),
                "json_object_size({})",
                cs_[i].0
            );
            assert_eq!((c.json_object_size)(cs_[i].1), 0, "C: size of {} is 0", cs_[i].0);
            decref(c, cs_[i].1);
            decref(r, rs_[i].1);
        }
    }
}

#[test]
fn row179_json_object_get() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0179_0001);
    unsafe {
        for n in [1usize, 12] {
            let (cj, rj) = objs_with_n(c, r, n);
            for i in 0..n {
                let k = cs(&format!("key{i:04}"));
                let cv = (c.json_object_get)(cj, k.as_ptr());
                let rv = (r.json_object_get)(rj, k.as_ptr());
                cmp(c, r, cv, rv, &format!("json_object_get(present key{i:04}, n={n})"));
            }
            // Absent keys, including randomised ones.
            let mut absent: Vec<String> = vec![
                "".to_string(),
                "key".to_string(),
                "key0".to_string(),
                "key00000".to_string(),
                "KEY0000".to_string(),
            ];
            for _ in 0..40 {
                absent.push(rng.ascii_string(8));
            }
            for k in &absent {
                if k.contains('\0') {
                    continue;
                }
                let kk = cs(k);
                let cv = (c.json_object_get)(cj, kk.as_ptr());
                let rv = (r.json_object_get)(rj, kk.as_ptr());
                diff_eq!(cv.is_null(), rv.is_null(), "json_object_get({k:?}) null-ness");
                if !cv.is_null() {
                    cmp(c, r, cv, rv, &format!("json_object_get({k:?})"));
                }
            }
            // key == NULL -> NULL
            diff_eq!(
                (c.json_object_get)(cj, std::ptr::null()).is_null(),
                (r.json_object_get)(rj, std::ptr::null()).is_null(),
                "json_object_get(NULL key)"
            );
            assert!(
                (c.json_object_get)(cj, std::ptr::null()).is_null(),
                "C: NULL key -> NULL"
            );
            cmp_free(c, r, cj, rj, &format!("object of {n} unchanged by gets"));
        }
        // Non-object json and NULL json -> NULL.
        let k = cs("key0000");
        let cs_ = non_objects(c);
        let rs_ = non_objects(r);
        for i in 0..cs_.len() {
            diff_eq!(
                (c.json_object_get)(cs_[i].1, k.as_ptr()).is_null(),
                (r.json_object_get)(rs_[i].1, k.as_ptr()).is_null(),
                "json_object_get on {}",
                cs_[i].0
            );
            assert!(
                (c.json_object_get)(cs_[i].1, k.as_ptr()).is_null(),
                "C: get on {} -> NULL",
                cs_[i].0
            );
            decref(c, cs_[i].1);
            decref(r, rs_[i].1);
        }
    }
}

#[test]
fn row180_json_object_getn_key_len_variants() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        // Stored keys: "", "abc", "ab\0cd", a long one and a UTF-8 one.
        let stored: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"abc".to_vec(),
            b"ab\0cd".to_vec(),
            vec![b'L'; 1500],
            "kéy日".as_bytes().to_vec(),
        ];
        for (i, k) in stored.iter().enumerate() {
            let ret_c = (c.json_object_setn_new_nocheck)(
                cj,
                k.as_ptr() as *const c_char,
                k.len(),
                (c.json_integer)(i as i64),
            );
            let ret_r = (r.json_object_setn_new_nocheck)(
                rj,
                k.as_ptr() as *const c_char,
                k.len(),
                (r.json_integer)(i as i64),
            );
            diff_eq!(ret_c, ret_r, "setn_new_nocheck stored key #{i}");
        }
        cmp(c, r, cj, rj, "object with mixed keys");

        // Probe with exact, shorter, longer and zero key_len.
        for (i, k) in stored.iter().enumerate() {
            for delta in [0isize, -1, 1, -2, 2] {
                let l = k.len() as isize + delta;
                if l < 0 {
                    continue;
                }
                let l = l as usize;
                // Reading `l` bytes must stay inside a buffer we own.
                let mut buf = k.clone();
                buf.resize(k.len().max(l) + 1, b'Z');
                let cv = (c.json_object_getn)(cj, buf.as_ptr() as *const c_char, l);
                let rv = (r.json_object_getn)(rj, buf.as_ptr() as *const c_char, l);
                diff_eq!(
                    cv.is_null(),
                    rv.is_null(),
                    "json_object_getn(stored #{i}, key_len {l}) null-ness"
                );
                if !cv.is_null() {
                    cmp(c, r, cv, rv, &format!("json_object_getn(stored #{i}, len {l})"));
                }
            }
        }
        // key_len == 0 must find the stored "" key.
        let empty = cs_bytes(b"");
        let cv = (c.json_object_getn)(cj, empty.as_ptr(), 0);
        let rv = (r.json_object_getn)(rj, empty.as_ptr(), 0);
        assert!(!cv.is_null(), "C: key_len 0 finds the \"\" key");
        cmp(c, r, cv, rv, "json_object_getn(\"\", 0)");

        // key == NULL -> NULL; non-object -> NULL.
        diff_eq!(
            (c.json_object_getn)(cj, std::ptr::null(), 3).is_null(),
            (r.json_object_getn)(rj, std::ptr::null(), 3).is_null(),
            "json_object_getn(NULL key)"
        );
        let probe = cs_bytes(b"abc");
        let cs_ = non_objects(c);
        let rs_ = non_objects(r);
        for i in 0..cs_.len() {
            diff_eq!(
                (c.json_object_getn)(cs_[i].1, probe.as_ptr(), 3).is_null(),
                (r.json_object_getn)(rs_[i].1, probe.as_ptr(), 3).is_null(),
                "json_object_getn on {}",
                cs_[i].0
            );
            decref(c, cs_[i].1);
            decref(r, rs_[i].1);
        }
        cmp_free(c, r, cj, rj, "object after getn probing");
    }
}

#[test]
fn row181_json_object_set_new() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0181_0001);
    unsafe {
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        // Valid ASCII keys on a fresh object, then overwrite each.
        for round in 0..3 {
            for i in 0..12 {
                let k = cs(&format!("k{i}"));
                let v = rng.json_int();
                let cret = (c.json_object_set_new)(cj, k.as_ptr(), (c.json_integer)(v));
                let rret = (r.json_object_set_new)(rj, k.as_ptr(), (r.json_integer)(v));
                diff_eq!(cret, rret, "json_object_set_new(k{i}) round {round}");
                cmp(c, r, cj, rj, &format!("after set_new(k{i}) round {round}"));
            }
        }
        // key == NULL -> value decref'd, -1.
        let cret = (c.json_object_set_new)(cj, std::ptr::null(), (c.json_integer)(1));
        let rret = (r.json_object_set_new)(rj, std::ptr::null(), (r.json_integer)(1));
        diff_eq!(cret, rret, "json_object_set_new(NULL key)");
        assert_eq!(cret, -1, "C: NULL key -> -1");
        cmp(c, r, cj, rj, "object unchanged after NULL-key set_new");

        // Invalid-UTF-8 key -> -1 (utf8_check_string in json_object_setn_new).
        for (name, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            let cret = (c.json_object_set_new)(cj, buf.as_ptr(), (c.json_integer)(2));
            let rret = (r.json_object_set_new)(rj, buf.as_ptr(), (r.json_integer)(2));
            diff_eq!(cret, rret, "json_object_set_new(bad key {name})");
            assert_eq!(cret, -1, "C: invalid UTF-8 key {name} -> -1");
            cmp(c, r, cj, rj, &format!("object unchanged after bad key {name}"));
        }
        cmp_free(c, r, cj, rj, "final object of row181");
    }
}

#[test]
fn row182_json_object_setn_new_key_len_variants() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let long_key = vec![b'q'; 1300];
        let cases: Vec<(&str, Vec<u8>, size_t, bool)> = vec![
            ("empty key", b"".to_vec(), 0, true),
            ("long key", long_key.clone(), long_key.len(), true),
            ("utf8 key", "kéy日本".as_bytes().to_vec(), "kéy日本".len(), true),
            ("embedded NUL key", b"a\0b".to_vec(), 3, true),
            ("NUL only", b"\0".to_vec(), 1, true),
            // key_len cutting a multi-byte sequence -> -1
            ("cut 2-byte", "é".as_bytes().to_vec(), 1, false),
            ("cut 3-byte", "日".as_bytes().to_vec(), 2, false),
            ("cut 4-byte", vec![0xF0, 0x9F, 0x98, 0x80], 3, false),
        ];
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        for (name, bytes, len, expect_ok) in &cases {
            let buf = cs_bytes(bytes);
            let cret = (c.json_object_setn_new)(cj, buf.as_ptr(), *len, (c.json_integer)(1));
            let rret = (r.json_object_setn_new)(rj, buf.as_ptr(), *len, (r.json_integer)(1));
            diff_eq!(cret, rret, "json_object_setn_new({name}, len {len})");
            assert_eq!(
                cret == 0,
                *expect_ok,
                "C: json_object_setn_new({name}) expectation"
            );
            cmp(c, r, cj, rj, &format!("object after setn_new({name})"));
        }
        // key == NULL
        let cret = (c.json_object_setn_new)(cj, std::ptr::null(), 3, (c.json_integer)(1));
        let rret = (r.json_object_setn_new)(rj, std::ptr::null(), 3, (r.json_integer)(1));
        diff_eq!(cret, rret, "json_object_setn_new(NULL key)");
        assert_eq!(cret, -1, "C: NULL key -> -1");
        cmp_free(c, r, cj, rj, "final object of row182");
    }
}

#[test]
fn row183_set_new_nocheck_error_branches() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // Invalid-UTF-8 keys are ACCEPTED by the _nocheck entry points.
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        for (name, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            let cret = (c.json_object_set_new_nocheck)(cj, buf.as_ptr(), (c.json_integer)(1));
            let rret = (r.json_object_set_new_nocheck)(rj, buf.as_ptr(), (r.json_integer)(1));
            diff_eq!(cret, rret, "set_new_nocheck(bad key {name})");
            assert_eq!(cret, 0, "C: _nocheck accepts invalid UTF-8 key {name}");
            let cret = (c.json_object_setn_new_nocheck)(
                cj,
                buf.as_ptr(),
                bytes.len(),
                (c.json_integer)(2),
            );
            let rret = (r.json_object_setn_new_nocheck)(
                rj,
                buf.as_ptr(),
                bytes.len(),
                (r.json_integer)(2),
            );
            diff_eq!(cret, rret, "setn_new_nocheck(bad key {name})");
            cmp(c, r, cj, rj, &format!("object after _nocheck bad key {name}"));
        }
        cmp(c, r, cj, rj, "object with invalid-UTF-8 keys");

        // value == NULL -> -1, checked BEFORE anything else, no decref.
        let k = cs("k");
        diff_eq!(
            (c.json_object_setn_new_nocheck)(cj, k.as_ptr(), 1, std::ptr::null_mut()),
            (r.json_object_setn_new_nocheck)(rj, k.as_ptr(), 1, std::ptr::null_mut()),
            "setn_new_nocheck(value NULL)"
        );
        assert_eq!(
            (c.json_object_setn_new_nocheck)(cj, k.as_ptr(), 1, std::ptr::null_mut()),
            -1,
            "C: NULL value -> -1"
        );
        diff_eq!(
            (c.json_object_set_new_nocheck)(cj, k.as_ptr(), std::ptr::null_mut()),
            (r.json_object_set_new_nocheck)(rj, k.as_ptr(), std::ptr::null_mut()),
            "set_new_nocheck(value NULL)"
        );
        // key == NULL -> -1 + decref of value.
        diff_eq!(
            (c.json_object_set_new_nocheck)(cj, std::ptr::null(), (c.json_integer)(3)),
            (r.json_object_set_new_nocheck)(rj, std::ptr::null(), (r.json_integer)(3)),
            "set_new_nocheck(NULL key)"
        );
        diff_eq!(
            (c.json_object_setn_new_nocheck)(cj, std::ptr::null(), 2, (c.json_integer)(3)),
            (r.json_object_setn_new_nocheck)(rj, std::ptr::null(), 2, (r.json_integer)(3)),
            "setn_new_nocheck(NULL key)"
        );
        cmp(c, r, cj, rj, "object unchanged by NULL key/value");

        // Non-object target -> -1 + decref.
        let cn = non_objects(c);
        let rn = non_objects(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_object_setn_new_nocheck)(cn[i].1, k.as_ptr(), 1, (c.json_integer)(4)),
                (r.json_object_setn_new_nocheck)(rn[i].1, k.as_ptr(), 1, (r.json_integer)(4)),
                "setn_new_nocheck on non-object {}",
                cn[i].0
            );
            diff_eq!(
                (c.json_object_set_new_nocheck)(cn[i].1, k.as_ptr(), (c.json_integer)(4)),
                (r.json_object_set_new_nocheck)(rn[i].1, k.as_ptr(), (r.json_integer)(4)),
                "set_new_nocheck on non-object {}",
                cn[i].0
            );
            if !cn[i].1.is_null() {
                cmp(c, r, cn[i].1, rn[i].1, &format!("{} untouched", cn[i].0));
            }
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        cmp(c, r, cj, rj, "object still intact");
        decref(c, cj);
        decref(r, rj);

        // json == value self-insert -> -1 + decref. The decref is of the object
        // itself, so hold an extra reference to survive it.
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        incref(cj);
        incref(rj);
        diff_eq!(
            (c.json_object_setn_new_nocheck)(cj, k.as_ptr(), 1, cj),
            (r.json_object_setn_new_nocheck)(rj, k.as_ptr(), 1, rj),
            "setn_new_nocheck self-insert"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after self-insert");
        assert_eq!((*cj).refcount, 1, "C: self-insert decref'd the object");
        diff_eq!(
            (c.json_object_set_new_nocheck)(cj, k.as_ptr(), incref(cj)),
            (r.json_object_set_new_nocheck)(rj, k.as_ptr(), incref(rj)),
            "set_new_nocheck self-insert"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after 2nd self-insert");
        diff_eq!(
            (c.json_object_set_new)(cj, k.as_ptr(), incref(cj)),
            (r.json_object_set_new)(rj, k.as_ptr(), incref(rj)),
            "set_new self-insert"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after set_new self-insert");
        cmp_free(c, r, cj, rj, "object after self-insert attempts");
    }
}

#[test]
fn row184_setn_new_nocheck_all_eight_types() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        for (name, v) in all_eight() {
            let k = name.as_bytes();
            let cret = (c.json_object_setn_new_nocheck)(
                cj,
                k.as_ptr() as *const c_char,
                k.len(),
                build(c, &v),
            );
            let rret = (r.json_object_setn_new_nocheck)(
                rj,
                k.as_ptr() as *const c_char,
                k.len(),
                build(r, &v),
            );
            diff_eq!(cret, rret, "setn_new_nocheck({name})");
            cmp(c, r, cj, rj, &format!("object after storing {name}"));
        }
        // Read each back and compare type + value.
        for (name, _) in all_eight() {
            let k = name.as_bytes();
            let cv = (c.json_object_getn)(cj, k.as_ptr() as *const c_char, k.len());
            let rv = (r.json_object_getn)(rj, k.as_ptr() as *const c_char, k.len());
            assert!(!cv.is_null(), "C: {name} readable");
            cmp(c, r, cv, rv, &format!("readback of {name}"));
        }
        cmp_free(c, r, cj, rj, "object holding all eight types");
    }
}

#[test]
fn row185_object_growth_across_rehash_boundaries() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        // k0..k7 fills the 8 buckets; k8 rehashes to order 4; k16 to order 5.
        for i in 0..40 {
            let k = cs(&format!("k{i}"));
            let cret = (c.json_object_set_new)(cj, k.as_ptr(), (c.json_integer)(i));
            let rret = (r.json_object_set_new)(rj, k.as_ptr(), (r.json_integer)(i));
            diff_eq!(cret, rret, "set_new(k{i})");
            // The order field pins the exact moment of each rehash; the full
            // iteration pins its result.
            diff_eq!(obj_order(cj), obj_order(rj), "hashtable order after k{i}");
            cmp(c, r, cj, rj, &format!("object after inserting k{i}"));
            // Every key inserted so far is still retrievable.
            for j in 0..=i {
                let kj = cs(&format!("k{j}"));
                let cv = (c.json_object_get)(cj, kj.as_ptr());
                let rv = (r.json_object_get)(rj, kj.as_ptr());
                diff_eq!(
                    (c.json_integer_value)(cv),
                    (r.json_integer_value)(rv),
                    "k{j} still present after k{i}"
                );
                assert!(!cv.is_null(), "C: k{j} present after inserting k{i}");
            }
        }
        assert_eq!(obj_order(cj), 6, "C: 40 keys -> order 6");
        cmp_free(c, r, cj, rj, "40-key object");
    }
}

#[test]
fn row186_json_object_del_and_deln() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0186_0001);
    unsafe {
        // Delete every key of a 12-key object, in randomised order.
        for trial in 0..20 {
            let (cj, rj) = objs_with_n(c, r, 12);
            let mut keys: Vec<usize> = (0..12).collect();
            for i in (1..keys.len()).rev() {
                let j = rng.below(i + 1);
                keys.swap(i, j);
            }
            for &i in &keys {
                let k = cs(&format!("key{i:04}"));
                let cret = (c.json_object_del)(cj, k.as_ptr());
                let rret = (r.json_object_del)(rj, k.as_ptr());
                diff_eq!(cret, rret, "trial {trial}: del key{i:04}");
                assert_eq!(cret, 0, "C: del of present key succeeds");
                cmp(c, r, cj, rj, &format!("trial {trial}: after del key{i:04}"));
                // A second delete must fail.
                diff_eq!(
                    (c.json_object_del)(cj, k.as_ptr()),
                    (r.json_object_del)(rj, k.as_ptr()),
                    "trial {trial}: second del key{i:04}"
                );
            }
            // Delete-then-reinsert.
            let k = cs("key0003");
            diff_eq!(
                (c.json_object_set_new)(cj, k.as_ptr(), (c.json_integer)(99)),
                (r.json_object_set_new)(rj, k.as_ptr(), (r.json_integer)(99)),
                "trial {trial}: reinsert after full drain"
            );
            cmp_free(c, r, cj, rj, &format!("trial {trial}: after reinsert"));
        }

        // deln with wrong key_len, key_len == 0 on "", key with embedded NUL.
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        let stored: Vec<Vec<u8>> = vec![b"".to_vec(), b"abc".to_vec(), b"ab\0cd".to_vec()];
        for (i, k) in stored.iter().enumerate() {
            (c.json_object_setn_new_nocheck)(
                cj,
                k.as_ptr() as *const c_char,
                k.len(),
                (c.json_integer)(i as i64),
            );
            (r.json_object_setn_new_nocheck)(
                rj,
                k.as_ptr() as *const c_char,
                k.len(),
                (r.json_integer)(i as i64),
            );
        }
        for (i, k) in stored.iter().enumerate() {
            for delta in [-1isize, 1, 0] {
                let l = k.len() as isize + delta;
                if l < 0 {
                    continue;
                }
                let l = l as usize;
                let mut buf = k.clone();
                buf.resize(k.len().max(l) + 1, b'Z');
                let cret = (c.json_object_deln)(cj, buf.as_ptr() as *const c_char, l);
                let rret = (r.json_object_deln)(rj, buf.as_ptr() as *const c_char, l);
                diff_eq!(cret, rret, "deln(stored #{i}, len {l})");
                cmp(c, r, cj, rj, &format!("after deln(stored #{i}, len {l})"));
            }
        }
        // key == NULL and non-object -> -1.
        diff_eq!(
            (c.json_object_del)(cj, std::ptr::null()),
            (r.json_object_del)(rj, std::ptr::null()),
            "json_object_del(NULL key)"
        );
        diff_eq!(
            (c.json_object_deln)(cj, std::ptr::null(), 2),
            (r.json_object_deln)(rj, std::ptr::null(), 2),
            "json_object_deln(NULL key)"
        );
        let probe = cs("abc");
        let cn = non_objects(c);
        let rn = non_objects(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_object_del)(cn[i].1, probe.as_ptr()),
                (r.json_object_del)(rn[i].1, probe.as_ptr()),
                "json_object_del on {}",
                cn[i].0
            );
            diff_eq!(
                (c.json_object_deln)(cn[i].1, probe.as_ptr(), 3),
                (r.json_object_deln)(rn[i].1, probe.as_ptr(), 3),
                "json_object_deln on {}",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        cmp_free(c, r, cj, rj, "row186 final object");
    }
}

#[test]
fn row187_json_object_clear() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [0usize, 1, 12, 40] {
            let (cj, rj) = objs_with_n(c, r, n);
            let cret = (c.json_object_clear)(cj);
            let rret = (r.json_object_clear)(rj);
            diff_eq!(cret, rret, "json_object_clear({n} keys)");
            assert_eq!(cret, 0, "C: clear returns 0");
            assert_eq!((c.json_object_size)(cj), 0, "C: size 0 after clear");
            cmp(c, r, cj, rj, &format!("object after clear of {n} keys"));
            // Still usable, and clear() keeps the grown bucket order.
            for i in 0..10 {
                let k = cs(&format!("new{i}"));
                diff_eq!(
                    (c.json_object_set_new)(cj, k.as_ptr(), (c.json_integer)(1000 + i)),
                    (r.json_object_set_new)(rj, k.as_ptr(), (r.json_integer)(1000 + i)),
                    "reuse after clear({n}) insert {i}"
                );
                cmp(c, r, cj, rj, &format!("reuse after clear({n}) insert {i}"));
            }
            // Clearing twice in a row is valid.
            diff_eq!(
                (c.json_object_clear)(cj),
                (r.json_object_clear)(rj),
                "first of double clear"
            );
            diff_eq!(
                (c.json_object_clear)(cj),
                (r.json_object_clear)(rj),
                "second of double clear"
            );
            cmp_free(c, r, cj, rj, &format!("after double clear ({n})"));
        }
        // Non-object -> -1.
        let cn = non_objects(c);
        let rn = non_objects(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_object_clear)(cn[i].1),
                (r.json_object_clear)(rn[i].1),
                "json_object_clear on {}",
                cn[i].0
            );
            assert_eq!(
                (c.json_object_clear)(cn[i].1),
                -1,
                "C: clear on {} -> -1",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
    }
}

// ===========================================================================
// Rows 188-193 — object update family
// ===========================================================================

/// The four update entry points, keyed by name.
fn update_fns(
    api: &Api,
) -> Vec<(&'static str, unsafe extern "C" fn(*mut json_t, *mut json_t) -> c_int)> {
    vec![
        ("json_object_update", api.json_object_update),
        ("json_object_update_existing", api.json_object_update_existing),
        ("json_object_update_missing", api.json_object_update_missing),
        ("json_object_update_recursive", api.json_object_update_recursive),
    ]
}

#[test]
fn row188_json_object_update() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0188_0001);
    unsafe {
        // Overlapping + disjoint keys, plus empty on either side.
        let shapes: Vec<(V, V)> = vec![
            (
                V::Obj(vec![
                    (b"a".to_vec(), V::Int(1)),
                    (b"b".to_vec(), V::Int(2)),
                    (b"c".to_vec(), V::Int(3)),
                ]),
                V::Obj(vec![
                    (b"b".to_vec(), V::Int(20)),
                    (b"d".to_vec(), V::Int(40)),
                ]),
            ),
            (V::Obj(vec![]), V::Obj(vec![(b"x".to_vec(), V::Int(1))])),
            (V::Obj(vec![(b"x".to_vec(), V::Int(1))]), V::Obj(vec![])),
            (V::Obj(vec![]), V::Obj(vec![])),
        ];
        for (i, (a, b)) in shapes.iter().enumerate() {
            let ca = build(c, a);
            let cb = build(c, b);
            let ra = build(r, a);
            let rb = build(r, b);
            let cret = (c.json_object_update)(ca, cb);
            let rret = (r.json_object_update)(ra, rb);
            diff_eq!(cret, rret, "json_object_update shape {i}");
            cmp(c, r, ca, ra, &format!("target after update shape {i}"));
            cmp(c, r, cb, rb, &format!("source after update shape {i}"));
            // Values are SHARED (incref'd), not copied: the value stored under a
            // key of `other` must be the very same node in both containers.
            let mut it = (c.json_object_iter)(cb);
            while !it.is_null() {
                let kp = (c.json_object_iter_key)(it);
                let kl = (c.json_object_iter_key_len)(it);
                let sv = (c.json_object_iter_value)(it);
                let tv = (c.json_object_getn)(ca, kp, kl);
                assert_eq!(sv, tv, "C: json_object_update shares values (shape {i})");
                it = (c.json_object_iter_next)(cb, it);
            }
            let mut it = (r.json_object_iter)(rb);
            let mut shared = true;
            while !it.is_null() {
                let kp = (r.json_object_iter_key)(it);
                let kl = (r.json_object_iter_key_len)(it);
                shared &= (r.json_object_iter_value)(it) == (r.json_object_getn)(ra, kp, kl);
                it = (r.json_object_iter_next)(rb, it);
            }
            diff_eq!(true, shared, "Rust json_object_update must share values (shape {i})");
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }

        // Self-update (object == other).
        for n in [0usize, 1, 5, 12] {
            let (ca, ra) = objs_with_n(c, r, n);
            let cret = (c.json_object_update)(ca, ca);
            let rret = (r.json_object_update)(ra, ra);
            diff_eq!(cret, rret, "self json_object_update with {n} keys");
            cmp_free(c, r, ca, ra, &format!("after self-update with {n} keys"));
        }

        // Non-object either argument -> -1, for all four update entry points.
        let cn = non_objects(c);
        let rn = non_objects(r);
        let cobj = (c.json_object)();
        let robj = (r.json_object)();
        (c.json_object_set_new)(cobj, cs("z").as_ptr(), (c.json_integer)(1));
        (r.json_object_set_new)(robj, cs("z").as_ptr(), (r.json_integer)(1));
        let cf = update_fns(c);
        let rf = update_fns(r);
        for f in 0..cf.len() {
            for i in 0..cn.len() {
                diff_eq!(
                    (cf[f].1)(cn[i].1, cobj),
                    (rf[f].1)(rn[i].1, robj),
                    "{}(non-object {} , object)",
                    cf[f].0,
                    cn[i].0
                );
                diff_eq!(
                    (cf[f].1)(cobj, cn[i].1),
                    (rf[f].1)(robj, rn[i].1),
                    "{}(object, non-object {})",
                    cf[f].0,
                    cn[i].0
                );
                diff_eq!(
                    (cf[f].1)(cn[i].1, cn[i].1),
                    (rf[f].1)(rn[i].1, rn[i].1),
                    "{}(both non-object {})",
                    cf[f].0,
                    cn[i].0
                );
            }
            cmp(c, r, cobj, robj, &format!("object untouched by {} error paths", cf[f].0));
        }
        for i in 0..cn.len() {
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        decref(c, cobj);
        decref(r, robj);

        // Randomised: many random object pairs through all four entry points.
        for trial in 0..60 {
            let a = V::Obj(
                (0..rng.below(8))
                    .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 2)))
                    .collect(),
            );
            let b = V::Obj(
                (0..rng.below(8))
                    .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 2)))
                    .collect(),
            );
            for f in 0..4 {
                let ca = build(c, &a);
                let cb = build(c, &b);
                let ra = build(r, &a);
                let rb = build(r, &b);
                let cf = update_fns(c);
                let rf = update_fns(r);
                diff_eq!(
                    (cf[f].1)(ca, cb),
                    (rf[f].1)(ra, rb),
                    "trial {trial}: {} return",
                    cf[f].0
                );
                cmp(c, r, ca, ra, &format!("trial {trial}: {} target", cf[f].0));
                cmp(c, r, cb, rb, &format!("trial {trial}: {} source", cf[f].0));
                decref(c, ca);
                decref(c, cb);
                decref(r, ra);
                decref(r, rb);
            }
        }
    }
}

#[test]
fn row189_update_existing_and_missing() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let base = V::Obj(vec![
            (b"a".to_vec(), V::Int(1)),
            (b"b".to_vec(), V::Int(2)),
            (b"c".to_vec(), V::Int(3)),
        ]);
        let others: Vec<(&str, V)> = vec![
            (
                "all present",
                V::Obj(vec![
                    (b"a".to_vec(), V::Int(10)),
                    (b"c".to_vec(), V::Int(30)),
                ]),
            ),
            (
                "none present",
                V::Obj(vec![
                    (b"x".to_vec(), V::Int(10)),
                    (b"y".to_vec(), V::Int(30)),
                ]),
            ),
            (
                "mixed",
                V::Obj(vec![
                    (b"b".to_vec(), V::Int(20)),
                    (b"z".to_vec(), V::Int(90)),
                ]),
            ),
            ("empty other", V::Obj(vec![])),
        ];
        for (name, other) in &others {
            // update_existing never ADDS keys.
            let ca = build(c, &base);
            let ra = build(r, &base);
            let cb = build(c, other);
            let rb = build(r, other);
            let before = (c.json_object_size)(ca);
            let cret = (c.json_object_update_existing)(ca, cb);
            let rret = (r.json_object_update_existing)(ra, rb);
            diff_eq!(cret, rret, "update_existing({name})");
            assert_eq!(
                (c.json_object_size)(ca),
                before,
                "C: update_existing never adds keys ({name})"
            );
            cmp(c, r, ca, ra, &format!("update_existing({name}) target"));
            decref(c, ca);
            decref(r, ra);
            decref(c, cb);
            decref(r, rb);

            // update_missing never OVERWRITES.
            let ca = build(c, &base);
            let ra = build(r, &base);
            let cb = build(c, other);
            let rb = build(r, other);
            let cret = (c.json_object_update_missing)(ca, cb);
            let rret = (r.json_object_update_missing)(ra, rb);
            diff_eq!(cret, rret, "update_missing({name})");
            for k in [b"a", b"b", b"c"] {
                let v = (c.json_object_getn)(ca, k.as_ptr() as *const c_char, 1);
                let want = match k[0] {
                    b'a' => 1,
                    b'b' => 2,
                    _ => 3,
                };
                assert_eq!(
                    (c.json_integer_value)(v),
                    want,
                    "C: update_missing must not overwrite {:?} ({name})",
                    k[0] as char
                );
            }
            cmp(c, r, ca, ra, &format!("update_missing({name}) target"));
            decref(c, ca);
            decref(r, ra);
            decref(c, cb);
            decref(r, rb);
        }
    }
}

#[test]
fn row190_update_recursive_nested() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0190_0001);
    unsafe {
        // 3+ levels where both sides are objects at each level, one side scalar
        // at some keys, and keys missing in `object` entirely.
        let target = V::Obj(vec![
            (
                b"deep".to_vec(),
                V::Obj(vec![(
                    b"deeper".to_vec(),
                    V::Obj(vec![
                        (b"leaf".to_vec(), V::Int(1)),
                        (b"keep".to_vec(), V::Int(2)),
                    ]),
                )]),
            ),
            (b"scalar".to_vec(), V::Int(5)),
            (b"objhere".to_vec(), V::Obj(vec![(b"k".to_vec(), V::Int(1))])),
        ]);
        let other = V::Obj(vec![
            (
                b"deep".to_vec(),
                V::Obj(vec![(
                    b"deeper".to_vec(),
                    V::Obj(vec![
                        (b"leaf".to_vec(), V::Int(111)),
                        (b"added".to_vec(), V::Str(b"new".to_vec())),
                    ]),
                )]),
            ),
            // object on the other side, scalar on this side -> plain overwrite
            (b"scalar".to_vec(), V::Obj(vec![(b"q".to_vec(), V::Int(9))])),
            // scalar on the other side, object on this side -> plain overwrite
            (b"objhere".to_vec(), V::Int(77)),
            // key missing in `object` -> overwrite branch (v == NULL)
            (b"brandnew".to_vec(), V::Arr(vec![V::True, V::Null])),
        ]);
        let ca = build(c, &target);
        let ra = build(r, &target);
        let cb = build(c, &other);
        let rb = build(r, &other);
        let cret = (c.json_object_update_recursive)(ca, cb);
        let rret = (r.json_object_update_recursive)(ra, rb);
        diff_eq!(cret, rret, "json_object_update_recursive nested return");
        assert_eq!(cret, 0, "C: recursive update succeeds");
        cmp(c, r, ca, ra, "target after recursive update");
        cmp(c, r, cb, rb, "source after recursive update");
        // The nested "keep" key must survive (recursive descent, not overwrite).
        let cdeep = (c.json_object_get)(ca, cs("deep").as_ptr());
        let cdeeper = (c.json_object_get)(cdeep, cs("deeper").as_ptr());
        assert!(
            !(c.json_object_get)(cdeeper, cs("keep").as_ptr()).is_null(),
            "C: recursive descent preserves untouched nested keys"
        );
        decref(c, ca);
        decref(r, ra);
        decref(c, cb);
        decref(r, rb);

        // Empty other -> 0.
        let (ca, ra) = objs_with_n(c, r, 5);
        let cb = (c.json_object)();
        let rb = (r.json_object)();
        diff_eq!(
            (c.json_object_update_recursive)(ca, cb),
            (r.json_object_update_recursive)(ra, rb),
            "update_recursive with empty other"
        );
        cmp(c, r, ca, ra, "target unchanged by empty recursive update");
        decref(c, ca);
        decref(r, ra);
        decref(c, cb);
        decref(r, rb);

        // do_object_update_recursive called directly with a caller hashtable.
        for trial in 0..40 {
            let a = V::Obj(
                (0..rng.below(6))
                    .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 3)))
                    .collect(),
            );
            let b = V::Obj(
                (0..rng.below(6))
                    .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 3)))
                    .collect(),
            );
            let ca = build(c, &a);
            let ra = build(r, &a);
            let cb = build(c, &b);
            let rb = build(r, &b);
            let mut cht = Ht::new(c);
            let mut rht = Ht::new(r);
            let cret = (c.do_object_update_recursive)(ca, cb, cht.p());
            let rret = (r.do_object_update_recursive)(ra, rb, rht.p());
            diff_eq!(cret, rret, "trial {trial}: do_object_update_recursive return");
            // The loop key must have been removed on the way out.
            diff_eq!(cht.t.size, rht.t.size, "trial {trial}: parents set size after");
            assert_eq!(cht.t.size, 0, "C: parents set is empty again");
            cmp(c, r, ca, ra, &format!("trial {trial}: do_..._recursive target"));
            cmp(c, r, cb, rb, &format!("trial {trial}: do_..._recursive source"));
            drop(cht);
            drop(rht);
            decref(c, ca);
            decref(r, ra);
            decref(c, cb);
            decref(r, rb);
        }
    }
}

/// Build `a -> b -> a` object cycle in one library; returns (a, b).
/// Both hold a reference to the other, so the cycle must be broken by
/// `json_object_clear` before dropping.
unsafe fn obj_cycle(api: &Api) -> (*mut json_t, *mut json_t) {
    let a = (api.json_object)();
    let b = (api.json_object)();
    // Direct self-insertion is rejected by the C (json == value), so the cycle
    // has to go through a second object.
    (api.json_object_set_new)(a, cs("to_b").as_ptr(), incref(b));
    (api.json_object_set_new)(b, cs("to_a").as_ptr(), incref(a));
    (a, b)
}

unsafe fn break_cycle(api: &Api, a: *mut json_t, b: *mut json_t) {
    (api.json_object_clear)(a);
    (api.json_object_clear)(b);
    decref(api, a);
    decref(api, b);
}

#[test]
fn row191_update_recursive_rejects_cycles() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // (1) other contains itself indirectly: other -> mid -> other.
        let (ca, cb) = obj_cycle(c);
        let (ra, rb) = obj_cycle(r);
        let ctgt = (c.json_object)();
        let rtgt = (r.json_object)();
        // Give the target matching nested objects so the recursion descends.
        (c.json_object_set_new)(ctgt, cs("to_b").as_ptr(), (c.json_object)());
        (r.json_object_set_new)(rtgt, cs("to_b").as_ptr(), (r.json_object)());
        let cinner = (c.json_object_get)(ctgt, cs("to_b").as_ptr());
        let rinner = (r.json_object_get)(rtgt, cs("to_b").as_ptr());
        (c.json_object_set_new)(cinner, cs("to_a").as_ptr(), (c.json_object)());
        (r.json_object_set_new)(rinner, cs("to_a").as_ptr(), (r.json_object)());

        let cret = (c.json_object_update_recursive)(ctgt, ca);
        let rret = (r.json_object_update_recursive)(rtgt, ra);
        diff_eq!(cret, rret, "update_recursive with 2-object cycle A->B->A");
        assert_eq!(cret, -1, "C: cycle must be rejected");
        cmp(c, r, ctgt, rtgt, "target after cycle rejection");
        decref(c, ctgt);
        decref(r, rtgt);
        break_cycle(c, ca, cb);
        break_cycle(r, ra, rb);

        // (2) The cycle nested three levels down inside `other`.
        let (ca, cb) = obj_cycle(c);
        let (ra, rb) = obj_cycle(r);
        let cother = (c.json_object)();
        let rother = (r.json_object)();
        let mut ccur = cother;
        let mut rcur = rother;
        for lvl in 0..3 {
            let cn = (c.json_object)();
            let rn = (r.json_object)();
            let k = cs(&format!("l{lvl}"));
            (c.json_object_set_new)(ccur, k.as_ptr(), cn);
            (r.json_object_set_new)(rcur, k.as_ptr(), rn);
            ccur = cn;
            rcur = rn;
        }
        (c.json_object_set_new)(ccur, cs("cyc").as_ptr(), incref(ca));
        (r.json_object_set_new)(rcur, cs("cyc").as_ptr(), incref(ra));
        // A target with the same nested object shape so descent reaches the cycle.
        let ctgt = (c.json_deep_copy)(cother);
        let rtgt = (r.json_deep_copy)(rother);
        // deep_copy of the cyclic structure fails; build the prefix by hand.
        diff_eq!(ctgt.is_null(), rtgt.is_null(), "deep_copy of nested cycle prefix");
        let ctgt = (c.json_object)();
        let rtgt = (r.json_object)();
        let mut ccur = ctgt;
        let mut rcur = rtgt;
        for lvl in 0..3 {
            let cn = (c.json_object)();
            let rn = (r.json_object)();
            let k = cs(&format!("l{lvl}"));
            (c.json_object_set_new)(ccur, k.as_ptr(), cn);
            (r.json_object_set_new)(rcur, k.as_ptr(), rn);
            ccur = cn;
            rcur = rn;
        }
        (c.json_object_set_new)(ccur, cs("cyc").as_ptr(), (c.json_object)());
        (r.json_object_set_new)(rcur, cs("cyc").as_ptr(), (r.json_object)());
        let ccyc = (c.json_object_get)(ccur, cs("cyc").as_ptr());
        let rcyc = (r.json_object_get)(rcur, cs("cyc").as_ptr());
        (c.json_object_set_new)(ccyc, cs("to_b").as_ptr(), (c.json_object)());
        (r.json_object_set_new)(rcyc, cs("to_b").as_ptr(), (r.json_object)());
        let cb2 = (c.json_object_get)(ccyc, cs("to_b").as_ptr());
        let rb2 = (r.json_object_get)(rcyc, cs("to_b").as_ptr());
        (c.json_object_set_new)(cb2, cs("to_a").as_ptr(), (c.json_object)());
        (r.json_object_set_new)(rb2, cs("to_a").as_ptr(), (r.json_object)());

        let cret = (c.json_object_update_recursive)(ctgt, cother);
        let rret = (r.json_object_update_recursive)(rtgt, rother);
        diff_eq!(cret, rret, "update_recursive with cycle nested 3 levels down");
        assert_eq!(cret, -1, "C: nested cycle must be rejected");
        cmp(c, r, ctgt, rtgt, "target after nested cycle rejection");
        decref(c, ctgt);
        decref(r, rtgt);
        // Break the cycle inside `other` before releasing it.
        (c.json_object_del)(ccur, cs("cyc").as_ptr());
        (r.json_object_del)(rcur, cs("cyc").as_ptr());
        decref(c, cother);
        decref(r, rother);
        break_cycle(c, ca, cb);
        break_cycle(r, ra, rb);

        // (3) other updated into ITSELF, which is the degenerate loop case: the
        // outer object is registered in `parents` before the walk, so the first
        // recursive descent into the same object fails.
        let (ca, cb) = obj_cycle(c);
        let (ra, rb) = obj_cycle(r);
        diff_eq!(
            (c.json_object_update_recursive)(ca, ca),
            (r.json_object_update_recursive)(ra, ra),
            "update_recursive(other, other) on a cyclic object"
        );
        cmp(c, r, ca, ra, "cyclic object after self recursive update");
        break_cycle(c, ca, cb);
        break_cycle(r, ra, rb);
    }
}

#[test]
fn row192_do_object_update_recursive_shared_subtree() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // The SAME nested object under two different keys of `other` is a DAG,
        // not a cycle: hashtable_del(parents, ...) on the way out means the
        // second visit is allowed.
        for (name, extra) in [("plain", 0usize), ("three keys", 1)] {
            let cshared = (c.json_object)();
            let rshared = (r.json_object)();
            (c.json_object_set_new)(cshared, cs("n").as_ptr(), (c.json_integer)(1));
            (r.json_object_set_new)(rshared, cs("n").as_ptr(), (r.json_integer)(1));

            let cother = (c.json_object)();
            let rother = (r.json_object)();
            let mut keys = vec!["a", "b"];
            if extra == 1 {
                keys.push("c");
            }
            for k in &keys {
                (c.json_object_set_new)(cother, cs(k).as_ptr(), incref(cshared));
                (r.json_object_set_new)(rother, cs(k).as_ptr(), incref(rshared));
            }
            let ctgt = (c.json_object)();
            let rtgt = (r.json_object)();
            for k in &keys {
                (c.json_object_set_new)(ctgt, cs(k).as_ptr(), (c.json_object)());
                (r.json_object_set_new)(rtgt, cs(k).as_ptr(), (r.json_object)());
            }
            let mut cht = Ht::new(c);
            let mut rht = Ht::new(r);
            let cret = (c.do_object_update_recursive)(ctgt, cother, cht.p());
            let rret = (r.do_object_update_recursive)(rtgt, rother, rht.p());
            diff_eq!(cret, rret, "do_object_update_recursive shared subtree ({name})");
            assert_eq!(cret, 0, "C: shared subtree must succeed ({name})");
            diff_eq!(cht.t.size, rht.t.size, "parents set drained ({name})");
            assert_eq!(cht.t.size, 0, "C: parents set drained ({name})");
            cmp(c, r, ctgt, rtgt, &format!("target after shared-subtree update ({name})"));
            drop(cht);
            drop(rht);
            decref(c, ctgt);
            decref(r, rtgt);
            decref(c, cother);
            decref(r, rother);
            decref(c, cshared);
            decref(r, rshared);
        }

        // Same via the public entry point.
        let cshared = (c.json_object)();
        let rshared = (r.json_object)();
        (c.json_object_set_new)(cshared, cs("n").as_ptr(), (c.json_integer)(5));
        (r.json_object_set_new)(rshared, cs("n").as_ptr(), (r.json_integer)(5));
        let cother = (c.json_object)();
        let rother = (r.json_object)();
        for k in ["a", "b"] {
            (c.json_object_set_new)(cother, cs(k).as_ptr(), incref(cshared));
            (r.json_object_set_new)(rother, cs(k).as_ptr(), incref(rshared));
        }
        let ctgt = (c.json_object)();
        let rtgt = (r.json_object)();
        for k in ["a", "b"] {
            (c.json_object_set_new)(ctgt, cs(k).as_ptr(), (c.json_object)());
            (r.json_object_set_new)(rtgt, cs(k).as_ptr(), (r.json_object)());
        }
        diff_eq!(
            (c.json_object_update_recursive)(ctgt, cother),
            (r.json_object_update_recursive)(rtgt, rother),
            "json_object_update_recursive shared subtree"
        );
        cmp(c, r, ctgt, rtgt, "public entry point, shared subtree");
        decref(c, ctgt);
        decref(r, rtgt);
        decref(c, cother);
        decref(r, rother);
        decref(c, cshared);
        decref(r, rshared);
    }
}

#[test]
fn row193_update_recursive_non_objects() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // The type check precedes the loop check, so no hashtable entry is made.
        let cobj = (c.json_object)();
        let robj = (r.json_object)();
        let cn = non_objects(c);
        let rn = non_objects(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_object_update_recursive)(cn[i].1, cobj),
                (r.json_object_update_recursive)(rn[i].1, robj),
                "update_recursive(non-object {}, object)",
                cn[i].0
            );
            diff_eq!(
                (c.json_object_update_recursive)(cobj, cn[i].1),
                (r.json_object_update_recursive)(robj, rn[i].1),
                "update_recursive(object, non-object {})",
                cn[i].0
            );
            diff_eq!(
                (c.json_object_update_recursive)(cn[i].1, cn[i].1),
                (r.json_object_update_recursive)(rn[i].1, rn[i].1),
                "update_recursive(both non-object {})",
                cn[i].0
            );
            // And the same through do_object_update_recursive with a live set.
            let mut cht = Ht::new(c);
            let mut rht = Ht::new(r);
            diff_eq!(
                (c.do_object_update_recursive)(cn[i].1, cobj, cht.p()),
                (r.do_object_update_recursive)(rn[i].1, robj, rht.p()),
                "do_object_update_recursive(non-object {}, object)",
                cn[i].0
            );
            diff_eq!(cht.t.size, rht.t.size, "parents untouched for {}", cn[i].0);
            assert_eq!(cht.t.size, 0, "C: type check precedes the loop check");
            drop(cht);
            drop(rht);
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        cmp_free(c, r, cobj, robj, "object untouched by non-object updates");
    }
}

// ===========================================================================
// Row 194 — jsonp_loop_check
// ===========================================================================

#[test]
fn row194_jsonp_loop_check() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        // LOOP_KEY_LEN = 2 + 2*sizeof(json_t*) + 1
        const LOOP_KEY_LEN: usize = 2 + 2 * std::mem::size_of::<*mut json_t>() + 1;
        let mut cht = Ht::new(c);
        let mut rht = Ht::new(r);
        let mut cvals: Vec<*mut json_t> = Vec::new();
        let mut rvals: Vec<*mut json_t> = Vec::new();
        for i in 0..30 {
            cvals.push((c.json_integer)(i));
            rvals.push((r.json_integer)(i));
        }

        let mut ckey = vec![0 as c_char; LOOP_KEY_LEN];
        let mut rkey = vec![0 as c_char; LOOP_KEY_LEN];

        // First call for each distinct pointer -> 0, and the pointer is recorded.
        for i in 0..cvals.len() {
            let mut cklen: size_t = 0;
            let mut rklen: size_t = 0;
            let cret = (c.jsonp_loop_check)(
                cht.p(),
                cvals[i],
                ckey.as_mut_ptr(),
                LOOP_KEY_LEN,
                &mut cklen,
            );
            let rret = (r.jsonp_loop_check)(
                rht.p(),
                rvals[i],
                rkey.as_mut_ptr(),
                LOOP_KEY_LEN,
                &mut rklen,
            );
            diff_eq!(cret, rret, "jsonp_loop_check first call #{i}");
            assert_eq!(cret, 0, "C: first sighting of a pointer returns 0");
            // "%p" of the pointer, and it must fit in LOOP_KEY_LEN.
            let cbuf: Vec<u8> = (0..cklen).map(|j| ckey[j] as u8).collect();
            let rbuf: Vec<u8> = (0..rklen).map(|j| rkey[j] as u8).collect();
            assert_eq!(
                cbuf,
                format!("{:p}", cvals[i]).into_bytes(),
                "C: key is the %p of the pointer"
            );
            assert_eq!(
                rbuf,
                format!("{:p}", rvals[i]).into_bytes(),
                "Rust: key must be the %p of the pointer"
            );
            diff_eq!(cklen, rklen, "jsonp_loop_check key_len #{i}");
            assert!(cklen < LOOP_KEY_LEN, "C: %p output fits in LOOP_KEY_LEN");
            diff_eq!(cht.t.size, rht.t.size, "parents size after #{i}");
        }
        // A second call with the same pointer -> -1, and the set does not grow.
        for i in 0..cvals.len() {
            let before_c = cht.t.size;
            let before_r = rht.t.size;
            let mut cklen: size_t = 0;
            let mut rklen: size_t = 0;
            diff_eq!(
                (c.jsonp_loop_check)(cht.p(), cvals[i], ckey.as_mut_ptr(), LOOP_KEY_LEN, &mut cklen),
                (r.jsonp_loop_check)(rht.p(), rvals[i], rkey.as_mut_ptr(), LOOP_KEY_LEN, &mut rklen),
                "jsonp_loop_check repeat #{i}"
            );
            diff_eq!(cklen, rklen, "repeat key_len #{i}");
            diff_eq!(
                (cht.t.size == before_c, rht.t.size == before_r),
                (true, true),
                "repeat #{i} must not grow the parents set"
            );
        }
        // key_len_out == NULL variant.
        for i in 0..5 {
            diff_eq!(
                (c.jsonp_loop_check)(
                    cht.p(),
                    cvals[i],
                    ckey.as_mut_ptr(),
                    LOOP_KEY_LEN,
                    std::ptr::null_mut()
                ),
                (r.jsonp_loop_check)(
                    rht.p(),
                    rvals[i],
                    rkey.as_mut_ptr(),
                    LOOP_KEY_LEN,
                    std::ptr::null_mut()
                ),
                "jsonp_loop_check with key_len_out == NULL #{i}"
            );
        }
        // Fresh table: every pointer is new again.
        drop(cht);
        drop(rht);
        let mut cht = Ht::new(c);
        let mut rht = Ht::new(r);
        for i in 0..cvals.len() {
            let mut cklen: size_t = 0;
            let mut rklen: size_t = 0;
            diff_eq!(
                (c.jsonp_loop_check)(cht.p(), cvals[i], ckey.as_mut_ptr(), LOOP_KEY_LEN, &mut cklen),
                (r.jsonp_loop_check)(rht.p(), rvals[i], rkey.as_mut_ptr(), LOOP_KEY_LEN, &mut rklen),
                "jsonp_loop_check on a fresh table #{i}"
            );
            diff_eq!(cklen, rklen, "fresh table key_len #{i}");
        }
        diff_eq!(cht.t.size, rht.t.size, "fresh table final size");
        drop(cht);
        drop(rht);
        for i in 0..cvals.len() {
            decref(c, cvals[i]);
            decref(r, rvals[i]);
        }
    }
}

// ===========================================================================
// Rows 195-198 — object iteration
// ===========================================================================

#[test]
fn row195_object_iteration() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [0usize, 1, 12, 40] {
            let (cj, rj) = objs_with_n(c, r, n);
            let cit = (c.json_object_iter)(cj);
            let rit = (r.json_object_iter)(rj);
            diff_eq!(cit.is_null(), rit.is_null(), "json_object_iter({n} keys) null-ness");
            assert_eq!(cit.is_null(), n == 0, "C: empty object -> NULL iter");
            // The full traversal is compared by `snap`, but walk it explicitly
            // too so the key/len/value triple at each step is checked.
            let mut cw = cit;
            let mut rw = rit;
            let mut step = 0;
            while !cw.is_null() {
                let ckl = (c.json_object_iter_key_len)(cw);
                let rkl = (r.json_object_iter_key_len)(rw);
                diff_eq!(ckl, rkl, "n={n} step {step}: iter_key_len");
                let ckp = (c.json_object_iter_key)(cw) as *const u8;
                let rkp = (r.json_object_iter_key)(rw) as *const u8;
                let ck: Vec<u8> = (0..ckl).map(|i| *ckp.add(i)).collect();
                let rk: Vec<u8> = (0..rkl).map(|i| *rkp.add(i)).collect();
                diff_eq!(B(ck), B(rk), "n={n} step {step}: iter_key bytes");
                cmp(
                    c,
                    r,
                    (c.json_object_iter_value)(cw),
                    (r.json_object_iter_value)(rw),
                    &format!("n={n} step {step}: iter_value"),
                );
                cw = (c.json_object_iter_next)(cj, cw);
                rw = (r.json_object_iter_next)(rj, rw);
                step += 1;
            }
            diff_eq!(cw.is_null(), rw.is_null(), "n={n}: iterations ended together");
            diff_eq!(step, n, "n={n}: iteration visited every key");

            // iter_at on every present key, then continue to the end.
            for i in 0..n {
                let k = cs(&format!("key{i:04}"));
                let cat = (c.json_object_iter_at)(cj, k.as_ptr());
                let rat = (r.json_object_iter_at)(rj, k.as_ptr());
                diff_eq!(cat.is_null(), rat.is_null(), "iter_at(key{i:04}) null-ness");
                assert!(!cat.is_null(), "C: iter_at finds a present key");
                let mut ctail: Vec<(size_t, i64)> = Vec::new();
                let mut rtail: Vec<(size_t, i64)> = Vec::new();
                let mut cw = cat;
                let mut rw = rat;
                while !cw.is_null() {
                    ctail.push((
                        (c.json_object_iter_key_len)(cw),
                        (c.json_integer_value)((c.json_object_iter_value)(cw)),
                    ));
                    cw = (c.json_object_iter_next)(cj, cw);
                }
                while !rw.is_null() {
                    rtail.push((
                        (r.json_object_iter_key_len)(rw),
                        (r.json_integer_value)((r.json_object_iter_value)(rw)),
                    ));
                    rw = (r.json_object_iter_next)(rj, rw);
                }
                diff_eq!(ctail, rtail, "tail of iteration from iter_at(key{i:04}), n={n}");
            }
            // iter_at on an absent key / NULL key -> NULL.
            let absent = cs("nope");
            diff_eq!(
                (c.json_object_iter_at)(cj, absent.as_ptr()).is_null(),
                (r.json_object_iter_at)(rj, absent.as_ptr()).is_null(),
                "iter_at(absent), n={n}"
            );
            diff_eq!(
                (c.json_object_iter_at)(cj, std::ptr::null()).is_null(),
                (r.json_object_iter_at)(rj, std::ptr::null()).is_null(),
                "iter_at(NULL key), n={n}"
            );
            // iter == NULL passed to _next -> NULL.
            diff_eq!(
                (c.json_object_iter_next)(cj, std::ptr::null_mut()).is_null(),
                (r.json_object_iter_next)(rj, std::ptr::null_mut()).is_null(),
                "iter_next(NULL iter), n={n}"
            );
            cmp_free(c, r, cj, rj, &format!("object of {n} after iteration"));
        }
        // Non-object -> NULL for iter / iter_at / iter_next.
        let k = cs("k");
        let cn = non_objects(c);
        let rn = non_objects(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_object_iter)(cn[i].1).is_null(),
                (r.json_object_iter)(rn[i].1).is_null(),
                "json_object_iter on {}",
                cn[i].0
            );
            diff_eq!(
                (c.json_object_iter_at)(cn[i].1, k.as_ptr()).is_null(),
                (r.json_object_iter_at)(rn[i].1, k.as_ptr()).is_null(),
                "json_object_iter_at on {}",
                cn[i].0
            );
            diff_eq!(
                (c.json_object_iter_next)(cn[i].1, std::ptr::null_mut()).is_null(),
                (r.json_object_iter_next)(rn[i].1, std::ptr::null_mut()).is_null(),
                "json_object_iter_next on {}",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
    }
}

#[test]
fn row196_json_object_key_to_iter_round_trip() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [1usize, 12, 33] {
            let (cj, rj) = objs_with_n(c, r, n);
            let mut cw = (c.json_object_iter)(cj);
            let mut rw = (r.json_object_iter)(rj);
            let mut step = 0;
            while !cw.is_null() {
                let ckey = (c.json_object_iter_key)(cw);
                let rkey = (r.json_object_iter_key)(rw);
                let cback = (c.json_object_key_to_iter)(ckey);
                let rback = (r.json_object_key_to_iter)(rkey);
                assert_eq!(cback, cw, "C: key_to_iter round-trips to the same iter");
                diff_eq!(cback == cw, rback == rw, "n={n} step {step}: key_to_iter identity");
                cmp(
                    c,
                    r,
                    (c.json_object_iter_value)(cback),
                    (r.json_object_iter_value)(rback),
                    &format!("n={n} step {step}: value via key_to_iter"),
                );
                diff_eq!(
                    (c.json_object_iter_key_len)(cback),
                    (r.json_object_iter_key_len)(rback),
                    "n={n} step {step}: key_len via key_to_iter"
                );
                cw = (c.json_object_iter_next)(cj, cw);
                rw = (r.json_object_iter_next)(rj, rw);
                step += 1;
            }
            cmp_free(c, r, cj, rj, &format!("object of {n} after key_to_iter walk"));
        }
        // key == NULL -> NULL
        diff_eq!(
            (c.json_object_key_to_iter)(std::ptr::null()).is_null(),
            (r.json_object_key_to_iter)(std::ptr::null()).is_null(),
            "json_object_key_to_iter(NULL)"
        );
        assert!(
            (c.json_object_key_to_iter)(std::ptr::null()).is_null(),
            "C: key_to_iter(NULL) == NULL"
        );
    }
}

#[test]
fn row197_iter_key_key_len_value_edge_keys() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let long = vec![b'W'; 1400];
        let keys: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            long.clone(),
            "kéy日本語".as_bytes().to_vec(),
            b"a\0b".to_vec(),
            b"\0".to_vec(),
            b"\0\0\0".to_vec(),
            b"tail\0".to_vec(),
        ];
        let types = all_eight();
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        for (i, k) in keys.iter().enumerate() {
            let (_, v) = &types[i % types.len()];
            (c.json_object_setn_new_nocheck)(
                cj,
                k.as_ptr() as *const c_char,
                k.len(),
                build(c, v),
            );
            (r.json_object_setn_new_nocheck)(
                rj,
                k.as_ptr() as *const c_char,
                k.len(),
                build(r, v),
            );
        }
        cmp(c, r, cj, rj, "object with edge-case keys and all value types");
        // Explicit walk: key bytes must be compared over key_len (which for
        // NUL-containing keys is longer than strlen).
        let mut cw = (c.json_object_iter)(cj);
        let mut rw = (r.json_object_iter)(rj);
        let mut step = 0;
        while !cw.is_null() {
            let ckl = (c.json_object_iter_key_len)(cw);
            let rkl = (r.json_object_iter_key_len)(rw);
            diff_eq!(ckl, rkl, "step {step}: key_len");
            let ckp = (c.json_object_iter_key)(cw) as *const u8;
            let rkp = (r.json_object_iter_key)(rw) as *const u8;
            let ck = B((0..ckl).map(|i| *ckp.add(i)).collect::<Vec<u8>>());
            let rk = B((0..rkl).map(|i| *rkp.add(i)).collect::<Vec<u8>>());
            diff_eq!(ck.clone(), rk, "step {step}: key bytes");
            // key_len can exceed the NUL-terminated length.
            let strlen_vis = ck.0.iter().position(|&b| b == 0).unwrap_or(ck.0.len());
            if ck.0.contains(&0) {
                assert!(ckl > strlen_vis, "C: key_len > strlen for NUL-containing key");
            }
            cmp(
                c,
                r,
                (c.json_object_iter_value)(cw),
                (r.json_object_iter_value)(rw),
                &format!("step {step}: value"),
            );
            cw = (c.json_object_iter_next)(cj, cw);
            rw = (r.json_object_iter_next)(rj, rw);
            step += 1;
        }
        // iter == NULL -> NULL / 0 / NULL
        diff_eq!(
            (c.json_object_iter_key)(std::ptr::null_mut()).is_null(),
            (r.json_object_iter_key)(std::ptr::null_mut()).is_null(),
            "json_object_iter_key(NULL)"
        );
        assert!(
            (c.json_object_iter_key)(std::ptr::null_mut()).is_null(),
            "C: iter_key(NULL) == NULL"
        );
        diff_eq!(
            (c.json_object_iter_key_len)(std::ptr::null_mut()),
            (r.json_object_iter_key_len)(std::ptr::null_mut()),
            "json_object_iter_key_len(NULL)"
        );
        assert_eq!(
            (c.json_object_iter_key_len)(std::ptr::null_mut()),
            0,
            "C: iter_key_len(NULL) == 0"
        );
        diff_eq!(
            (c.json_object_iter_value)(std::ptr::null_mut()).is_null(),
            (r.json_object_iter_value)(std::ptr::null_mut()).is_null(),
            "json_object_iter_value(NULL)"
        );
        cmp_free(c, r, cj, rj, "row197 final object");
    }
}

#[test]
fn row198_json_object_iter_set_new() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let types = all_eight();
        for (ti, (name, v)) in types.iter().enumerate() {
            let (cj, rj) = objs_with_n(c, r, 12);
            // Replace the value at each iteration position in turn.
            let mut cw = (c.json_object_iter)(cj);
            let mut rw = (r.json_object_iter)(rj);
            let mut step = 0;
            while !cw.is_null() {
                let cret = (c.json_object_iter_set_new)(cj, cw, build(c, v));
                let rret = (r.json_object_iter_set_new)(rj, rw, build(r, v));
                diff_eq!(cret, rret, "iter_set_new({name}) at step {step}");
                assert_eq!(cret, 0, "C: iter_set_new succeeds");
                cmp(c, r, cj, rj, &format!("after iter_set_new({name}) step {step}"));
                cw = (c.json_object_iter_next)(cj, cw);
                rw = (r.json_object_iter_next)(rj, rw);
                step += 1;
            }
            // value == NULL -> -1
            let cw = (c.json_object_iter)(cj);
            let rw = (r.json_object_iter)(rj);
            diff_eq!(
                (c.json_object_iter_set_new)(cj, cw, std::ptr::null_mut()),
                (r.json_object_iter_set_new)(rj, rw, std::ptr::null_mut()),
                "iter_set_new(value NULL), type {ti}"
            );
            // iter == NULL -> -1 + decref
            diff_eq!(
                (c.json_object_iter_set_new)(cj, std::ptr::null_mut(), (c.json_integer)(1)),
                (r.json_object_iter_set_new)(rj, std::ptr::null_mut(), (r.json_integer)(1)),
                "iter_set_new(iter NULL), type {ti}"
            );
            cmp(c, r, cj, rj, &format!("object unchanged by iter_set_new errors ({name})"));
            // non-object json -> -1 + decref
            let cn = non_objects(c);
            let rn = non_objects(r);
            for i in 0..cn.len() {
                diff_eq!(
                    (c.json_object_iter_set_new)(cn[i].1, cw, (c.json_integer)(2)),
                    (r.json_object_iter_set_new)(rn[i].1, rw, (r.json_integer)(2)),
                    "iter_set_new on non-object {}",
                    cn[i].0
                );
                decref(c, cn[i].1);
                decref(r, rn[i].1);
            }
            cmp_free(c, r, cj, rj, &format!("row198 object after {name}"));
        }
    }
}

// ===========================================================================
// Rows 199-206 — arrays
// ===========================================================================

unsafe fn arrays_with_n(c: &Api, r: &Api, n: usize) -> (*mut json_t, *mut json_t) {
    let cj = (c.json_array)();
    let rj = (r.json_array)();
    for i in 0..n {
        (c.json_array_append_new)(cj, (c.json_integer)(i as i64));
        (r.json_array_append_new)(rj, (r.json_integer)(i as i64));
    }
    (cj, rj)
}

#[test]
fn row199_json_array_size_and_get() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [0usize, 1, 8, 9, 16, 17, 33] {
            let (cj, rj) = arrays_with_n(c, r, n);
            diff_eq!(
                (c.json_array_size)(cj),
                (r.json_array_size)(rj),
                "json_array_size({n})"
            );
            assert_eq!((c.json_array_size)(cj), n, "C: size == {n}");
            diff_eq!(arr_cap(cj), arr_cap(rj), "capacity with {n} entries");
            // index 0, middle, last, entries, entries+1, SIZE_MAX
            let mut idxs: Vec<size_t> = vec![0, n / 2, n.saturating_sub(1), n, n + 1, size_t::MAX];
            idxs.push(size_t::MAX / 2);
            for &i in &idxs {
                let cv = (c.json_array_get)(cj, i);
                let rv = (r.json_array_get)(rj, i);
                diff_eq!(cv.is_null(), rv.is_null(), "json_array_get({i}) on {n} null-ness");
                if !cv.is_null() {
                    cmp(c, r, cv, rv, &format!("json_array_get({i}) on {n}"));
                } else {
                    assert!(i >= n, "C: only out-of-range indices are NULL");
                }
            }
            cmp_free(c, r, cj, rj, &format!("array of {n}"));
        }
        // Every non-array type and NULL -> 0 / NULL.
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_size)(cn[i].1),
                (r.json_array_size)(rn[i].1),
                "json_array_size on {}",
                cn[i].0
            );
            assert_eq!((c.json_array_size)(cn[i].1), 0, "C: size of {} is 0", cn[i].0);
            for idx in [0usize, 1, size_t::MAX] {
                diff_eq!(
                    (c.json_array_get)(cn[i].1, idx).is_null(),
                    (r.json_array_get)(rn[i].1, idx).is_null(),
                    "json_array_get({idx}) on {}",
                    cn[i].0
                );
                assert!(
                    (c.json_array_get)(cn[i].1, idx).is_null(),
                    "C: get on {} -> NULL",
                    cn[i].0
                );
            }
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
    }
}

#[test]
fn row200_json_array_set_new() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0200_0001);
    unsafe {
        for n in [1usize, 5, 8, 17] {
            for &idx in &[0usize, n / 2, n - 1, n, n + 1, n + 5, size_t::MAX] {
                let (cj, rj) = arrays_with_n(c, r, n);
                let v = rng.json_int();
                let cret = (c.json_array_set_new)(cj, idx, (c.json_integer)(v));
                let rret = (r.json_array_set_new)(rj, idx, (r.json_integer)(v));
                diff_eq!(cret, rret, "json_array_set_new(idx {idx}) on {n}");
                assert_eq!(cret == 0, idx < n, "C: set_new succeeds iff index < entries");
                cmp_free(c, r, cj, rj, &format!("array after set_new({idx}) on {n}"));
            }
        }
        // Empty array: every index fails.
        let (cj, rj) = arrays_with_n(c, r, 0);
        for idx in [0usize, 1, size_t::MAX] {
            diff_eq!(
                (c.json_array_set_new)(cj, idx, (c.json_integer)(1)),
                (r.json_array_set_new)(rj, idx, (r.json_integer)(1)),
                "set_new({idx}) on empty array"
            );
        }
        // value == NULL -> -1 with no decref (checked first).
        diff_eq!(
            (c.json_array_set_new)(cj, 0, std::ptr::null_mut()),
            (r.json_array_set_new)(rj, 0, std::ptr::null_mut()),
            "set_new(value NULL)"
        );
        cmp_free(c, r, cj, rj, "empty array unchanged by failed set_new");

        // Non-array target -> -1 + decref.
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_set_new)(cn[i].1, 0, (c.json_integer)(1)),
                (r.json_array_set_new)(rn[i].1, 0, (r.json_integer)(1)),
                "json_array_set_new on {}",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        // json == value -> -1 + decref (of the array itself).
        let (cj, rj) = arrays_with_n(c, r, 3);
        incref(cj);
        incref(rj);
        diff_eq!(
            (c.json_array_set_new)(cj, 0, cj),
            (r.json_array_set_new)(rj, 0, rj),
            "json_array_set_new self"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after self set_new");
        assert_eq!((*cj).refcount, 1, "C: self set_new decref'd the array");
        cmp_free(c, r, cj, rj, "array after self set_new");
    }
}

#[test]
fn row201_json_array_append_new_growth() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cj = (c.json_array)();
        let rj = (r.json_array)();
        // new_size = max(size + amount, size * 2): 8 -> 16 -> 32 -> 64.
        let expected_cap = |entries: usize| -> usize {
            let mut cap = 8usize;
            for e in 0..entries {
                if e + 1 > cap {
                    cap = std::cmp::max(cap + 1, cap * 2);
                }
            }
            cap
        };
        for i in 0..80usize {
            let cret = (c.json_array_append_new)(cj, (c.json_integer)(i as i64));
            let rret = (r.json_array_append_new)(rj, (r.json_integer)(i as i64));
            diff_eq!(cret, rret, "json_array_append_new #{i}");
            diff_eq!(arr_cap(cj), arr_cap(rj), "capacity after append #{i}");
            assert_eq!(
                arr_cap(cj),
                expected_cap(i + 1),
                "C: capacity growth rule at {} entries",
                i + 1
            );
            cmp(c, r, cj, rj, &format!("array after append #{i}"));
        }
        // Exactly the documented boundaries.
        assert_eq!(expected_cap(8), 8, "C: 8 entries still fit the initial table");
        assert_eq!(expected_cap(9), 16, "C: the 9th append doubles to 16");
        assert_eq!(expected_cap(17), 32, "C: the 17th append doubles to 32");
        cmp_free(c, r, cj, rj, "80-element array");

        // value == NULL -> -1; non-array -> -1 + decref; self -> -1 + decref.
        let (cj, rj) = arrays_with_n(c, r, 3);
        diff_eq!(
            (c.json_array_append_new)(cj, std::ptr::null_mut()),
            (r.json_array_append_new)(rj, std::ptr::null_mut()),
            "append_new(NULL value)"
        );
        cmp(c, r, cj, rj, "array unchanged by NULL append");
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_append_new)(cn[i].1, (c.json_integer)(1)),
                (r.json_array_append_new)(rn[i].1, (r.json_integer)(1)),
                "json_array_append_new on {}",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        incref(cj);
        incref(rj);
        diff_eq!(
            (c.json_array_append_new)(cj, cj),
            (r.json_array_append_new)(rj, rj),
            "json_array_append_new self"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after self append");
        cmp_free(c, r, cj, rj, "array after self append");
    }
}

#[test]
fn row202_json_array_append_all_eight_types() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        let cj = (c.json_array)();
        let rj = (r.json_array)();
        for (name, v) in all_eight() {
            let cret = (c.json_array_append_new)(cj, build(c, &v));
            let rret = (r.json_array_append_new)(rj, build(r, &v));
            diff_eq!(cret, rret, "append_new({name})");
            cmp(c, r, cj, rj, &format!("array after appending {name}"));
        }
        for (i, (name, _)) in all_eight().iter().enumerate() {
            let cv = (c.json_array_get)(cj, i);
            let rv = (r.json_array_get)(rj, i);
            diff_eq!(typeof_(cv), typeof_(rv), "type at index {i} ({name})");
            cmp(c, r, cv, rv, &format!("element {i} ({name})"));
        }
        cmp_free(c, r, cj, rj, "array of all eight types");
    }
}

#[test]
fn row203_json_array_insert_new() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0203_0001);
    unsafe {
        for n in [0usize, 1, 4, 8, 9, 17] {
            for &idx in &[0usize, n / 2, n.saturating_sub(1), n, n + 1, n + 3, size_t::MAX] {
                let (cj, rj) = arrays_with_n(c, r, n);
                let v = rng.json_int();
                let cret = (c.json_array_insert_new)(cj, idx, (c.json_integer)(v));
                let rret = (r.json_array_insert_new)(rj, idx, (r.json_integer)(v));
                diff_eq!(cret, rret, "insert_new(idx {idx}) on {n}");
                assert_eq!(cret == 0, idx <= n, "C: insert succeeds iff index <= entries");
                diff_eq!(arr_cap(cj), arr_cap(rj), "capacity after insert({idx}) on {n}");
                cmp_free(c, r, cj, rj, &format!("array after insert_new({idx}) on {n}"));
            }
        }
        // Repeated inserts at index 0 past capacity 8 (growth AND move in one call).
        let cj = (c.json_array)();
        let rj = (r.json_array)();
        for i in 0..40usize {
            let cret = (c.json_array_insert_new)(cj, 0, (c.json_integer)(i as i64));
            let rret = (r.json_array_insert_new)(rj, 0, (r.json_integer)(i as i64));
            diff_eq!(cret, rret, "insert at 0 #{i}");
            diff_eq!(arr_cap(cj), arr_cap(rj), "capacity after insert at 0 #{i}");
            cmp(c, r, cj, rj, &format!("array after insert at 0 #{i}"));
        }
        cmp_free(c, r, cj, rj, "reverse-built array");

        // Randomised insert positions.
        let cj = (c.json_array)();
        let rj = (r.json_array)();
        for i in 0..120usize {
            let n = (c.json_array_size)(cj);
            let idx = rng.below(n + 2);
            let v = rng.json_int();
            let cret = (c.json_array_insert_new)(cj, idx, (c.json_integer)(v));
            let rret = (r.json_array_insert_new)(rj, idx, (r.json_integer)(v));
            diff_eq!(cret, rret, "random insert #{i} at {idx} (size {n})");
            cmp(c, r, cj, rj, &format!("after random insert #{i} at {idx}"));
        }
        cmp_free(c, r, cj, rj, "randomly built array");

        // Error branches.
        let (cj, rj) = arrays_with_n(c, r, 3);
        diff_eq!(
            (c.json_array_insert_new)(cj, 0, std::ptr::null_mut()),
            (r.json_array_insert_new)(rj, 0, std::ptr::null_mut()),
            "insert_new(NULL value)"
        );
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_insert_new)(cn[i].1, 0, (c.json_integer)(1)),
                (r.json_array_insert_new)(rn[i].1, 0, (r.json_integer)(1)),
                "json_array_insert_new on {}",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        incref(cj);
        incref(rj);
        diff_eq!(
            (c.json_array_insert_new)(cj, 1, cj),
            (r.json_array_insert_new)(rj, 1, rj),
            "json_array_insert_new self"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after self insert");
        cmp_free(c, r, cj, rj, "array after self insert");
    }
}

#[test]
fn row204_json_array_remove() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0204_0001);
    unsafe {
        for n in [0usize, 1, 5, 8, 9, 17] {
            for &idx in &[0usize, n / 2, n.saturating_sub(1), n, n + 1, size_t::MAX] {
                let (cj, rj) = arrays_with_n(c, r, n);
                let cret = (c.json_array_remove)(cj, idx);
                let rret = (r.json_array_remove)(rj, idx);
                diff_eq!(cret, rret, "json_array_remove({idx}) on {n}");
                assert_eq!(cret == 0, idx < n, "C: remove succeeds iff index < entries");
                diff_eq!(arr_cap(cj), arr_cap(rj), "capacity after remove({idx}) on {n}");
                cmp_free(c, r, cj, rj, &format!("array after remove({idx}) on {n}"));
            }
        }
        // Drain front-to-back and back-to-front.
        for &front in &[true, false] {
            let (cj, rj) = arrays_with_n(c, r, 20);
            let mut step = 0;
            while (c.json_array_size)(cj) > 0 {
                let idx = if front { 0 } else { (c.json_array_size)(cj) - 1 };
                diff_eq!(
                    (c.json_array_remove)(cj, idx),
                    (r.json_array_remove)(rj, idx),
                    "drain front={front} step {step}"
                );
                cmp(c, r, cj, rj, &format!("drain front={front} after step {step}"));
                step += 1;
            }
            // Removing from the now-empty array fails.
            diff_eq!(
                (c.json_array_remove)(cj, 0),
                (r.json_array_remove)(rj, 0),
                "remove from drained array (front={front})"
            );
            cmp_free(c, r, cj, rj, &format!("drained array (front={front})"));
        }
        // Randomised removals.
        let (cj, rj) = arrays_with_n(c, r, 40);
        for i in 0..60 {
            let n = (c.json_array_size)(cj);
            let idx = rng.below(n + 2);
            diff_eq!(
                (c.json_array_remove)(cj, idx),
                (r.json_array_remove)(rj, idx),
                "random remove #{i} at {idx} (size {n})"
            );
            cmp(c, r, cj, rj, &format!("after random remove #{i} at {idx}"));
        }
        cmp_free(c, r, cj, rj, "array after random removals");
        // Non-array -> -1.
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_remove)(cn[i].1, 0),
                (r.json_array_remove)(rn[i].1, 0),
                "json_array_remove on {}",
                cn[i].0
            );
            assert_eq!(
                (c.json_array_remove)(cn[i].1, 0),
                -1,
                "C: remove on {} -> -1",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
    }
}

#[test]
fn row205_json_array_clear() {
    let _g = global_state_lock();
    let (c, r) = both();
    unsafe {
        for n in [0usize, 1, 17, 40] {
            let (cj, rj) = arrays_with_n(c, r, n);
            let cap_before = arr_cap(cj);
            let cret = (c.json_array_clear)(cj);
            let rret = (r.json_array_clear)(rj);
            diff_eq!(cret, rret, "json_array_clear({n})");
            assert_eq!(cret, 0, "C: clear returns 0");
            assert_eq!((c.json_array_size)(cj), 0, "C: entries == 0 after clear");
            assert_eq!(arr_cap(cj), cap_before, "C: capacity retained across clear");
            diff_eq!(arr_cap(cj), arr_cap(rj), "capacity after clear({n})");
            cmp(c, r, cj, rj, &format!("array after clear({n})"));
            // Reusable.
            for i in 0..12 {
                diff_eq!(
                    (c.json_array_append_new)(cj, (c.json_integer)(100 + i)),
                    (r.json_array_append_new)(rj, (r.json_integer)(100 + i)),
                    "reuse after clear({n}) append {i}"
                );
                diff_eq!(arr_cap(cj), arr_cap(rj), "capacity on reuse after clear({n}) #{i}");
                cmp(c, r, cj, rj, &format!("reuse after clear({n}) append {i}"));
            }
            // Double clear.
            diff_eq!((c.json_array_clear)(cj), (r.json_array_clear)(rj), "first of double clear");
            diff_eq!((c.json_array_clear)(cj), (r.json_array_clear)(rj), "second of double clear");
            cmp_free(c, r, cj, rj, &format!("array after double clear ({n})"));
        }
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_clear)(cn[i].1),
                (r.json_array_clear)(rn[i].1),
                "json_array_clear on {}",
                cn[i].0
            );
            assert_eq!((c.json_array_clear)(cn[i].1), -1, "C: clear on {} -> -1", cn[i].0);
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
    }
}

#[test]
fn row206_json_array_extend() {
    let _g = global_state_lock();
    let (c, r) = both();
    let mut rng = Rng::new(0x0206_0001);
    unsafe {
        // empty+empty, empty+non-empty, non-empty+empty, growth-forcing.
        for (a, b) in [
            (0usize, 0usize),
            (0, 5),
            (5, 0),
            (2, 20),
            (8, 1),
            (8, 8),
            (17, 33),
            (3, 6),
        ] {
            let (ca, ra) = arrays_with_n(c, r, a);
            let cb = (c.json_array)();
            let rb = (r.json_array)();
            for i in 0..b {
                (c.json_array_append_new)(cb, (c.json_string)(cs(&format!("s{i}")).as_ptr()));
                (r.json_array_append_new)(rb, (r.json_string)(cs(&format!("s{i}")).as_ptr()));
            }
            let cap_before = arr_cap(ca);
            let cret = (c.json_array_extend)(ca, cb);
            let rret = (r.json_array_extend)(ra, rb);
            diff_eq!(cret, rret, "json_array_extend({a} += {b}) return");
            assert_eq!(cret, 0, "C: extend succeeds");
            diff_eq!(arr_cap(ca), arr_cap(ra), "capacity after extend({a} += {b})");
            if b == 0 {
                assert_eq!(arr_cap(ca), cap_before, "C: grow(array, 0) reallocates nothing");
            }
            if a == 2 && b == 20 {
                assert_eq!(arr_cap(ca), 28, "C: new_size = max(8+20, 16) = 28");
            }
            cmp(c, r, ca, ra, &format!("target after extend({a} += {b})"));
            cmp(c, r, cb, rb, &format!("source after extend({a} += {b})"));
            // Elements are SHARED (incref'd), not copied.
            let mut all_shared = true;
            for i in 0..b {
                all_shared &= (c.json_array_get)(cb, i) == (c.json_array_get)(ca, a + i);
            }
            assert!(all_shared, "C: extend shares the other array's elements");
            let mut all_shared_r = true;
            for i in 0..b {
                all_shared_r &= (r.json_array_get)(rb, i) == (r.json_array_get)(ra, a + i);
            }
            diff_eq!(true, all_shared_r, "Rust extend must share elements ({a} += {b})");
            decref(c, ca);
            decref(r, ra);
            decref(c, cb);
            decref(r, rb);
        }

        // Self-extend: the array doubles from its own contents. `other->table`
        // is read AFTER the grow, so it must observe the reallocated table.
        for n in [0usize, 1, 3, 8, 9, 20] {
            let (ca, ra) = arrays_with_n(c, r, n);
            let cret = (c.json_array_extend)(ca, ca);
            let rret = (r.json_array_extend)(ra, ra);
            diff_eq!(cret, rret, "self-extend of {n}");
            diff_eq!(arr_cap(ca), arr_cap(ra), "capacity after self-extend of {n}");
            diff_eq!(
                (c.json_array_size)(ca),
                (r.json_array_size)(ra),
                "size after self-extend of {n}"
            );
            assert_eq!((c.json_array_size)(ca), 2 * n, "C: self-extend doubles the size");
            cmp_free(c, r, ca, ra, &format!("array after self-extend of {n}"));
        }

        // Non-array either argument -> -1.
        let (ca, ra) = arrays_with_n(c, r, 3);
        let cn = non_arrays(c);
        let rn = non_arrays(r);
        for i in 0..cn.len() {
            diff_eq!(
                (c.json_array_extend)(cn[i].1, ca),
                (r.json_array_extend)(rn[i].1, ra),
                "extend(non-array {}, array)",
                cn[i].0
            );
            diff_eq!(
                (c.json_array_extend)(ca, cn[i].1),
                (r.json_array_extend)(ra, rn[i].1),
                "extend(array, non-array {})",
                cn[i].0
            );
            decref(c, cn[i].1);
            decref(r, rn[i].1);
        }
        cmp_free(c, r, ca, ra, "array untouched by failed extends");

        // Randomised extends.
        for trial in 0..40 {
            let ca = (c.json_array)();
            let ra = (r.json_array)();
            for step in 0..12 {
                let recipe = V::Arr(
                    (0..rng.below(10))
                        .map(|_| rand_value(&mut rng, 2))
                        .collect(),
                );
                let cb = build(c, &recipe);
                let rb = build(r, &recipe);
                diff_eq!(
                    (c.json_array_extend)(ca, cb),
                    (r.json_array_extend)(ra, rb),
                    "trial {trial} step {step}: extend return"
                );
                diff_eq!(arr_cap(ca), arr_cap(ra), "trial {trial} step {step}: capacity");
                cmp(c, r, ca, ra, &format!("trial {trial} step {step}: extended array"));
                decref(c, cb);
                decref(r, rb);
            }
            cmp_free(c, r, ca, ra, &format!("trial {trial}: final extended array"));
        }
    }
}

// ===========================================================================
// Rows 207-212 — scalar accessors and setters
// ===========================================================================

/// One value of every type, in a fixed order, for cross-type probing.
unsafe fn every_type(api: &Api) -> Vec<(&'static str, *mut json_t)> {
    let mut v: Vec<(&'static str, *mut json_t)> = Vec::new();
    for (name, recipe) in all_eight() {
        v.push((name, build(api, &recipe)));
    }
    v.push(("NULL", std::ptr::null_mut()));
    v
}

#[test]
fn row207_json_string_value_and_length() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0207_0001);
    unsafe {
        let mut cases: Vec<(Vec<u8>, size_t)> = vec![
            (b"".to_vec(), 0),
            (b"ascii".to_vec(), 5),
            ("héllo日本".as_bytes().to_vec(), "héllo日本".len()),
            (b"a\0b".to_vec(), 3),
            (b"\0\0".to_vec(), 2),
            (vec![b'p'; 2000], 2000),
        ];
        for _ in 0..150 {
            let n = rng.below(30);
            let bytes: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
            let l = bytes.len();
            cases.push((bytes, l));
        }
        for (i, (bytes, len)) in cases.iter().enumerate() {
            let buf = cs_bytes(bytes);
            let cj = (c.json_stringn_nocheck)(buf.as_ptr(), *len);
            let rj = (r.json_stringn_nocheck)(buf.as_ptr(), *len);
            diff_eq!(
                (c.json_string_length)(cj),
                (r.json_string_length)(rj),
                "json_string_length case {i}"
            );
            assert_eq!((c.json_string_length)(cj), *len, "C: length is the stored len");
            let cp = (c.json_string_value)(cj) as *const u8;
            let rp = (r.json_string_value)(rj) as *const u8;
            assert!(!cp.is_null() && !rp.is_null());
            let cb = B((0..*len).map(|j| *cp.add(j)).collect::<Vec<u8>>());
            let rb = B((0..*len).map(|j| *rp.add(j)).collect::<Vec<u8>>());
            diff_eq!(cb, rb, "json_string_value bytes case {i}");
            // The buffer is always NUL-terminated at [len] (jsonp_strndup).
            diff_eq!(*cp.add(*len), *rp.add(*len), "NUL terminator case {i}");
            assert_eq!(*cp.add(*len), 0, "C: buffer is NUL-terminated at len");
            cmp_free(c, r, cj, rj, &format!("string node case {i}"));
        }
        // Every non-string type and NULL -> NULL / 0.
        let ct = every_type(c);
        let rt = every_type(r);
        for i in 0..ct.len() {
            if ct[i].0 == "string" {
                continue;
            }
            diff_eq!(
                (c.json_string_value)(ct[i].1).is_null(),
                (r.json_string_value)(rt[i].1).is_null(),
                "json_string_value on {}",
                ct[i].0
            );
            assert!(
                (c.json_string_value)(ct[i].1).is_null(),
                "C: string_value on {} -> NULL",
                ct[i].0
            );
            diff_eq!(
                (c.json_string_length)(ct[i].1),
                (r.json_string_length)(rt[i].1),
                "json_string_length on {}",
                ct[i].0
            );
            assert_eq!(
                (c.json_string_length)(ct[i].1),
                0,
                "C: string_length on {} -> 0",
                ct[i].0
            );
        }
        for i in 0..ct.len() {
            decref(c, ct[i].1);
            decref(r, rt[i].1);
        }
    }
}

#[test]
fn row208_integer_and_real_value_accessors() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0208_0001);
    unsafe {
        let mut ints: Vec<i64> = vec![0, 1, -1, i32::MAX as i64, i32::MIN as i64, i64::MAX, i64::MIN];
        for _ in 0..200 {
            ints.push(rng.json_int());
        }
        for v in &ints {
            let cj = (c.json_integer)(*v);
            let rj = (r.json_integer)(*v);
            diff_eq!(
                (c.json_integer_value)(cj),
                (r.json_integer_value)(rj),
                "json_integer_value({v})"
            );
            assert_eq!((c.json_integer_value)(cj), *v, "C: round-trip");
            // Cross-typed: json_real_value on an integer node -> 0.
            diff_eq!(
                (c.json_real_value)(cj).to_bits(),
                (r.json_real_value)(rj).to_bits(),
                "json_real_value on integer node ({v})"
            );
            assert_eq!((c.json_real_value)(cj), 0.0, "C: real_value on integer -> 0");
            decref(c, cj);
            decref(r, rj);
        }
        let mut reals: Vec<f64> = vec![0.0, -0.0, 1.5, f64::MIN_POSITIVE, f64::MAX, 1e-300, -1e300];
        for _ in 0..200 {
            reals.push(rng.real());
        }
        for v in &reals {
            let cj = (c.json_real)(*v);
            let rj = (r.json_real)(*v);
            diff_eq!(
                (c.json_real_value)(cj).to_bits(),
                (r.json_real_value)(rj).to_bits(),
                "json_real_value({v:e})"
            );
            assert_eq!(
                (c.json_real_value)(cj).to_bits(),
                v.to_bits(),
                "C: real round-trip preserves the bit pattern"
            );
            // Cross-typed: json_integer_value on a real node -> 0.
            diff_eq!(
                (c.json_integer_value)(cj),
                (r.json_integer_value)(rj),
                "json_integer_value on real node ({v:e})"
            );
            assert_eq!((c.json_integer_value)(cj), 0, "C: integer_value on real -> 0");
            decref(c, cj);
            decref(r, rj);
        }
        // All other types -> 0 / 0.0.
        let ct = every_type(c);
        let rt = every_type(r);
        for i in 0..ct.len() {
            diff_eq!(
                (c.json_integer_value)(ct[i].1),
                (r.json_integer_value)(rt[i].1),
                "json_integer_value on {}",
                ct[i].0
            );
            diff_eq!(
                (c.json_real_value)(ct[i].1).to_bits(),
                (r.json_real_value)(rt[i].1).to_bits(),
                "json_real_value on {}",
                ct[i].0
            );
            if ct[i].0 != "integer" {
                assert_eq!((c.json_integer_value)(ct[i].1), 0, "C: on {} -> 0", ct[i].0);
            }
            if ct[i].0 != "real" {
                assert_eq!((c.json_real_value)(ct[i].1), 0.0, "C: on {} -> 0.0", ct[i].0);
            }
            decref(c, ct[i].1);
            decref(r, rt[i].1);
        }
    }
}

#[test]
fn row209_json_number_value() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0209_0001);
    unsafe {
        let mut ints: Vec<i64> = vec![
            0,
            1,
            -1,
            i32::MAX as i64,
            i32::MIN as i64,
            i64::MAX, // (double)INT64_MAX loses precision
            i64::MIN,
            (1i64 << 53) - 1,
            1i64 << 53,
            (1i64 << 53) + 1,
            -(1i64 << 53) - 1,
        ];
        for _ in 0..250 {
            ints.push(rng.json_int());
        }
        for v in &ints {
            let cj = (c.json_integer)(*v);
            let rj = (r.json_integer)(*v);
            diff_eq!(
                (c.json_number_value)(cj).to_bits(),
                (r.json_number_value)(rj).to_bits(),
                "json_number_value(integer {v})"
            );
            assert_eq!(
                (c.json_number_value)(cj),
                *v as f64,
                "C: number_value of an integer is (double)value"
            );
            decref(c, cj);
            decref(r, rj);
        }
        let mut reals: Vec<f64> = vec![0.0, -0.0, 1.5, f64::MAX, 5e-324];
        for _ in 0..250 {
            reals.push(rng.real());
        }
        for v in &reals {
            let cj = (c.json_real)(*v);
            let rj = (r.json_real)(*v);
            diff_eq!(
                (c.json_number_value)(cj).to_bits(),
                (r.json_number_value)(rj).to_bits(),
                "json_number_value(real {v:e})"
            );
            decref(c, cj);
            decref(r, rj);
        }
        // The six remaining types (and NULL) -> 0.0.
        let ct = every_type(c);
        let rt = every_type(r);
        for i in 0..ct.len() {
            diff_eq!(
                (c.json_number_value)(ct[i].1).to_bits(),
                (r.json_number_value)(rt[i].1).to_bits(),
                "json_number_value on {}",
                ct[i].0
            );
            if ct[i].0 != "integer" && ct[i].0 != "real" {
                assert_eq!(
                    (c.json_number_value)(ct[i].1).to_bits(),
                    0.0f64.to_bits(),
                    "C: number_value on {} -> +0.0",
                    ct[i].0
                );
            }
            decref(c, ct[i].1);
            decref(r, rt[i].1);
        }
    }
}

#[test]
fn row210_json_string_set_and_setn() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0210_0001);
    unsafe {
        // Valid replacements.
        let mut valid: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b"replacement".to_vec(),
            "héllo日本語".as_bytes().to_vec(),
            vec![b'y'; 3000],
            b"a\0b".to_vec(),
        ];
        for _ in 0..150 {
            valid.push(rng.utf8_string(24).into_bytes());
        }
        for (i, bytes) in valid.iter().enumerate() {
            let cj = (c.json_string)(cs("initial").as_ptr());
            let rj = (r.json_string)(cs("initial").as_ptr());
            let buf = cs_bytes(bytes);
            // json_string_set uses strlen(), so it stops at the first NUL.
            let cret = (c.json_string_set)(cj, buf.as_ptr());
            let rret = (r.json_string_set)(rj, buf.as_ptr());
            diff_eq!(cret, rret, "json_string_set(case {i})");
            cmp(c, r, cj, rj, &format!("string node after set(case {i})"));
            // json_string_setn with the full length.
            let cret = (c.json_string_setn)(cj, buf.as_ptr(), bytes.len());
            let rret = (r.json_string_setn)(rj, buf.as_ptr(), bytes.len());
            diff_eq!(cret, rret, "json_string_setn(case {i}, full len)");
            cmp_free(c, r, cj, rj, &format!("string node after setn(case {i})"));
        }
        // len == 0, len < strlen, len spanning an embedded NUL, len cutting a
        // multi-byte sequence (-> -1, node unchanged).
        let setn_cases: Vec<(&str, Vec<u8>, size_t, bool)> = vec![
            ("len 0", b"abc".to_vec(), 0, true),
            ("len < strlen", b"abcdef".to_vec(), 2, true),
            ("spanning NUL", b"a\0b".to_vec(), 3, true),
            ("cut 2-byte", "é".as_bytes().to_vec(), 1, false),
            ("cut 3-byte", "日".as_bytes().to_vec(), 2, false),
            ("cut 4-byte", vec![0xF0, 0x9F, 0x98, 0x80], 2, false),
        ];
        for (name, bytes, len, ok) in &setn_cases {
            let cj = (c.json_string)(cs("before").as_ptr());
            let rj = (r.json_string)(cs("before").as_ptr());
            let buf = cs_bytes(bytes);
            let cret = (c.json_string_setn)(cj, buf.as_ptr(), *len);
            let rret = (r.json_string_setn)(rj, buf.as_ptr(), *len);
            diff_eq!(cret, rret, "json_string_setn({name})");
            assert_eq!(cret == 0, *ok, "C: setn({name}) expectation");
            cmp_free(c, r, cj, rj, &format!("string node after setn({name})"));
        }
        // value == NULL -> -1; invalid UTF-8 -> -1 with the node unchanged.
        let cj = (c.json_string)(cs("keepme").as_ptr());
        let rj = (r.json_string)(cs("keepme").as_ptr());
        diff_eq!(
            (c.json_string_set)(cj, std::ptr::null()),
            (r.json_string_set)(rj, std::ptr::null()),
            "json_string_set(NULL)"
        );
        diff_eq!(
            (c.json_string_setn)(cj, std::ptr::null(), 3),
            (r.json_string_setn)(rj, std::ptr::null(), 3),
            "json_string_setn(NULL)"
        );
        for (name, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            diff_eq!(
                (c.json_string_set)(cj, buf.as_ptr()),
                (r.json_string_set)(rj, buf.as_ptr()),
                "json_string_set(bad {name})"
            );
            diff_eq!(
                (c.json_string_setn)(cj, buf.as_ptr(), bytes.len()),
                (r.json_string_setn)(rj, buf.as_ptr(), bytes.len()),
                "json_string_setn(bad {name})"
            );
            cmp(c, r, cj, rj, &format!("node unchanged after bad {name}"));
        }
        assert_eq!(
            (c.json_string_length)(cj),
            6,
            "C: node untouched by rejected sets"
        );
        cmp_free(c, r, cj, rj, "row210 node");

        // Non-string node -> -1.
        let repl = cs("x");
        let ct = every_type(c);
        let rt = every_type(r);
        for i in 0..ct.len() {
            if ct[i].0 == "string" {
                continue;
            }
            diff_eq!(
                (c.json_string_set)(ct[i].1, repl.as_ptr()),
                (r.json_string_set)(rt[i].1, repl.as_ptr()),
                "json_string_set on {}",
                ct[i].0
            );
            diff_eq!(
                (c.json_string_setn)(ct[i].1, repl.as_ptr(), 1),
                (r.json_string_setn)(rt[i].1, repl.as_ptr(), 1),
                "json_string_setn on {}",
                ct[i].0
            );
            assert_eq!(
                (c.json_string_set)(ct[i].1, repl.as_ptr()),
                -1,
                "C: set on {} -> -1",
                ct[i].0
            );
        }
        for i in 0..ct.len() {
            decref(c, ct[i].1);
            decref(r, rt[i].1);
        }
    }
}

#[test]
fn row211_json_string_set_nocheck_variants() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0211_0001);
    unsafe {
        // Invalid UTF-8 replacements are accepted.
        for (name, bytes) in bad_utf8() {
            let cj = (c.json_string)(cs("before").as_ptr());
            let rj = (r.json_string)(cs("before").as_ptr());
            let buf = cs_bytes(&bytes);
            let cret = (c.json_string_set_nocheck)(cj, buf.as_ptr());
            let rret = (r.json_string_set_nocheck)(rj, buf.as_ptr());
            diff_eq!(cret, rret, "json_string_set_nocheck({name})");
            assert_eq!(cret, 0, "C: _nocheck accepts {name}");
            cmp(c, r, cj, rj, &format!("node after set_nocheck({name})"));
            let cret = (c.json_string_setn_nocheck)(cj, buf.as_ptr(), bytes.len());
            let rret = (r.json_string_setn_nocheck)(rj, buf.as_ptr(), bytes.len());
            diff_eq!(cret, rret, "json_string_setn_nocheck({name})");
            cmp_free(c, r, cj, rj, &format!("node after setn_nocheck({name})"));
        }
        // Embedded NUL, len == 0, random bytes.
        let mut cases: Vec<(Vec<u8>, size_t)> = vec![
            (b"a\0b".to_vec(), 3),
            (b"abc".to_vec(), 0),
            (b"".to_vec(), 0),
            (b"\0".to_vec(), 1),
        ];
        for _ in 0..150 {
            let n = rng.below(20);
            let bytes: Vec<u8> = (0..n).map(|_| rng.next_u32() as u8).collect();
            let l = rng.below(bytes.len() + 1);
            cases.push((bytes, l));
        }
        for (i, (bytes, len)) in cases.iter().enumerate() {
            let cj = (c.json_string)(cs("before").as_ptr());
            let rj = (r.json_string)(cs("before").as_ptr());
            let buf = cs_bytes(bytes);
            diff_eq!(
                (c.json_string_setn_nocheck)(cj, buf.as_ptr(), *len),
                (r.json_string_setn_nocheck)(rj, buf.as_ptr(), *len),
                "setn_nocheck(case {i}, len {len})"
            );
            cmp_free(c, r, cj, rj, &format!("node after setn_nocheck(case {i})"));
        }
        // Set a string from a PREFIX of its own current buffer: jsonp_strndup
        // copies first, then the old buffer is freed, so this is well-defined.
        for len in [0usize, 1, 3, 6] {
            let cj = (c.json_string)(cs("selfref").as_ptr());
            let rj = (r.json_string)(cs("selfref").as_ptr());
            let cown = (c.json_string_value)(cj);
            let rown = (r.json_string_value)(rj);
            diff_eq!(
                (c.json_string_setn_nocheck)(cj, cown, len),
                (r.json_string_setn_nocheck)(rj, rown, len),
                "setn_nocheck from own buffer, len {len}"
            );
            cmp_free(c, r, cj, rj, &format!("node after self-set len {len}"));
        }
        // value == NULL -> -1; non-string node -> -1.
        let cj = (c.json_string)(cs("keep").as_ptr());
        let rj = (r.json_string)(cs("keep").as_ptr());
        diff_eq!(
            (c.json_string_set_nocheck)(cj, std::ptr::null()),
            (r.json_string_set_nocheck)(rj, std::ptr::null()),
            "set_nocheck(NULL)"
        );
        diff_eq!(
            (c.json_string_setn_nocheck)(cj, std::ptr::null(), 4),
            (r.json_string_setn_nocheck)(rj, std::ptr::null(), 4),
            "setn_nocheck(NULL)"
        );
        cmp_free(c, r, cj, rj, "node unchanged by NULL value");
        let repl = cs("x");
        let ct = every_type(c);
        let rt = every_type(r);
        for i in 0..ct.len() {
            if ct[i].0 == "string" {
                continue;
            }
            diff_eq!(
                (c.json_string_set_nocheck)(ct[i].1, repl.as_ptr()),
                (r.json_string_set_nocheck)(rt[i].1, repl.as_ptr()),
                "set_nocheck on {}",
                ct[i].0
            );
            diff_eq!(
                (c.json_string_setn_nocheck)(ct[i].1, repl.as_ptr(), 1),
                (r.json_string_setn_nocheck)(rt[i].1, repl.as_ptr(), 1),
                "setn_nocheck on {}",
                ct[i].0
            );
            decref(c, ct[i].1);
            decref(r, rt[i].1);
        }
    }
}

#[test]
fn row212_json_integer_set_and_real_set() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0212_0001);
    unsafe {
        let mut ints: Vec<i64> = vec![0, 1, -1, i32::MAX as i64, i32::MIN as i64, i64::MAX, i64::MIN];
        for _ in 0..200 {
            ints.push(rng.json_int());
        }
        let cj = (c.json_integer)(0);
        let rj = (r.json_integer)(0);
        for v in &ints {
            let cret = (c.json_integer_set)(cj, *v);
            let rret = (r.json_integer_set)(rj, *v);
            diff_eq!(cret, rret, "json_integer_set({v})");
            assert_eq!(cret, 0, "C: integer_set on an integer node -> 0");
            cmp(c, r, cj, rj, &format!("integer node after set({v})"));
        }
        cmp_free(c, r, cj, rj, "row212 integer node");

        let mut reals: Vec<f64> = vec![0.0, -0.0, 1.5, f64::MAX, f64::MIN_POSITIVE, -1e300];
        for _ in 0..200 {
            reals.push(rng.real());
        }
        let cj = (c.json_real)(1.0);
        let rj = (r.json_real)(1.0);
        for v in &reals {
            let cret = (c.json_real_set)(cj, *v);
            let rret = (r.json_real_set)(rj, *v);
            diff_eq!(cret, rret, "json_real_set({v:e})");
            assert_eq!(cret, 0, "C: real_set with a finite value -> 0");
            cmp(c, r, cj, rj, &format!("real node after set({v:e})"));
        }
        // NAN / +-INFINITY -> -1 and the value must be UNCHANGED.
        let zero = 0.0f64;
        for (name, bad) in [
            ("NAN", f64::NAN),
            ("INFINITY", f64::INFINITY),
            ("-INFINITY", f64::NEG_INFINITY),
            ("0/0", zero / zero),
        ] {
            (c.json_real_set)(cj, 42.5);
            (r.json_real_set)(rj, 42.5);
            let cret = (c.json_real_set)(cj, bad);
            let rret = (r.json_real_set)(rj, bad);
            diff_eq!(cret, rret, "json_real_set({name})");
            assert_eq!(cret, -1, "C: real_set({name}) -> -1");
            assert_eq!(
                (c.json_real_value)(cj),
                42.5,
                "C: value unchanged after real_set({name})"
            );
            cmp(c, r, cj, rj, &format!("real node unchanged after {name}"));
        }
        cmp_free(c, r, cj, rj, "row212 real node");

        // Wrong node types -> -1.
        let ct = every_type(c);
        let rt = every_type(r);
        for i in 0..ct.len() {
            if ct[i].0 != "integer" {
                diff_eq!(
                    (c.json_integer_set)(ct[i].1, 5),
                    (r.json_integer_set)(rt[i].1, 5),
                    "json_integer_set on {}",
                    ct[i].0
                );
                assert_eq!(
                    (c.json_integer_set)(ct[i].1, 5),
                    -1,
                    "C: integer_set on {} -> -1",
                    ct[i].0
                );
            }
            if ct[i].0 != "real" {
                diff_eq!(
                    (c.json_real_set)(ct[i].1, 5.0),
                    (r.json_real_set)(rt[i].1, 5.0),
                    "json_real_set on {}",
                    ct[i].0
                );
                assert_eq!(
                    (c.json_real_set)(ct[i].1, 5.0),
                    -1,
                    "C: real_set on {} -> -1",
                    ct[i].0
                );
            }
            decref(c, ct[i].1);
            decref(r, rt[i].1);
        }
    }
}

// ===========================================================================
// Rows 213-218 — json_equal
// ===========================================================================

#[test]
fn row213_json_equal_types_and_null() {
    let (c, r) = both();
    unsafe {
        let ct = every_type(c);
        let rt = every_type(r);
        // NULL on either side (and both) -> 0.
        for i in 0..ct.len() {
            diff_eq!(
                (c.json_equal)(std::ptr::null(), ct[i].1),
                (r.json_equal)(std::ptr::null(), rt[i].1),
                "json_equal(NULL, {})",
                ct[i].0
            );
            diff_eq!(
                (c.json_equal)(ct[i].1, std::ptr::null()),
                (r.json_equal)(rt[i].1, std::ptr::null()),
                "json_equal({}, NULL)",
                ct[i].0
            );
            assert_eq!(
                (c.json_equal)(std::ptr::null(), ct[i].1),
                0,
                "C: NULL is never equal"
            );
        }
        diff_eq!(
            (c.json_equal)(std::ptr::null(), std::ptr::null()),
            (r.json_equal)(std::ptr::null(), std::ptr::null()),
            "json_equal(NULL, NULL)"
        );

        // The full 8x8 (plus NULL) matrix, using two independently built values
        // of each type: mismatched types must give 0, matched types the
        // structural answer.
        let ct2 = every_type(c);
        let rt2 = every_type(r);
        for i in 0..ct.len() {
            for j in 0..ct2.len() {
                diff_eq!(
                    (c.json_equal)(ct[i].1, ct2[j].1),
                    (r.json_equal)(rt[i].1, rt2[j].1),
                    "json_equal({}, {})",
                    ct[i].0,
                    ct2[j].0
                );
                if ct[i].0 != ct2[j].0 {
                    assert_eq!(
                        (c.json_equal)(ct[i].1, ct2[j].1),
                        0,
                        "C: different types are never equal ({} vs {})",
                        ct[i].0,
                        ct2[j].0
                    );
                }
            }
            // Pointer-identity shortcut: every type equals itself.
            diff_eq!(
                (c.json_equal)(ct[i].1, ct[i].1),
                (r.json_equal)(rt[i].1, rt[i].1),
                "json_equal({0}, {0}) identity",
                ct[i].0
            );
            if ct[i].0 != "NULL" {
                assert_eq!(
                    (c.json_equal)(ct[i].1, ct[i].1),
                    1,
                    "C: {} equals itself",
                    ct[i].0
                );
            }
        }
        // The three singletons against themselves and each other.
        let csing = [(c.json_true)(), (c.json_false)(), (c.json_null)()];
        let rsing = [(r.json_true)(), (r.json_false)(), (r.json_null)()];
        let names = ["true", "false", "null"];
        for i in 0..3 {
            for j in 0..3 {
                diff_eq!(
                    (c.json_equal)(csing[i], csing[j]),
                    (r.json_equal)(rsing[i], rsing[j]),
                    "json_equal({}, {})",
                    names[i],
                    names[j]
                );
                assert_eq!(
                    (c.json_equal)(csing[i], csing[j]),
                    (i == j) as c_int,
                    "C: singleton equality {} vs {}",
                    names[i],
                    names[j]
                );
            }
        }
        for i in 0..ct.len() {
            decref(c, ct[i].1);
            decref(r, rt[i].1);
            decref(c, ct2[i].1);
            decref(r, rt2[i].1);
        }
    }
}

#[test]
fn row214_json_equal_strings() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0214_0001);
    unsafe {
        let mut pairs: Vec<(Vec<u8>, size_t, Vec<u8>, size_t)> = vec![
            (b"abc".to_vec(), 3, b"abc".to_vec(), 3),  // equal
            (b"abc".to_vec(), 3, b"abd".to_vec(), 3),  // same length, differing byte
            (b"abc".to_vec(), 3, b"abcd".to_vec(), 4), // same prefix, different length
            (b"".to_vec(), 0, b"".to_vec(), 0),        // both empty
            (b"".to_vec(), 0, b"a".to_vec(), 1),
            // Differ only AFTER an embedded NUL: memcmp over `length` sees it.
            (b"a\0b".to_vec(), 3, b"a\0c".to_vec(), 3),
            (b"a\0b".to_vec(), 3, b"a\0b".to_vec(), 3),
            (b"a\0".to_vec(), 2, b"a".to_vec(), 1),
            (vec![0xFF, 0x00, 0x01], 3, vec![0xFF, 0x00, 0x02], 3),
        ];
        for _ in 0..300 {
            let n = rng.below(10);
            let a: Vec<u8> = (0..n).map(|_| *rng.choice(&[0u8, 1, b'a', b'b', 0xFF])).collect();
            let mut b = a.clone();
            match rng.below(3) {
                0 => {}
                1 => {
                    if !b.is_empty() {
                        let i = rng.below(b.len());
                        b[i] = b[i].wrapping_add(1);
                    }
                }
                _ => b.push(0),
            }
            let (la, lb) = (a.len(), b.len());
            pairs.push((a, la, b, lb));
        }
        for (i, (a, la, b, lb)) in pairs.iter().enumerate() {
            let abuf = cs_bytes(a);
            let bbuf = cs_bytes(b);
            let ca = (c.json_stringn_nocheck)(abuf.as_ptr(), *la);
            let cb = (c.json_stringn_nocheck)(bbuf.as_ptr(), *lb);
            let ra = (r.json_stringn_nocheck)(abuf.as_ptr(), *la);
            let rb = (r.json_stringn_nocheck)(bbuf.as_ptr(), *lb);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "json_equal(strings case {i}: {:?} vs {:?})",
                B(a.clone()),
                B(b.clone())
            );
            // Ground truth: byte-wise comparison over the stored lengths.
            assert_eq!(
                (c.json_equal)(ca, cb),
                (a[..*la] == b[..*lb]) as c_int,
                "C: string equality is memcmp over length (case {i})"
            );
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }
    }
}

#[test]
fn row215_json_equal_numbers() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0215_0001);
    unsafe {
        let mut ipairs: Vec<(i64, i64)> = vec![
            (0, 0),
            (1, 1),
            (1, 2),
            (i64::MIN, i64::MAX),
            (i64::MAX, i64::MAX),
            (-1, 1),
        ];
        for _ in 0..250 {
            let a = rng.json_int();
            let b = if rng.bool() { a } else { rng.json_int() };
            ipairs.push((a, b));
        }
        for (a, b) in &ipairs {
            let ca = (c.json_integer)(*a);
            let cb = (c.json_integer)(*b);
            let ra = (r.json_integer)(*a);
            let rb = (r.json_integer)(*b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "json_equal({a}, {b})"
            );
            assert_eq!((c.json_equal)(ca, cb), (a == b) as c_int, "C: integer equality");
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }

        let mut rpairs: Vec<(f64, f64)> = vec![
            (0.0, 0.0),
            (0.0, -0.0), // equal under ==
            (-0.0, -0.0),
            (1.5, 1.5),
            (1.5, 1.6),
            // Differ in the last bit.
            (1.0, f64::from_bits(1.0f64.to_bits() + 1)),
            (f64::MAX, f64::from_bits(f64::MAX.to_bits() - 1)),
            (5e-324, 1e-323),
        ];
        for _ in 0..250 {
            let a = rng.real();
            let b = match rng.below(3) {
                0 => a,
                1 => f64::from_bits(a.to_bits() ^ 1),
                _ => rng.real(),
            };
            if b.is_finite() {
                rpairs.push((a, b));
            }
        }
        for (a, b) in &rpairs {
            let ca = (c.json_real)(*a);
            let cb = (c.json_real)(*b);
            let ra = (r.json_real)(*a);
            let rb = (r.json_real)(*b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "json_equal({a:e}, {b:e})"
            );
            assert_eq!(
                (c.json_equal)(ca, cb),
                (a == b) as c_int,
                "C: real equality is `==`, so 0.0 == -0.0"
            );
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }
    }
}

/// `[depth, [depth-1, [... leaf]]]` — a 5-level-deep array chain.
fn nest_arr(depth: usize, leaf: V) -> V {
    if depth == 0 {
        leaf
    } else {
        V::Arr(vec![V::Int(depth as i64), nest_arr(depth - 1, leaf)])
    }
}

#[test]
fn row216_json_equal_arrays() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0216_0001);
    unsafe {
        let cases: Vec<(&str, V, V, c_int)> = vec![
            ("both empty", V::Arr(vec![]), V::Arr(vec![]), 1),
            ("different sizes", V::Arr(vec![V::Int(1)]), V::Arr(vec![]), 0),
            (
                "same elements",
                V::Arr(vec![V::Int(1), V::Str(b"a".to_vec()), V::True]),
                V::Arr(vec![V::Int(1), V::Str(b"a".to_vec()), V::True]),
                1,
            ),
            (
                "differ at index 0",
                V::Arr(vec![V::Int(1), V::Int(2), V::Int(3)]),
                V::Arr(vec![V::Int(9), V::Int(2), V::Int(3)]),
                0,
            ),
            (
                "differ in the middle",
                V::Arr(vec![V::Int(1), V::Int(2), V::Int(3)]),
                V::Arr(vec![V::Int(1), V::Int(9), V::Int(3)]),
                0,
            ),
            (
                "differ at the last",
                V::Arr(vec![V::Int(1), V::Int(2), V::Int(3)]),
                V::Arr(vec![V::Int(1), V::Int(2), V::Int(9)]),
                0,
            ),
            ("5 deep equal", nest_arr(5, V::Int(7)), nest_arr(5, V::Int(7)), 1),
            (
                "5 deep differing leaf",
                nest_arr(5, V::Int(7)),
                nest_arr(5, V::Int(8)),
                0,
            ),
            (
                "5 deep differing leaf type",
                nest_arr(5, V::Int(7)),
                nest_arr(5, V::Real(7.0)),
                0,
            ),
        ];
        for (name, a, b, want) in &cases {
            let ca = build(c, a);
            let cb = build(c, b);
            let ra = build(r, a);
            let rb = build(r, b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "json_equal(arrays: {name})"
            );
            assert_eq!(
                (c.json_equal)(ca, cb),
                *want,
                "C: array equality expectation for {name}"
            );
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }
        // Randomised array pairs.
        for trial in 0..150 {
            let a = V::Arr((0..rng.below(6)).map(|_| rand_value(&mut rng, 3)).collect());
            let b = if rng.bool() {
                a.clone()
            } else {
                V::Arr((0..rng.below(6)).map(|_| rand_value(&mut rng, 3)).collect())
            };
            let ca = build(c, &a);
            let cb = build(c, &b);
            let ra = build(r, &a);
            let rb = build(r, &b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "trial {trial}: random array equality"
            );
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }
    }
}

#[test]
fn row217_json_equal_objects() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0217_0001);
    unsafe {
        // "same size, disjoint keys" is the `value2 == NULL` path:
        // json_object_getn returns NULL and json_equal(v, NULL) -> 0.
        let cases: Vec<(&str, V, V, c_int)> = vec![
            ("both empty", V::Obj(vec![]), V::Obj(vec![]), 1),
            (
                "different sizes",
                V::Obj(vec![(b"a".to_vec(), V::Int(1))]),
                V::Obj(vec![]),
                0,
            ),
            (
                "same keys and values",
                V::Obj(vec![(b"a".to_vec(), V::Int(1)), (b"b".to_vec(), V::True)]),
                V::Obj(vec![(b"a".to_vec(), V::Int(1)), (b"b".to_vec(), V::True)]),
                1,
            ),
            (
                "same size, disjoint keys",
                V::Obj(vec![(b"a".to_vec(), V::Int(1)), (b"b".to_vec(), V::Int(2))]),
                V::Obj(vec![(b"c".to_vec(), V::Int(1)), (b"d".to_vec(), V::Int(2))]),
                0,
            ),
            (
                "same keys, different values",
                V::Obj(vec![(b"a".to_vec(), V::Int(1))]),
                V::Obj(vec![(b"a".to_vec(), V::Int(2))]),
                0,
            ),
            (
                "keys differing past an embedded NUL",
                V::Obj(vec![(b"a\0b".to_vec(), V::Int(1))]),
                V::Obj(vec![(b"a\0c".to_vec(), V::Int(1))]),
                0,
            ),
            (
                "keys equal including an embedded NUL",
                V::Obj(vec![(b"a\0b".to_vec(), V::Int(1))]),
                V::Obj(vec![(b"a\0b".to_vec(), V::Int(1))]),
                1,
            ),
        ];
        for (name, a, b, want) in &cases {
            let ca = build(c, a);
            let cb = build(c, b);
            let ra = build(r, a);
            let rb = build(r, b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "json_equal(objects: {name})"
            );
            assert_eq!((c.json_equal)(ca, cb), *want, "C: expectation for {name}");
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }

        // 12-key objects built in DIFFERENT insertion orders must compare equal.
        for trial in 0..30 {
            let mut order: Vec<usize> = (0..12).collect();
            let ca = (c.json_object)();
            let ra = (r.json_object)();
            for i in &order {
                let k = cs(&format!("key{i}"));
                (c.json_object_set_new)(ca, k.as_ptr(), (c.json_integer)(*i as i64));
                (r.json_object_set_new)(ra, k.as_ptr(), (r.json_integer)(*i as i64));
            }
            for i in (1..order.len()).rev() {
                let j = rng.below(i + 1);
                order.swap(i, j);
            }
            let cb = (c.json_object)();
            let rb = (r.json_object)();
            for i in &order {
                let k = cs(&format!("key{i}"));
                (c.json_object_set_new)(cb, k.as_ptr(), (c.json_integer)(*i as i64));
                (r.json_object_set_new)(rb, k.as_ptr(), (r.json_integer)(*i as i64));
            }
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "trial {trial}: 12-key objects in different insertion orders"
            );
            assert_eq!(
                (c.json_equal)(ca, cb),
                1,
                "C: insertion order does not affect equality"
            );
            // The iteration orders themselves are still compared, since they are
            // the hashtable fingerprint.
            cmp(c, r, ca, ra, &format!("trial {trial}: object a"));
            cmp(c, r, cb, rb, &format!("trial {trial}: object b"));
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }

        // Randomised object pairs.
        for trial in 0..150 {
            let a = V::Obj(
                (0..rng.below(7))
                    .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 3)))
                    .collect(),
            );
            let b = if rng.bool() {
                a.clone()
            } else {
                V::Obj(
                    (0..rng.below(7))
                        .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 3)))
                        .collect(),
                )
            };
            let ca = build(c, &a);
            let cb = build(c, &b);
            let ra = build(r, &a);
            let rb = build(r, &b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "trial {trial}: random object equality"
            );
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }
    }
}

/// Object of arrays of objects, 4+ levels, with all eight leaf types.
fn mixed_deep() -> V {
    let leaves: Vec<V> = all_eight().into_iter().map(|(_, v)| v).collect();
    V::Obj(vec![
        (
            b"l1".to_vec(),
            V::Arr(vec![
                V::Obj(vec![(b"l3".to_vec(), V::Arr(leaves))]),
                V::Obj(vec![(b"l3b".to_vec(), V::Arr(vec![V::Int(1), V::Null]))]),
            ]),
        ),
        (b"l1b".to_vec(), V::Str("mixed \u{e9} \u{65e5}".as_bytes().to_vec())),
    ])
}

#[test]
fn row218_json_equal_mixed_deep_structures() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0218_0001);
    unsafe {
        let deep = mixed_deep();
        let ca = build(c, &deep);
        let cb = build(c, &deep);
        let ra = build(r, &deep);
        let rb = build(r, &deep);
        diff_eq!(
            (c.json_equal)(ca, cb),
            (r.json_equal)(ra, rb),
            "json_equal(identical deep trees)"
        );
        assert_eq!((c.json_equal)(ca, cb), 1, "C: identical deep trees are equal");
        cmp(c, r, ca, ra, "deep tree a");
        cmp(c, r, cb, rb, "deep tree b");

        // Differ only at the deepest leaf.
        let mut deep2 = deep.clone();
        if let V::Obj(top) = &mut deep2 {
            if let V::Arr(arr) = &mut top[0].1 {
                if let V::Obj(o) = &mut arr[0] {
                    if let V::Arr(l) = &mut o[0].1 {
                        l[3] = V::Int(999_999); // the "integer" leaf
                    }
                }
            }
        }
        let cd2 = build(c, &deep2);
        let rd2 = build(r, &deep2);
        diff_eq!(
            (c.json_equal)(ca, cd2),
            (r.json_equal)(ra, rd2),
            "json_equal(trees differing at the deepest leaf)"
        );
        assert_eq!(
            (c.json_equal)(ca, cd2),
            0,
            "C: a differing deep leaf makes trees unequal"
        );
        decref(c, cd2);
        decref(r, rd2);

        // A shared-subtree (DAG) variant compared against the expanded tree:
        // json_equal is purely structural, so they must be equal.
        let cshared = (c.json_array)();
        let rshared = (r.json_array)();
        (c.json_array_append_new)(cshared, (c.json_integer)(1));
        (r.json_array_append_new)(rshared, (r.json_integer)(1));
        let cdag = (c.json_object)();
        let rdag = (r.json_object)();
        let cexp = (c.json_object)();
        let rexp = (r.json_object)();
        for k in ["x", "y"] {
            (c.json_object_set_new)(cdag, cs(k).as_ptr(), incref(cshared));
            (r.json_object_set_new)(rdag, cs(k).as_ptr(), incref(rshared));
            let cfresh = (c.json_array)();
            let rfresh = (r.json_array)();
            (c.json_array_append_new)(cfresh, (c.json_integer)(1));
            (r.json_array_append_new)(rfresh, (r.json_integer)(1));
            (c.json_object_set_new)(cexp, cs(k).as_ptr(), cfresh);
            (r.json_object_set_new)(rexp, cs(k).as_ptr(), rfresh);
        }
        diff_eq!(
            (c.json_equal)(cdag, cexp),
            (r.json_equal)(rdag, rexp),
            "json_equal(DAG vs expanded tree)"
        );
        assert_eq!(
            (c.json_equal)(cdag, cexp),
            1,
            "C: sharing does not affect structural equality"
        );
        cmp(c, r, cdag, rdag, "DAG object");
        decref(c, cdag);
        decref(r, rdag);
        decref(c, cexp);
        decref(r, rexp);
        decref(c, cshared);
        decref(r, rshared);
        decref(c, ca);
        decref(c, cb);
        decref(r, ra);
        decref(r, rb);

        // Randomised deep trees, sometimes identical, sometimes independent.
        for trial in 0..120 {
            let a = rand_value(&mut rng, 5);
            let b = if rng.bool() { a.clone() } else { rand_value(&mut rng, 5) };
            let ca = build(c, &a);
            let cb = build(c, &b);
            let ra = build(r, &a);
            let rb = build(r, &b);
            diff_eq!(
                (c.json_equal)(ca, cb),
                (r.json_equal)(ra, rb),
                "trial {trial}: random deep equality"
            );
            decref(c, ca);
            decref(c, cb);
            decref(r, ra);
            decref(r, rb);
        }
    }
}

// ===========================================================================
// Rows 219-224 — json_copy / json_deep_copy / do_deep_copy
// ===========================================================================

#[test]
fn row219_json_copy_is_shallow() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0219_0001);
    unsafe {
        // json == NULL -> NULL
        diff_eq!(
            (c.json_copy)(std::ptr::null_mut()).is_null(),
            (r.json_copy)(std::ptr::null_mut()).is_null(),
            "json_copy(NULL)"
        );
        assert!((c.json_copy)(std::ptr::null_mut()).is_null(), "C: copy(NULL) == NULL");

        // The three singletons come back as the SAME pointer, no allocation, and
        // the refcount stays (size_t)-1.
        for (name, cf, rf) in [
            ("json_true", c.json_true, r.json_true),
            ("json_false", c.json_false, r.json_false),
            ("json_null", c.json_null, r.json_null),
        ] {
            let cj = cf();
            let rj = rf();
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            assert_eq!(cc, cj, "C: json_copy({name}) returns the singleton itself");
            diff_eq!(cc == cj, rc == rj, "json_copy({name}) pointer identity");
            diff_eq!((*cc).refcount, (*rc).refcount, "json_copy({name}) refcount");
            assert_eq!((*cc).refcount, usize::MAX, "C: refcount stays (size_t)-1");
            cmp(c, r, cc, rc, &format!("json_copy({name})"));
        }

        // string / integer / real: new node, equal value.
        for (name, v) in [
            ("string", V::Str(b"copy me\0and more".to_vec())),
            ("integer", V::Int(i64::MIN)),
            ("real", V::Real(-0.0)),
        ] {
            let cj = build(c, &v);
            let rj = build(r, &v);
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            assert_ne!(cc, cj, "C: json_copy({name}) allocates a new node");
            diff_eq!(cc != cj, rc != rj, "json_copy({name}) is a new node");
            diff_eq!(
                (c.json_equal)(cj, cc),
                (r.json_equal)(rj, rc),
                "json_copy({name}) is equal to the original"
            );
            cmp(c, r, cc, rc, &format!("json_copy({name})"));
            cmp(c, r, cj, rj, &format!("original {name} after copy"));
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);
        }

        // Objects (empty / 12 keys) and arrays (empty / 17 elements): shallow,
        // so children are SHARED and their refcount goes to 2.
        for n in [0usize, 1, 12, 17] {
            let (cj, rj) = objs_with_n(c, r, n);
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            cmp(c, r, cc, rc, &format!("json_copy(object with {n} keys)"));
            cmp(c, r, cj, rj, &format!("original object with {n} keys after copy"));
            // Every child pointer is shared.
            let mut it = (c.json_object_iter)(cj);
            let mut all = true;
            while !it.is_null() {
                let kp = (c.json_object_iter_key)(it);
                let kl = (c.json_object_iter_key_len)(it);
                let v = (c.json_object_iter_value)(it);
                all &= v == (c.json_object_getn)(cc, kp, kl);
                assert_eq!((*v).refcount, 2, "C: shallow copy increfs the child");
                it = (c.json_object_iter_next)(cj, it);
            }
            assert!(all, "C: json_copy shares object children");
            let mut it = (r.json_object_iter)(rj);
            let mut allr = true;
            while !it.is_null() {
                let kp = (r.json_object_iter_key)(it);
                let kl = (r.json_object_iter_key_len)(it);
                allr &= (r.json_object_iter_value)(it) == (r.json_object_getn)(rc, kp, kl);
                it = (r.json_object_iter_next)(rj, it);
            }
            diff_eq!(true, allr, "Rust json_copy must share object children (n={n})");
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);

            let (cj, rj) = arrays_with_n(c, r, n);
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            cmp(c, r, cc, rc, &format!("json_copy(array of {n})"));
            cmp(c, r, cj, rj, &format!("original array of {n} after copy"));
            let mut all = true;
            let mut allr = true;
            for i in 0..n {
                all &= (c.json_array_get)(cj, i) == (c.json_array_get)(cc, i);
                allr &= (r.json_array_get)(rj, i) == (r.json_array_get)(rc, i);
                assert_eq!(
                    (*(c.json_array_get)(cj, i)).refcount,
                    2,
                    "C: shallow array copy increfs the element"
                );
            }
            assert!(all, "C: json_copy shares array elements");
            diff_eq!(true, allr, "Rust json_copy must share array elements (n={n})");
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);
        }

        // Randomised trees through json_copy.
        for trial in 0..80 {
            let v = rand_value(&mut rng, 4);
            let cj = build(c, &v);
            let rj = build(r, &v);
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            cmp(c, r, cc, rc, &format!("trial {trial}: json_copy of a random tree"));
            cmp(c, r, cj, rj, &format!("trial {trial}: original after json_copy"));
            diff_eq!(
                (c.json_equal)(cj, cc),
                (r.json_equal)(rj, rc),
                "trial {trial}: copy equals original"
            );
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn row220_json_deep_copy_per_type() {
    let (c, r) = both();
    unsafe {
        // json == NULL -> NULL, for both entry points.
        diff_eq!(
            (c.json_deep_copy)(std::ptr::null()).is_null(),
            (r.json_deep_copy)(std::ptr::null()).is_null(),
            "json_deep_copy(NULL)"
        );
        assert!((c.json_deep_copy)(std::ptr::null()).is_null(), "C: NULL -> NULL");
        {
            let mut cht = Ht::new(c);
            let mut rht = Ht::new(r);
            diff_eq!(
                (c.do_deep_copy)(std::ptr::null(), cht.p()).is_null(),
                (r.do_deep_copy)(std::ptr::null(), rht.p()).is_null(),
                "do_deep_copy(NULL)"
            );
            diff_eq!(cht.t.size, rht.t.size, "parents untouched by do_deep_copy(NULL)");
        }

        for (name, v) in all_eight() {
            let cj = build(c, &v);
            let rj = build(r, &v);
            let cc = (c.json_deep_copy)(cj);
            let rc = (r.json_deep_copy)(rj);
            let is_singleton = matches!(v, V::True | V::False | V::Null);
            if is_singleton {
                assert_eq!(cc, cj, "C: deep_copy({name}) returns the singleton itself");
            } else {
                assert_ne!(cc, cj, "C: deep_copy({name}) allocates a new node");
            }
            diff_eq!(cc == cj, rc == rj, "deep_copy({name}) pointer identity");
            diff_eq!(
                (c.json_equal)(cj, cc),
                (r.json_equal)(rj, rc),
                "deep_copy({name}) equality"
            );
            cmp(c, r, cc, rc, &format!("json_deep_copy({name})"));
            cmp(c, r, cj, rj, &format!("original {name} after deep_copy"));

            // do_deep_copy with a caller-supplied parents set behaves the same
            // and drains the set on the way out.
            let mut cht = Ht::new(c);
            let mut rht = Ht::new(r);
            let cc2 = (c.do_deep_copy)(cj, cht.p());
            let rc2 = (r.do_deep_copy)(rj, rht.p());
            diff_eq!(cht.t.size, rht.t.size, "do_deep_copy({name}) parents size after");
            assert_eq!(cht.t.size, 0, "C: parents set drained after do_deep_copy");
            cmp(c, r, cc2, rc2, &format!("do_deep_copy({name})"));
            drop(cht);
            drop(rht);
            decref(c, cc2);
            decref(r, rc2);
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);
        }
        // Empty object and empty array.
        for (name, v) in [("empty object", V::Obj(vec![])), ("empty array", V::Arr(vec![]))] {
            let cj = build(c, &v);
            let rj = build(r, &v);
            let cc = (c.json_deep_copy)(cj);
            let rc = (r.json_deep_copy)(rj);
            cmp(c, r, cc, rc, &format!("json_deep_copy({name})"));
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn row221_json_deep_copy_nested() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0221_0001);
    unsafe {
        // 5 levels deep, all eight leaf types at the bottom.
        let mut trees: Vec<V> = vec![
            mixed_deep(),
            nest_arr(5, V::Arr(all_eight().into_iter().map(|(_, v)| v).collect())),
            V::Obj(
                (0..12)
                    .map(|i| {
                        (
                            format!("key{i}").into_bytes(),
                            V::Arr(vec![V::Int(i), V::Obj(vec![(b"n".to_vec(), V::Real(i as f64))])]),
                        )
                    })
                    .collect(),
            ),
        ];
        for _ in 0..80 {
            trees.push(rand_value(&mut rng, 5));
        }

        for (i, t) in trees.iter().enumerate() {
            let cj = build(c, t);
            let rj = build(r, t);
            let cc = (c.json_deep_copy)(cj);
            let rc = (r.json_deep_copy)(rj);
            diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(tree {i}) null-ness");
            assert!(!cc.is_null(), "C: deep_copy of an acyclic tree succeeds");
            diff_eq!(
                (c.json_equal)(cj, cc),
                (r.json_equal)(rj, rc),
                "json_equal(orig, deep copy) for tree {i}"
            );
            assert_eq!((c.json_equal)(cj, cc), 1, "C: deep copy is equal to original");
            cmp(c, r, cc, rc, &format!("deep copy of tree {i}"));
            cmp(c, r, cj, rj, &format!("original tree {i} after deep_copy"));
            // No non-singleton child pointer may be shared.
            let cshared = count_shared(c, cj, cc);
            let rshared = count_shared(r, rj, rc);
            diff_eq!(cshared, rshared, "shared pointer count for tree {i}");
            assert_eq!(cshared, 0, "C: deep copy shares no non-singleton node");
            decref(c, cc);
            decref(r, rc);
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn row222_json_deep_copy_shared_subtrees() {
    let (c, r) = both();
    unsafe {
        // The same ARRAY at two array indices.
        let cinner = (c.json_array)();
        let rinner = (r.json_array)();
        for i in 0..3 {
            (c.json_array_append_new)(cinner, (c.json_integer)(i));
            (r.json_array_append_new)(rinner, (r.json_integer)(i));
        }
        let couter = (c.json_array)();
        let router = (r.json_array)();
        for _ in 0..3 {
            (c.json_array_append_new)(couter, incref(cinner));
            (r.json_array_append_new)(router, incref(rinner));
        }
        let cc = (c.json_deep_copy)(couter);
        let rc = (r.json_deep_copy)(router);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(array DAG) null-ness");
        assert!(!cc.is_null(), "C: a shared subtree is not a cycle");
        cmp(c, r, cc, rc, "deep copy of an array DAG");
        // The copies are INDEPENDENT: three distinct new arrays.
        let c0 = (c.json_array_get)(cc, 0);
        let c1 = (c.json_array_get)(cc, 1);
        let r0 = (r.json_array_get)(rc, 0);
        let r1 = (r.json_array_get)(rc, 1);
        assert_ne!(c0, c1, "C: the two copies of a shared subtree are independent");
        diff_eq!(c0 != c1, r0 != r1, "independence of DAG copies");
        diff_eq!(count_shared(c, couter, cc), count_shared(r, router, rc), "DAG sharing");
        assert_eq!(count_shared(c, couter, cc), 0, "C: no node is shared with the original");
        decref(c, cc);
        decref(r, rc);
        decref(c, couter);
        decref(r, router);
        decref(c, cinner);
        decref(r, rinner);

        // The same OBJECT under two object keys.
        let cshared = (c.json_object)();
        let rshared = (r.json_object)();
        (c.json_object_set_new)(cshared, cs("v").as_ptr(), (c.json_integer)(9));
        (r.json_object_set_new)(rshared, cs("v").as_ptr(), (r.json_integer)(9));
        let cobj = (c.json_object)();
        let robj = (r.json_object)();
        for k in ["a", "b", "c"] {
            (c.json_object_set_new)(cobj, cs(k).as_ptr(), incref(cshared));
            (r.json_object_set_new)(robj, cs(k).as_ptr(), incref(rshared));
        }
        let cc = (c.json_deep_copy)(cobj);
        let rc = (r.json_deep_copy)(robj);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(object DAG) null-ness");
        assert!(!cc.is_null(), "C: object DAG deep-copies fine");
        cmp(c, r, cc, rc, "deep copy of an object DAG");
        let ca = (c.json_object_get)(cc, cs("a").as_ptr());
        let cb = (c.json_object_get)(cc, cs("b").as_ptr());
        let ra = (r.json_object_get)(rc, cs("a").as_ptr());
        let rb = (r.json_object_get)(rc, cs("b").as_ptr());
        assert_ne!(ca, cb, "C: object DAG copies are independent");
        diff_eq!(ca != cb, ra != rb, "independence of object DAG copies");
        diff_eq!(count_shared(c, cobj, cc), count_shared(r, robj, rc), "object DAG sharing");
        assert_eq!(count_shared(c, cobj, cc), 0, "C: nothing shared with the original");
        decref(c, cc);
        decref(r, rc);
        decref(c, cobj);
        decref(r, robj);
        decref(c, cshared);
        decref(r, rshared);

        // A mixed DAG where the same object is reachable through both an array
        // and an object, twice each.
        let cs2 = (c.json_object)();
        let rs2 = (r.json_object)();
        (c.json_object_set_new)(cs2, cs("k").as_ptr(), (c.json_string)(cs("dag").as_ptr()));
        (r.json_object_set_new)(rs2, cs("k").as_ptr(), (r.json_string)(cs("dag").as_ptr()));
        let croot = (c.json_object)();
        let rroot = (r.json_object)();
        let carr = (c.json_array)();
        let rarr = (r.json_array)();
        for _ in 0..2 {
            (c.json_array_append_new)(carr, incref(cs2));
            (r.json_array_append_new)(rarr, incref(rs2));
        }
        (c.json_object_set_new)(croot, cs("arr").as_ptr(), carr);
        (r.json_object_set_new)(rroot, cs("arr").as_ptr(), rarr);
        (c.json_object_set_new)(croot, cs("obj").as_ptr(), incref(cs2));
        (r.json_object_set_new)(rroot, cs("obj").as_ptr(), incref(rs2));
        let cc = (c.json_deep_copy)(croot);
        let rc = (r.json_deep_copy)(rroot);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(mixed DAG) null-ness");
        assert!(!cc.is_null(), "C: mixed DAG deep-copies fine");
        cmp(c, r, cc, rc, "deep copy of a mixed DAG");
        decref(c, cc);
        decref(r, rc);
        decref(c, croot);
        decref(r, rroot);
        decref(c, cs2);
        decref(r, rs2);
    }
}

/// `a = [b]`, `b = [a]` — an indirect array cycle (direct self-insertion is
/// rejected by the C, so the cycle has to go through a second container).
unsafe fn arr_cycle(api: &Api) -> (*mut json_t, *mut json_t) {
    let a = (api.json_array)();
    let b = (api.json_array)();
    (api.json_array_append_new)(a, incref(b));
    (api.json_array_append_new)(b, incref(a));
    (a, b)
}

unsafe fn break_arr_cycle(api: &Api, a: *mut json_t, b: *mut json_t) {
    (api.json_array_clear)(a);
    (api.json_array_clear)(b);
    decref(api, a);
    decref(api, b);
}

#[test]
fn row223_json_deep_copy_rejects_cycles() {
    let (c, r) = both();
    unsafe {
        // Direct self-insertion is impossible through the public API — assert
        // that both libraries reject it, then use indirect cycles.
        let ca = (c.json_array)();
        let ra = (r.json_array)();
        incref(ca);
        incref(ra);
        diff_eq!(
            (c.json_array_append_new)(ca, ca),
            (r.json_array_append_new)(ra, ra),
            "direct array self-insertion is rejected"
        );
        assert_eq!(
            (c.json_array_append_new)(ca, incref(ca)),
            -1,
            "C: json == value -> -1"
        );
        decref(c, ca);
        decref(r, ra);
        decref(c, ca);
        decref(r, ra);

        // (1) array cycle a = [b], b = [a]
        let (ca, cb) = arr_cycle(c);
        let (ra, rb) = arr_cycle(r);
        let cc = (c.json_deep_copy)(ca);
        let rc = (r.json_deep_copy)(ra);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(array cycle)");
        assert!(cc.is_null(), "C: a cycle makes deep_copy return NULL");
        decref(c, cc);
        decref(r, rc);
        // Also from the other end of the cycle.
        let cc = (c.json_deep_copy)(cb);
        let rc = (r.json_deep_copy)(rb);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(array cycle, other end)");
        decref(c, cc);
        decref(r, rc);
        break_arr_cycle(c, ca, cb);
        break_arr_cycle(r, ra, rb);

        // (2) object cycle o["to_b"] = p, p["to_a"] = o
        let (ca, cb) = obj_cycle(c);
        let (ra, rb) = obj_cycle(r);
        let cc = (c.json_deep_copy)(ca);
        let rc = (r.json_deep_copy)(ra);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(object cycle)");
        assert!(cc.is_null(), "C: object cycle -> NULL");
        decref(c, cc);
        decref(r, rc);
        break_cycle(c, ca, cb);
        break_cycle(r, ra, rb);

        // (3) mutual cycle array -> object -> array
        let carr = (c.json_array)();
        let rarr = (r.json_array)();
        let cobj = (c.json_object)();
        let robj = (r.json_object)();
        (c.json_array_append_new)(carr, incref(cobj));
        (r.json_array_append_new)(rarr, incref(robj));
        (c.json_object_set_new)(cobj, cs("back").as_ptr(), incref(carr));
        (r.json_object_set_new)(robj, cs("back").as_ptr(), incref(rarr));
        for (name, cj, rj) in [("from array", carr, rarr), ("from object", cobj, robj)] {
            let cc = (c.json_deep_copy)(cj);
            let rc = (r.json_deep_copy)(rj);
            diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(array<->object cycle, {name})");
            assert!(cc.is_null(), "C: mutual array/object cycle -> NULL ({name})");
            decref(c, cc);
            decref(r, rc);
            // do_deep_copy with a caller set behaves the same and drains it.
            let mut cht = Ht::new(c);
            let mut rht = Ht::new(r);
            let cc = (c.do_deep_copy)(cj, cht.p());
            let rc = (r.do_deep_copy)(rj, rht.p());
            diff_eq!(cc.is_null(), rc.is_null(), "do_deep_copy(cycle, {name})");
            diff_eq!(cht.t.size, rht.t.size, "parents drained after cycle ({name})");
            assert_eq!(cht.t.size, 0, "C: parents drained even on the failure path");
            drop(cht);
            drop(rht);
            decref(c, cc);
            decref(r, rc);
        }
        (c.json_array_clear)(carr);
        (r.json_array_clear)(rarr);
        (c.json_object_clear)(cobj);
        (r.json_object_clear)(robj);
        decref(c, carr);
        decref(r, rarr);
        decref(c, cobj);
        decref(r, robj);
    }
}

#[test]
fn row224_json_deep_copy_cycle_under_valid_prefix() {
    let (c, r) = both();
    unsafe {
        // {"a": {"b": <cycle>}} — the inner failure must propagate all the way
        // out: the partially built `result` is decref'd and NULL is returned.
        let (cca, ccb) = obj_cycle(c);
        let (rca, rcb) = obj_cycle(r);
        let croot = (c.json_object)();
        let rroot = (r.json_object)();
        let cmid = (c.json_object)();
        let rmid = (r.json_object)();
        (c.json_object_set_new)(cmid, cs("b").as_ptr(), incref(cca));
        (r.json_object_set_new)(rmid, cs("b").as_ptr(), incref(rca));
        (c.json_object_set_new)(croot, cs("a").as_ptr(), cmid);
        (r.json_object_set_new)(rroot, cs("a").as_ptr(), rmid);
        // Plus some perfectly copyable siblings, so the prefix really is valid.
        (c.json_object_set_new)(croot, cs("ok").as_ptr(), (c.json_integer)(1));
        (r.json_object_set_new)(rroot, cs("ok").as_ptr(), (r.json_integer)(1));
        let cc = (c.json_deep_copy)(croot);
        let rc = (r.json_deep_copy)(rroot);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy({{a:{{b:cycle}}}})");
        assert!(cc.is_null(), "C: an inner cycle fails the whole deep copy");
        decref(c, cc);
        decref(r, rc);
        // The original must be untouched.
        cmp(c, r, croot, rroot, "original object after failed deep_copy");
        (c.json_object_del)(cmid, cs("b").as_ptr());
        (r.json_object_del)(rmid, cs("b").as_ptr());
        decref(c, croot);
        decref(r, rroot);
        break_cycle(c, cca, ccb);
        break_cycle(r, rca, rcb);

        // [[<cycle>]] — the array flavour: json_array_append_new(result, NULL)
        // returns -1, result is decref'd, NULL propagates.
        let (cca, ccb) = arr_cycle(c);
        let (rca, rcb) = arr_cycle(r);
        let couter = (c.json_array)();
        let router = (r.json_array)();
        let cmid = (c.json_array)();
        let rmid = (r.json_array)();
        (c.json_array_append_new)(cmid, incref(cca));
        (r.json_array_append_new)(rmid, incref(rca));
        (c.json_array_append_new)(couter, cmid);
        (r.json_array_append_new)(router, rmid);
        (c.json_array_append_new)(couter, (c.json_string)(cs("ok").as_ptr()));
        (r.json_array_append_new)(router, (r.json_string)(cs("ok").as_ptr()));
        let cc = (c.json_deep_copy)(couter);
        let rc = (r.json_deep_copy)(router);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy([[cycle]])");
        assert!(cc.is_null(), "C: nested array cycle -> NULL");
        decref(c, cc);
        decref(r, rc);
        cmp(c, r, couter, router, "original array after failed deep_copy");
        (c.json_array_clear)(cmid);
        (r.json_array_clear)(rmid);
        decref(c, couter);
        decref(r, router);
        break_arr_cycle(c, cca, ccb);
        break_arr_cycle(r, rca, rcb);

        // A cycle buried under a deep valid prefix (5 levels of objects).
        let (cca, ccb) = obj_cycle(c);
        let (rca, rcb) = obj_cycle(r);
        let croot = (c.json_object)();
        let rroot = (r.json_object)();
        let mut ccur = croot;
        let mut rcur = rroot;
        for lvl in 0..5 {
            let cn = (c.json_object)();
            let rn = (r.json_object)();
            let k = cs(&format!("l{lvl}"));
            (c.json_object_set_new)(ccur, k.as_ptr(), cn);
            (r.json_object_set_new)(rcur, k.as_ptr(), rn);
            ccur = cn;
            rcur = rn;
        }
        (c.json_object_set_new)(ccur, cs("cyc").as_ptr(), incref(cca));
        (r.json_object_set_new)(rcur, cs("cyc").as_ptr(), incref(rca));
        let cc = (c.json_deep_copy)(croot);
        let rc = (r.json_deep_copy)(rroot);
        diff_eq!(cc.is_null(), rc.is_null(), "deep_copy(cycle 6 levels down)");
        assert!(cc.is_null(), "C: a deeply buried cycle still fails");
        decref(c, cc);
        decref(r, rc);
        (c.json_object_del)(ccur, cs("cyc").as_ptr());
        (r.json_object_del)(rcur, cs("cyc").as_ptr());
        decref(c, croot);
        decref(r, rroot);
        break_cycle(c, cca, ccb);
        break_cycle(r, rca, rcb);
    }
}

// ===========================================================================
// Rows 225-226 — json_sprintf / json_vsprintf (also in value.c)
// ===========================================================================

#[test]
fn row225_json_sprintf() {
    let (c, r) = both();
    unsafe {
        // fmt == "" -> length 0 -> the json_string("") early-out.
        let empty = cs("");
        let cj = (c.json_sprintf)(empty.as_ptr());
        let rj = (r.json_sprintf)(empty.as_ptr());
        assert!(!cj.is_null(), "C: empty format yields json_string(\"\")");
        assert_eq!((c.json_string_length)(cj), 0, "C: length 0");
        cmp_free(c, r, cj, rj, "json_sprintf(\"\")");

        // A literal with no conversions, and "%%" only.
        for lit in ["plain text", "%%", "100%%", "a%%b"] {
            let f = cs(lit);
            let cj = (c.json_sprintf)(f.as_ptr());
            let rj = (r.json_sprintf)(f.as_ptr());
            cmp_free(c, r, cj, rj, &format!("json_sprintf({lit:?})"));
        }

        // length > 0 ASCII: "%s-%d"
        let fmt = cs("%s-%d");
        for (s, n) in [("alpha", 0i32), ("beta", -17), ("", 2147483647), ("x", -2147483648)] {
            let sv = cs(s);
            let cj = (c.json_sprintf)(fmt.as_ptr(), sv.as_ptr(), n as c_int);
            let rj = (r.json_sprintf)(fmt.as_ptr(), sv.as_ptr(), n as c_int);
            assert!(!cj.is_null(), "C: json_sprintf(\"%s-%d\") succeeds");
            cmp_free(c, r, cj, rj, &format!("json_sprintf(\"%s-%d\", {s:?}, {n})"));
        }

        // Output > 1 KiB, exercising jsonp_malloc(length + 1).
        let fmt_s = cs("%s");
        for n in [1023usize, 1024, 1025, 5000] {
            let big = cs(&"Q".repeat(n));
            let cj = (c.json_sprintf)(fmt_s.as_ptr(), big.as_ptr());
            let rj = (r.json_sprintf)(fmt_s.as_ptr(), big.as_ptr());
            assert_eq!((c.json_string_length)(cj), n, "C: length is {n}");
            cmp_free(c, r, cj, rj, &format!("json_sprintf(\"%s\", {n} bytes)"));
        }

        // Output that is invalid UTF-8 -> the buffer is freed and NULL returned.
        for (name, bytes) in bad_utf8() {
            let buf = cs_bytes(&bytes);
            if bytes.is_empty() {
                continue;
            }
            let cj = (c.json_sprintf)(fmt_s.as_ptr(), buf.as_ptr());
            let rj = (r.json_sprintf)(fmt_s.as_ptr(), buf.as_ptr());
            diff_eq!(cj.is_null(), rj.is_null(), "json_sprintf(invalid UTF-8 {name})");
            assert!(cj.is_null(), "C: invalid UTF-8 output -> NULL ({name})");
            decref(c, cj);
            decref(r, rj);
        }

        // Valid multi-byte UTF-8 output -> correct json_string_length.
        for s in ["héllo", "日本語", "\u{1F600}", "mix é 日 \u{1F600} ascii"] {
            let sv = cs(s);
            let cj = (c.json_sprintf)(fmt_s.as_ptr(), sv.as_ptr());
            let rj = (r.json_sprintf)(fmt_s.as_ptr(), sv.as_ptr());
            assert_eq!(
                (c.json_string_length)(cj),
                s.len(),
                "C: length is the byte length of {s:?}"
            );
            cmp_free(c, r, cj, rj, &format!("json_sprintf(\"%s\", {s:?})"));
        }
    }
}

#[test]
fn row226_json_vsprintf_with_a_real_va_list() {
    let (c, r) = both();
    unsafe {
        let sh = vashim();
        let cfn = sym_addr("C", b"json_vsprintf");
        let rfn = sym_addr("Rust", b"json_vsprintf");

        // Mixed conversions and 5+ varargs: both vsnprintf passes must see the
        // same argument list (the sizing pass on `ap`, the fill on the va_copy).
        let fmt = cs("[%s|%d|%f|%%|%s|%d|%.3f]");
        let s1 = cs("first");
        let s2 = cs("second");
        let cj = (sh.vsprintf)(
            cfn,
            fmt.as_ptr(),
            s1.as_ptr(),
            7 as c_int,
            1.5f64,
            s2.as_ptr(),
            -42 as c_int,
            2.0f64 / 3.0f64,
        );
        let rj = (sh.vsprintf)(
            rfn,
            fmt.as_ptr(),
            s1.as_ptr(),
            7 as c_int,
            1.5f64,
            s2.as_ptr(),
            -42 as c_int,
            2.0f64 / 3.0f64,
        );
        assert!(!cj.is_null(), "C: json_vsprintf with a real va_list succeeds");
        cmp_free(c, r, cj, rj, "json_vsprintf(mixed conversions, 6 varargs)");

        // The length-0 early-out through the shim, which takes the `out:` path
        // with va_end(aq) before any allocation.
        let empty = cs("");
        let cj = (sh.vsprintf)(cfn, empty.as_ptr());
        let rj = (sh.vsprintf)(rfn, empty.as_ptr());
        cmp_free(c, r, cj, rj, "json_vsprintf(\"\")");

        // A long output through the shim (both passes must agree on the size).
        let long_fmt = cs("%s%s%s%s%s");
        let a = cs(&"a".repeat(400));
        let b = cs(&"b".repeat(400));
        let cj = (sh.vsprintf)(
            cfn,
            long_fmt.as_ptr(),
            a.as_ptr(),
            b.as_ptr(),
            a.as_ptr(),
            b.as_ptr(),
            a.as_ptr(),
        );
        let rj = (sh.vsprintf)(
            rfn,
            long_fmt.as_ptr(),
            a.as_ptr(),
            b.as_ptr(),
            a.as_ptr(),
            b.as_ptr(),
            a.as_ptr(),
        );
        assert_eq!((c.json_string_length)(cj), 2000, "C: 5 x 400 bytes");
        cmp_free(c, r, cj, rj, "json_vsprintf(5 x 400 bytes)");

        // Invalid UTF-8 through the shim -> NULL from the utf8 check.
        let bad = cs_bytes(&[0xFF, 0xFE]);
        let one_s = cs("%s");
        let cj = (sh.vsprintf)(cfn, one_s.as_ptr(), bad.as_ptr());
        let rj = (sh.vsprintf)(rfn, one_s.as_ptr(), bad.as_ptr());
        diff_eq!(cj.is_null(), rj.is_null(), "json_vsprintf(invalid UTF-8)");
        assert!(cj.is_null(), "C: invalid UTF-8 -> NULL");
        decref(c, cj);
        decref(r, rj);
    }
}

// ===========================================================================
// Randomised mutation sequences — the divergence is attributed to the exact
// operation that caused it, because the full canonical state is compared after
// EVERY step.
// ===========================================================================

#[test]
fn mutation_sequence_object() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0404_0001);
    unsafe {
        for trial in 0..25 {
            let cj = (c.json_object)();
            let rj = (r.json_object)();
            for step in 0..70 {
                // A small key space so replacements, failed deletes and rehash
                // boundaries all occur frequently.
                let key = rand_key(&mut rng);
                let kp = key.as_ptr() as *const c_char;
                let klen = key.len();
                let kz = cs_bytes(&key); // NUL-terminated for the strlen variants
                let recipe = rand_value(&mut rng, 2);
                let op = rng.below(14);
                let opname: String;
                let cret: c_int;
                let rret: c_int;
                match op {
                    0 | 1 => {
                        opname = format!("set_new({:?})", B(key.clone()));
                        cret = (c.json_object_set_new)(cj, kz.as_ptr(), build(c, &recipe));
                        rret = (r.json_object_set_new)(rj, kz.as_ptr(), build(r, &recipe));
                    }
                    2 | 3 => {
                        opname = format!("setn_new({:?}, {klen})", B(key.clone()));
                        cret = (c.json_object_setn_new)(cj, kp, klen, build(c, &recipe));
                        rret = (r.json_object_setn_new)(rj, kp, klen, build(r, &recipe));
                    }
                    4 => {
                        opname = format!("set_new_nocheck({:?})", B(key.clone()));
                        cret = (c.json_object_set_new_nocheck)(cj, kz.as_ptr(), build(c, &recipe));
                        rret = (r.json_object_set_new_nocheck)(rj, kz.as_ptr(), build(r, &recipe));
                    }
                    5 => {
                        opname = format!("setn_new_nocheck({:?}, {klen})", B(key.clone()));
                        cret = (c.json_object_setn_new_nocheck)(cj, kp, klen, build(c, &recipe));
                        rret = (r.json_object_setn_new_nocheck)(rj, kp, klen, build(r, &recipe));
                    }
                    6 => {
                        opname = format!("del({:?})", B(key.clone()));
                        cret = (c.json_object_del)(cj, kz.as_ptr());
                        rret = (r.json_object_del)(rj, kz.as_ptr());
                    }
                    7 => {
                        opname = format!("deln({:?}, {klen})", B(key.clone()));
                        cret = (c.json_object_deln)(cj, kp, klen);
                        rret = (r.json_object_deln)(rj, kp, klen);
                    }
                    8 => {
                        opname = "clear".to_string();
                        cret = (c.json_object_clear)(cj);
                        rret = (r.json_object_clear)(rj);
                    }
                    9 | 10 | 11 => {
                        // update / update_existing / update_missing with a fresh
                        // random object.
                        let other = V::Obj(
                            (0..rng.below(5))
                                .map(|_| (rand_key(&mut rng), rand_value(&mut rng, 2)))
                                .collect(),
                        );
                        let cb = build(c, &other);
                        let rb = build(r, &other);
                        let which = op - 9;
                        let cf = update_fns(c);
                        let rf = update_fns(r);
                        opname = format!("{}", cf[which].0);
                        cret = (cf[which].1)(cj, cb);
                        rret = (rf[which].1)(rj, rb);
                        decref(c, cb);
                        decref(r, rb);
                    }
                    12 => {
                        let other = V::Obj(
                            (0..rng.below(4))
                                .map(|_| {
                                    (
                                        rand_key(&mut rng),
                                        V::Obj(vec![(rand_key(&mut rng), rand_value(&mut rng, 1))]),
                                    )
                                })
                                .collect(),
                        );
                        let cb = build(c, &other);
                        let rb = build(r, &other);
                        opname = "update_recursive".to_string();
                        cret = (c.json_object_update_recursive)(cj, cb);
                        rret = (r.json_object_update_recursive)(rj, rb);
                        decref(c, cb);
                        decref(r, rb);
                    }
                    _ => {
                        // iter_set_new at a random iteration position.
                        let n = (c.json_object_size)(cj);
                        if n == 0 {
                            opname = "iter_set_new(empty)".to_string();
                            cret = (c.json_object_iter_set_new)(
                                cj,
                                std::ptr::null_mut(),
                                build(c, &recipe),
                            );
                            rret = (r.json_object_iter_set_new)(
                                rj,
                                std::ptr::null_mut(),
                                build(r, &recipe),
                            );
                        } else {
                            let pos = rng.below(n);
                            let mut cit = (c.json_object_iter)(cj);
                            let mut rit = (r.json_object_iter)(rj);
                            for _ in 0..pos {
                                cit = (c.json_object_iter_next)(cj, cit);
                                rit = (r.json_object_iter_next)(rj, rit);
                            }
                            opname = format!("iter_set_new(pos {pos} of {n})");
                            cret = (c.json_object_iter_set_new)(cj, cit, build(c, &recipe));
                            rret = (r.json_object_iter_set_new)(rj, rit, build(r, &recipe));
                        }
                    }
                }
                diff_eq!(cret, rret, "trial {trial} step {step}: {opname} return");
                // The complete canonical state after EVERY step: sorted dump,
                // insertion-order dump, size, hashtable order and the full
                // ordered key/value iteration.
                diff_eq!(
                    snap(c, cj),
                    snap(r, rj),
                    "trial {trial} step {step}: state after {opname}"
                );
            }
            cmp_free(c, r, cj, rj, &format!("trial {trial}: final object"));
        }
    }
}

#[test]
fn mutation_sequence_array() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0404_0002);
    unsafe {
        for trial in 0..25 {
            let cj = (c.json_array)();
            let rj = (r.json_array)();
            for step in 0..90 {
                let n = (c.json_array_size)(cj);
                // Indices reach 0, middle, last and last+1 (and beyond).
                let idx = match rng.below(6) {
                    0 => 0,
                    1 => n / 2,
                    2 => n.saturating_sub(1),
                    3 => n,
                    4 => n + 1,
                    _ => rng.below(n + 3),
                };
                let recipe = rand_value(&mut rng, 2);
                let opname: String;
                let cret: c_int;
                let rret: c_int;
                match rng.below(12) {
                    0..=3 => {
                        opname = "append_new".to_string();
                        cret = (c.json_array_append_new)(cj, build(c, &recipe));
                        rret = (r.json_array_append_new)(rj, build(r, &recipe));
                    }
                    4 | 5 => {
                        opname = format!("insert_new({idx})");
                        cret = (c.json_array_insert_new)(cj, idx, build(c, &recipe));
                        rret = (r.json_array_insert_new)(rj, idx, build(r, &recipe));
                    }
                    6 => {
                        opname = format!("set_new({idx})");
                        cret = (c.json_array_set_new)(cj, idx, build(c, &recipe));
                        rret = (r.json_array_set_new)(rj, idx, build(r, &recipe));
                    }
                    7 | 8 => {
                        opname = format!("remove({idx})");
                        cret = (c.json_array_remove)(cj, idx);
                        rret = (r.json_array_remove)(rj, idx);
                    }
                    9 => {
                        opname = "clear".to_string();
                        cret = (c.json_array_clear)(cj);
                        rret = (r.json_array_clear)(rj);
                    }
                    10 => {
                        let other = V::Arr(
                            (0..rng.below(8)).map(|_| rand_value(&mut rng, 2)).collect(),
                        );
                        let cb = build(c, &other);
                        let rb = build(r, &other);
                        opname = "extend".to_string();
                        cret = (c.json_array_extend)(cj, cb);
                        rret = (r.json_array_extend)(rj, rb);
                        decref(c, cb);
                        decref(r, rb);
                    }
                    _ => {
                        opname = "self-extend".to_string();
                        cret = (c.json_array_extend)(cj, cj);
                        rret = (r.json_array_extend)(rj, rj);
                    }
                }
                diff_eq!(cret, rret, "trial {trial} step {step}: {opname} return");
                diff_eq!(
                    arr_cap(cj),
                    arr_cap(rj),
                    "trial {trial} step {step}: capacity after {opname}"
                );
                diff_eq!(
                    snap(c, cj),
                    snap(r, rj),
                    "trial {trial} step {step}: state after {opname}"
                );
                // Keep the arrays from growing without bound.
                while (c.json_array_size)(cj) > 200 {
                    (c.json_array_remove)(cj, 0);
                    (r.json_array_remove)(rj, 0);
                }
            }
            cmp_free(c, r, cj, rj, &format!("trial {trial}: final array"));
        }
    }
}

#[test]
fn mutation_sequence_mixed_tree() {
    // A tree of objects and arrays mutated together, so the operations interact
    // across container types and the dumps of nested values act as the
    // fingerprint of the whole structure after every step.
    let (c, r) = both();
    let mut rng = Rng::new(0x0404_0003);
    unsafe {
        for trial in 0..15 {
            let croot = (c.json_object)();
            let rroot = (r.json_object)();
            (c.json_object_set_new)(croot, cs("arr").as_ptr(), (c.json_array)());
            (r.json_object_set_new)(rroot, cs("arr").as_ptr(), (r.json_array)());
            (c.json_object_set_new)(croot, cs("obj").as_ptr(), (c.json_object)());
            (r.json_object_set_new)(rroot, cs("obj").as_ptr(), (r.json_object)());
            let carr = (c.json_object_get)(croot, cs("arr").as_ptr());
            let rarr = (r.json_object_get)(rroot, cs("arr").as_ptr());
            let cobj = (c.json_object_get)(croot, cs("obj").as_ptr());
            let robj = (r.json_object_get)(rroot, cs("obj").as_ptr());

            for step in 0..60 {
                let recipe = rand_value(&mut rng, 3);
                let key = rand_key(&mut rng);
                let kz = cs_bytes(&key);
                let n = (c.json_array_size)(carr);
                let idx = rng.below(n + 2);
                let opname: String;
                let cret: c_int;
                let rret: c_int;
                match rng.below(8) {
                    0 | 1 => {
                        opname = format!("arr.append_new (step {step})");
                        cret = (c.json_array_append_new)(carr, build(c, &recipe));
                        rret = (r.json_array_append_new)(rarr, build(r, &recipe));
                    }
                    2 => {
                        opname = format!("arr.insert_new({idx})");
                        cret = (c.json_array_insert_new)(carr, idx, build(c, &recipe));
                        rret = (r.json_array_insert_new)(rarr, idx, build(r, &recipe));
                    }
                    3 => {
                        opname = format!("arr.remove({idx})");
                        cret = (c.json_array_remove)(carr, idx);
                        rret = (r.json_array_remove)(rarr, idx);
                    }
                    4 | 5 => {
                        opname = format!("obj.set_new({:?})", B(key.clone()));
                        cret = (c.json_object_set_new_nocheck)(cobj, kz.as_ptr(), build(c, &recipe));
                        rret = (r.json_object_set_new_nocheck)(robj, kz.as_ptr(), build(r, &recipe));
                    }
                    6 => {
                        opname = format!("obj.del({:?})", B(key.clone()));
                        cret = (c.json_object_del)(cobj, kz.as_ptr());
                        rret = (r.json_object_del)(robj, kz.as_ptr());
                    }
                    _ => {
                        // Cross-link: put the array inside the object (a DAG),
                        // then dump the whole tree — dump.c's own loop check has
                        // to accept it.
                        opname = "obj[link] = arr".to_string();
                        cret = (c.json_object_set_new)(cobj, cs("link").as_ptr(), incref(carr));
                        rret = (r.json_object_set_new)(robj, cs("link").as_ptr(), incref(rarr));
                    }
                }
                diff_eq!(cret, rret, "trial {trial} step {step}: {opname} return");
                diff_eq!(
                    snap(c, croot),
                    snap(r, rroot),
                    "trial {trial} step {step}: root state after {opname}"
                );
                diff_eq!(
                    snap(c, carr),
                    snap(r, rarr),
                    "trial {trial} step {step}: array state after {opname}"
                );
                diff_eq!(
                    snap(c, cobj),
                    snap(r, robj),
                    "trial {trial} step {step}: object state after {opname}"
                );
                // json_copy / json_deep_copy / json_equal of the live tree at
                // every step, which is where do_deep_copy and jsonp_loop_check
                // get exercised on real shapes.
                let cshallow = (c.json_copy)(croot);
                let rshallow = (r.json_copy)(rroot);
                diff_eq!(
                    snap(c, cshallow),
                    snap(r, rshallow),
                    "trial {trial} step {step}: json_copy after {opname}"
                );
                diff_eq!(
                    (c.json_equal)(croot, cshallow),
                    (r.json_equal)(rroot, rshallow),
                    "trial {trial} step {step}: equal(root, shallow copy)"
                );
                decref(c, cshallow);
                decref(r, rshallow);
                let cdeep = (c.json_deep_copy)(croot);
                let rdeep = (r.json_deep_copy)(rroot);
                diff_eq!(
                    cdeep.is_null(),
                    rdeep.is_null()
                    , "trial {trial} step {step}: deep_copy null-ness after {opname}"
                );
                if !cdeep.is_null() {
                    diff_eq!(
                        snap(c, cdeep),
                        snap(r, rdeep),
                        "trial {trial} step {step}: json_deep_copy after {opname}"
                    );
                    diff_eq!(
                        (c.json_equal)(croot, cdeep),
                        (r.json_equal)(rroot, rdeep),
                        "trial {trial} step {step}: equal(root, deep copy)"
                    );
                    diff_eq!(
                        count_shared(c, croot, cdeep),
                        count_shared(r, rroot, rdeep),
                        "trial {trial} step {step}: deep copy sharing"
                    );
                }
                decref(c, cdeep);
                decref(r, rdeep);
            }
            // The cross-link may have made the tree a DAG; that is fine for
            // refcounting because no cycle was ever created.
            decref(c, croot);
            decref(r, rroot);
        }
    }
}

// ===========================================================================
// Extra adversarial coverage: raw (invalid-UTF-8) payloads and the "value is
// the container itself" branches, which the dumps alone cannot fingerprint.
// ===========================================================================

/// Like `rand_value` but keys and string values may be arbitrary bytes, so the
/// canonical dumps become NULL and only the byte-level snapshot distinguishes
/// the trees.
fn rand_value_raw(rng: &mut Rng, depth: usize) -> V {
    let raw_bytes = |rng: &mut Rng| -> Vec<u8> {
        let n = rng.below(8);
        (0..n).map(|_| rng.next_u32() as u8).collect()
    };
    if depth == 0 || rng.below(3) == 0 {
        return match rng.below(7) {
            0 | 1 => V::Str(raw_bytes(rng)),
            2 => V::Int(rng.json_int()),
            3 => V::Real(rng.real()),
            4 => V::True,
            5 => V::False,
            _ => V::Null,
        };
    }
    if rng.bool() {
        let n = rng.below(5);
        V::Obj(
            (0..n)
                .map(|_| (raw_bytes(rng), rand_value_raw(rng, depth - 1)))
                .collect(),
        )
    } else {
        let n = rng.below(5);
        V::Arr((0..n).map(|_| rand_value_raw(rng, depth - 1)).collect())
    }
}

#[test]
fn raw_byte_trees_copy_equal_and_deep_copy() {
    let (c, r) = both();
    let mut rng = Rng::new(0x0404_0004);
    unsafe {
        for trial in 0..120 {
            let t = rand_value_raw(&mut rng, 4);
            let cj = build(c, &t);
            let rj = build(r, &t);
            cmp(c, r, cj, rj, &format!("trial {trial}: raw-byte tree"));

            // json_copy: shallow, children shared.
            let cc = (c.json_copy)(cj);
            let rc = (r.json_copy)(rj);
            cmp(c, r, cc, rc, &format!("trial {trial}: json_copy of a raw tree"));
            diff_eq!(
                (c.json_equal)(cj, cc),
                (r.json_equal)(rj, rc),
                "trial {trial}: equal(raw tree, shallow copy)"
            );
            decref(c, cc);
            decref(r, rc);

            // json_deep_copy: new nodes, byte-identical string payloads
            // (json_string_copy goes through json_stringn_nocheck).
            let cd = (c.json_deep_copy)(cj);
            let rd = (r.json_deep_copy)(rj);
            diff_eq!(cd.is_null(), rd.is_null(), "trial {trial}: deep_copy null-ness");
            cmp(c, r, cd, rd, &format!("trial {trial}: json_deep_copy of a raw tree"));
            diff_eq!(
                (c.json_equal)(cj, cd),
                (r.json_equal)(rj, rd),
                "trial {trial}: equal(raw tree, deep copy)"
            );
            assert_eq!(
                (c.json_equal)(cj, cd),
                1,
                "C: deep copy of a raw-byte tree is equal to the original"
            );
            diff_eq!(
                count_shared(c, cj, cd),
                count_shared(r, rj, rd),
                "trial {trial}: raw deep copy sharing"
            );
            decref(c, cd);
            decref(r, rd);

            // A second independently built tree: equality must agree.
            let cj2 = build(c, &t);
            let rj2 = build(r, &t);
            diff_eq!(
                (c.json_equal)(cj, cj2),
                (r.json_equal)(rj, rj2),
                "trial {trial}: equal(two identical raw trees)"
            );
            decref(c, cj2);
            decref(r, rj2);
            decref(c, cj);
            decref(r, rj);
        }
    }
}

#[test]
fn container_stored_inside_the_updating_source() {
    let (c, r) = both();
    unsafe {
        // `other` holds the very object being updated: json_object_setn_nocheck
        // hits the `json == value` guard, decrefs the (incref'd) value and
        // returns -1, which json_object_update propagates.
        let cobj = (c.json_object)();
        let robj = (r.json_object)();
        (c.json_object_set_new)(cobj, cs("keep").as_ptr(), (c.json_integer)(1));
        (r.json_object_set_new)(robj, cs("keep").as_ptr(), (r.json_integer)(1));
        let cother = (c.json_object)();
        let rother = (r.json_object)();
        (c.json_object_set_new)(cother, cs("self").as_ptr(), incref(cobj));
        (r.json_object_set_new)(rother, cs("self").as_ptr(), incref(robj));

        let before_c = (*cobj).refcount;
        let before_r = (*robj).refcount;
        diff_eq!(before_c, before_r, "refcount before self-valued update");
        diff_eq!(
            (c.json_object_update)(cobj, cother),
            (r.json_object_update)(robj, rother),
            "json_object_update with the target as a value of other"
        );
        diff_eq!((*cobj).refcount, (*robj).refcount, "refcount after update");
        assert_eq!(
            (*cobj).refcount, before_c,
            "C: the failed set decref'd exactly the incref it made"
        );
        cmp(c, r, cobj, robj, "target after self-valued update");
        // update_existing / update_missing / update_recursive on the same shape.
        diff_eq!(
            (c.json_object_update_existing)(cobj, cother),
            (r.json_object_update_existing)(robj, rother),
            "update_existing with the target as a value"
        );
        diff_eq!(
            (c.json_object_update_missing)(cobj, cother),
            (r.json_object_update_missing)(robj, rother),
            "update_missing with the target as a value"
        );
        cmp(c, r, cobj, robj, "target after existing/missing self-valued update");
        diff_eq!(
            (c.json_object_update_recursive)(cobj, cother),
            (r.json_object_update_recursive)(robj, rother),
            "update_recursive with the target as a value"
        );
        cmp(c, r, cobj, robj, "target after recursive self-valued update");
        diff_eq!((*cobj).refcount, (*robj).refcount, "refcount after all updates");
        (c.json_object_del)(cother, cs("self").as_ptr());
        (r.json_object_del)(rother, cs("self").as_ptr());
        decref(c, cother);
        decref(r, rother);
        decref(c, cobj);
        decref(r, robj);

        // update_recursive(o, o) on an acyclic object: the outer object is in
        // `parents`, and every nested descent registers a different pointer.
        for shape in [
            V::Obj(vec![(b"a".to_vec(), V::Int(1))]),
            V::Obj(vec![(b"a".to_vec(), V::Obj(vec![(b"b".to_vec(), V::Int(1))]))]),
            V::Obj(vec![
                (b"a".to_vec(), V::Obj(vec![(b"b".to_vec(), V::Obj(vec![]))])),
                (b"c".to_vec(), V::Arr(vec![V::Int(1)])),
            ]),
            V::Obj(vec![]),
        ] {
            let cj = build(c, &shape);
            let rj = build(r, &shape);
            diff_eq!(
                (c.json_object_update_recursive)(cj, cj),
                (r.json_object_update_recursive)(rj, rj),
                "json_object_update_recursive(o, o)"
            );
            cmp_free(c, r, cj, rj, "object after update_recursive(o, o)");
        }

        // json_object_iter_set_new does NOT reject `value == json` (there is no
        // such guard in the C), so it silently creates a self reference. Both
        // libraries must behave identically; break it afterwards.
        let cj = (c.json_object)();
        let rj = (r.json_object)();
        (c.json_object_set_new)(cj, cs("k").as_ptr(), (c.json_integer)(1));
        (r.json_object_set_new)(rj, cs("k").as_ptr(), (r.json_integer)(1));
        let cit = (c.json_object_iter)(cj);
        let rit = (r.json_object_iter)(rj);
        diff_eq!(
            (c.json_object_iter_set_new)(cj, cit, incref(cj)),
            (r.json_object_iter_set_new)(rj, rit, incref(rj)),
            "json_object_iter_set_new(json, iter, json)"
        );
        diff_eq!((*cj).refcount, (*rj).refcount, "refcount after self iter_set_new");
        // A dump must fail on the resulting cycle in both libraries.
        diff_eq!(
            dump(c, cj, CANON).is_none(),
            dump(r, rj, CANON).is_none(),
            "json_dumps of a self-referential object"
        );
        (c.json_object_clear)(cj);
        (r.json_object_clear)(rj);
        decref(c, cj);
        decref(r, rj);
    }
}

#[test]
fn jsonp_loop_check_with_oversized_key_buffer() {
    let (c, r) = both();
    unsafe {
        // key_size larger than LOOP_KEY_LEN must behave identically (snprintf
        // writes the same bytes; the extra room is simply unused).
        let mut cht = Ht::new(c);
        let mut rht = Ht::new(r);
        let mut cbuf = vec![0x7f as c_char; 64];
        let mut rbuf = vec![0x7f as c_char; 64];
        let mut cvals = Vec::new();
        let mut rvals = Vec::new();
        for i in 0..20 {
            cvals.push((c.json_object)());
            rvals.push((r.json_object)());
            let mut cl: size_t = 0;
            let mut rl: size_t = 0;
            diff_eq!(
                (c.jsonp_loop_check)(cht.p(), cvals[i], cbuf.as_mut_ptr(), 64, &mut cl),
                (r.jsonp_loop_check)(rht.p(), rvals[i], rbuf.as_mut_ptr(), 64, &mut rl),
                "jsonp_loop_check with a 64-byte buffer #{i}"
            );
            diff_eq!(cl, rl, "key_len with a 64-byte buffer #{i}");
            // The written key is exactly "%p" of the pointer, NUL-terminated.
            let ck: Vec<u8> = (0..cl).map(|j| cbuf[j] as u8).collect();
            let rk: Vec<u8> = (0..rl).map(|j| rbuf[j] as u8).collect();
            assert_eq!(ck, format!("{:p}", cvals[i]).into_bytes(), "C: %p bytes");
            assert_eq!(rk, format!("{:p}", rvals[i]).into_bytes(), "Rust: %p bytes");
            diff_eq!(cbuf[cl], rbuf[rl], "NUL terminator #{i}");
            assert_eq!(cbuf[cl], 0, "C: snprintf NUL-terminates");
            diff_eq!(cht.t.size, rht.t.size, "parents size #{i}");
        }
        drop(cht);
        drop(rht);
        for i in 0..cvals.len() {
            decref(c, cvals[i]);
            decref(r, rvals[i]);
        }
    }
}
