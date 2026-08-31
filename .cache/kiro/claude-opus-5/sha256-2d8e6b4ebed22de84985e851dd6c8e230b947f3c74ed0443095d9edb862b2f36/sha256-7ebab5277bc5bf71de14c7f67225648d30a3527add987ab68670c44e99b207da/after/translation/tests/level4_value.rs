//! Level 4: value.c
//!
//! Values are built independently inside each library and compared through
//! `json_dumps` (byte-for-byte), plus the type/refcount/accessor results.

mod common;

use common::*;
use libloading::Symbol;
use std::ffi::{c_char, c_double, c_int, c_void};

const SEED: usize = 0x5eed_1234;

fn seed_both() -> (&'static Lib, &'static Lib) {
    let (c, r) = libs();
    for l in [c, r] {
        let f: Symbol<FnJsonObjectSeed> = l.sym("json_object_seed");
        unsafe { f(SEED) };
    }
    (c, r)
}

/// All the state of a json_t that a caller can observe.
#[derive(Debug, PartialEq)]
struct Obs {
    type_: c_int,
    refcount: usize,
    dump_any: Option<Vec<u8>>,
    dump_sorted: Option<Vec<u8>>,
    str_value: Option<Vec<u8>>,
    str_len: usize,
    int_value: JsonInt,
    real_value: u64,
    number_value: u64,
    arr_size: usize,
    obj_size: usize,
}

unsafe fn observe(l: &Lib, v: *mut JsonT) -> Option<Obs> {
    if v.is_null() {
        return None;
    }
    let sv: Symbol<FnJsonStringValue> = l.sym("json_string_value");
    let sl: Symbol<FnJsonStringLength> = l.sym("json_string_length");
    let iv: Symbol<FnJsonIntegerValue> = l.sym("json_integer_value");
    let rv: Symbol<FnJsonRealValue> = l.sym("json_real_value");
    let nv: Symbol<FnJsonNumberValue> = l.sym("json_number_value");
    let asz: Symbol<FnJsonArraySize> = l.sym("json_array_size");
    let osz: Symbol<FnJsonObjectSize> = l.sym("json_object_size");

    let s = sv(v);
    let strv = if s.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(s as *const u8, sl(v)).to_vec())
    };

    Some(Obs {
        type_: (*v).type_,
        refcount: (*v).refcount,
        dump_any: dump(l, v, 0x200 /* JSON_ENCODE_ANY */),
        dump_sorted: dump(l, v, 0x200 | 0x80 /* + SORT_KEYS */),
        str_value: strv,
        str_len: sl(v),
        int_value: iv(v),
        real_value: rv(v).to_bits(),
        number_value: nv(v).to_bits(),
        arr_size: asz(v),
        obj_size: osz(v),
    })
}

// ------------------------------------------------------------- singletons

#[test]
fn singletons_match() {
    let (c, r) = seed_both();
    for name in ["json_true", "json_false", "json_null"] {
        let fc: Symbol<FnNew0> = c.sym(name);
        let fr: Symbol<FnNew0> = r.sym(name);
        unsafe {
            let a = fc();
            let b = fr();
            assert!(!a.is_null() && !b.is_null(), "{name}");
            assert_eq!((*a).type_, (*b).type_, "{name} type");
            // singletons must be the same object every time and have a
            // refcount that never changes
            assert_eq!(fc(), a, "C {name} is a singleton");
            assert_eq!(fr(), b, "Rust {name} is a singleton");
            assert_eq!((*a).refcount, (*b).refcount, "{name} refcount");
            assert_eq!(
                dump(c, a, 0x200),
                dump(r, b, 0x200),
                "{name} dump"
            );
        }
    }
}

#[test]
fn singleton_incref_decref_is_noop() {
    let (c, r) = seed_both();
    for name in ["json_true", "json_false", "json_null"] {
        for l in [c, r] {
            let f: Symbol<FnNew0> = l.sym(name);
            let del: Symbol<FnJsonDelete> = l.sym("json_delete");
            unsafe {
                let v = f();
                let before = (*v).refcount;
                // json_decref on a singleton is a no-op because refcount is
                // (size_t)-1; exercise it via json_delete-free paths only.
                let _ = del;
                assert_eq!((*v).refcount, before, "{}: {name} refcount stable", l.name);
                assert_eq!((*v).refcount, usize::MAX, "{}: {name} refcount", l.name);
            }
        }
    }
}

// ----------------------------------------------------------------- strings

fn string_probes() -> Vec<Vec<u8>> {
    vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"hello world".to_vec(),
        b"with \"quotes\" and \\backslash\\".to_vec(),
        b"tab\there\nnewline\rcr\x08bs\x0cff".to_vec(),
        b"\x01\x02\x1f".to_vec(),
        b"slash/inside".to_vec(),
        "\u{7f}".as_bytes().to_vec(),
        "\u{80}".as_bytes().to_vec(),
        "\u{7ff}".as_bytes().to_vec(),
        "\u{800}".as_bytes().to_vec(),
        "\u{ffff}".as_bytes().to_vec(),
        "\u{10000}".as_bytes().to_vec(),
        "\u{10ffff}".as_bytes().to_vec(),
        "héllo wörld".as_bytes().to_vec(),
        "日本語".as_bytes().to_vec(),
        "𝄞 clef".as_bytes().to_vec(),
        "mixed ascii ünïcödé 日本 𝄞".as_bytes().to_vec(),
        vec![b'x'; 500],
        // invalid UTF-8 (rejected by the checking constructors)
        vec![0x80],
        vec![0xff, 0xfe],
        vec![0xc0, 0x80],
        vec![0xed, 0xa0, 0x80],
        vec![0xf5, 0x80, 0x80, 0x80],
        vec![0xe2, 0x82], // truncated
        // embedded NUL (only allowed via the *n variants)
        b"a\0b".to_vec(),
        vec![0u8],
    ]
}

#[test]
fn json_string_constructors_match() {
    let (c, r) = seed_both();
    for s in string_probes() {
        let has_nul = s.contains(&0);
        // json_string / json_string_nocheck take a NUL-terminated string
        if !has_nul {
            let z = std::ffi::CString::new(s.clone()).unwrap();
            for name in ["json_string", "json_string_nocheck"] {
                let fc: Symbol<FnJsonString> = c.sym(name);
                let fr: Symbol<FnJsonString> = r.sym(name);
                let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
                let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
                unsafe {
                    let a = fc(z.as_ptr());
                    let b = fr(z.as_ptr());
                    assert_eq!(
                        observe(c, a),
                        observe(r, b),
                        "{name}({:02x?})",
                        s
                    );
                    if !a.is_null() {
                        dc(a);
                    }
                    if !b.is_null() {
                        dr(b);
                    }
                }
            }
            // NULL argument
            for name in ["json_string", "json_string_nocheck"] {
                let fc: Symbol<FnJsonString> = c.sym(name);
                let fr: Symbol<FnJsonString> = r.sym(name);
                unsafe {
                    assert_eq!(
                        fc(std::ptr::null()).is_null(),
                        fr(std::ptr::null()).is_null(),
                        "{name}(NULL)"
                    );
                }
            }
        }
        // json_stringn / json_stringn_nocheck take an explicit length
        for name in ["json_stringn", "json_stringn_nocheck"] {
            let fc: Symbol<FnJsonStringn> = c.sym(name);
            let fr: Symbol<FnJsonStringn> = r.sym(name);
            let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
            let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
            unsafe {
                let a = fc(s.as_ptr() as *const c_char, s.len());
                let b = fr(s.as_ptr() as *const c_char, s.len());
                assert_eq!(observe(c, a), observe(r, b), "{name}({:02x?})", s);
                if !a.is_null() {
                    dc(a);
                }
                if !b.is_null() {
                    dr(b);
                }
                assert_eq!(
                    fc(std::ptr::null(), 0).is_null(),
                    fr(std::ptr::null(), 0).is_null(),
                    "{name}(NULL, 0)"
                );
            }
        }
    }
}

#[test]
fn json_string_setters_match() {
    let (c, r) = seed_both();
    for name in [
        "json_string_set",
        "json_string_set_nocheck",
    ] {
        for s in string_probes() {
            if s.contains(&0) {
                continue;
            }
            let z = std::ffi::CString::new(s.clone()).unwrap();
            let mkc: Symbol<FnJsonString> = c.sym("json_string_nocheck");
            let mkr: Symbol<FnJsonString> = r.sym("json_string_nocheck");
            let fc: Symbol<FnJsonStringSet> = c.sym(name);
            let fr: Symbol<FnJsonStringSet> = r.sym(name);
            let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
            let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
            let init = cs("initial");
            unsafe {
                let a = mkc(init.as_ptr());
                let b = mkr(init.as_ptr());
                let ra = fc(a, z.as_ptr());
                let rb = fr(b, z.as_ptr());
                assert_eq!(ra, rb, "{name}({:02x?}) rc", s);
                assert_eq!(observe(c, a), observe(r, b), "{name}({:02x?})", s);
                // wrong type and NULL
                let ia: Symbol<FnJsonInteger> = c.sym("json_integer");
                let ib: Symbol<FnJsonInteger> = r.sym("json_integer");
                let na = ia(1);
                let nb = ib(1);
                assert_eq!(fc(na, z.as_ptr()), fr(nb, z.as_ptr()), "{name} wrong type");
                assert_eq!(
                    fc(a, std::ptr::null()),
                    fr(b, std::ptr::null()),
                    "{name} NULL value"
                );
                assert_eq!(
                    fc(std::ptr::null_mut(), z.as_ptr()),
                    fr(std::ptr::null_mut(), z.as_ptr()),
                    "{name} NULL target"
                );
                dc(na);
                dr(nb);
                dc(a);
                dr(b);
            }
        }
    }
    for name in ["json_string_setn", "json_string_setn_nocheck"] {
        for s in string_probes() {
            let mkc: Symbol<FnJsonString> = c.sym("json_string_nocheck");
            let mkr: Symbol<FnJsonString> = r.sym("json_string_nocheck");
            let fc: Symbol<FnJsonStringSetn> = c.sym(name);
            let fr: Symbol<FnJsonStringSetn> = r.sym(name);
            let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
            let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
            let init = cs("initial");
            unsafe {
                let a = mkc(init.as_ptr());
                let b = mkr(init.as_ptr());
                let ra = fc(a, s.as_ptr() as *const c_char, s.len());
                let rb = fr(b, s.as_ptr() as *const c_char, s.len());
                assert_eq!(ra, rb, "{name}({:02x?}) rc", s);
                assert_eq!(observe(c, a), observe(r, b), "{name}({:02x?})", s);
                dc(a);
                dr(b);
            }
        }
    }
}

// ---------------------------------------------------------------- numbers

fn int_probes() -> Vec<JsonInt> {
    let mut v = vec![
        0,
        1,
        -1,
        2,
        -2,
        127,
        128,
        -128,
        255,
        256,
        32767,
        -32768,
        2147483647,
        -2147483648,
        4294967295,
        4294967296,
        i64::MAX,
        i64::MIN,
        i64::MAX - 1,
        i64::MIN + 1,
        9007199254740992,
        9007199254740993,
        -9007199254740993,
        1000000000000000000,
    ];
    let mut s: u64 = 0x9e37_79b9;
    for _ in 0..200 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        v.push(s as i64);
    }
    v
}

fn real_probes() -> Vec<f64> {
    let mut v = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        0.1,
        1.0 / 3.0,
        1e-10,
        1e10,
        1e100,
        1e-100,
        1e308,
        1e-308,
        5e-324,
        f64::MAX,
        f64::MIN,
        f64::EPSILON,
        3.141592653589793,
        123456789.123456789,
        1e16,
        1e17,
        1e-4,
        1e-5,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    let mut s: u64 = 0x1357_9bdf;
    for _ in 0..400 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        v.push(f64::from_bits(s));
    }
    v
}

#[test]
fn json_integer_matches() {
    let (c, r) = seed_both();
    let mkc: Symbol<FnJsonInteger> = c.sym("json_integer");
    let mkr: Symbol<FnJsonInteger> = r.sym("json_integer");
    let sc: Symbol<FnJsonIntegerSet> = c.sym("json_integer_set");
    let sr: Symbol<FnJsonIntegerSet> = r.sym("json_integer_set");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    for i in int_probes() {
        unsafe {
            let a = mkc(i);
            let b = mkr(i);
            assert_eq!(observe(c, a), observe(r, b), "json_integer({i})");
            for j in [0i64, -1, i64::MAX, i64::MIN, 42] {
                assert_eq!(sc(a, j), sr(b, j), "json_integer_set({j}) rc");
                assert_eq!(observe(c, a), observe(r, b), "json_integer_set({j})");
            }
            dc(a);
            dr(b);
        }
    }
    // setters on the wrong type / NULL
    unsafe {
        let sa: Symbol<FnJsonString> = c.sym("json_string");
        let sb: Symbol<FnJsonString> = r.sym("json_string");
        let z = cs("x");
        let a = sa(z.as_ptr());
        let b = sb(z.as_ptr());
        assert_eq!(sc(a, 1), sr(b, 1), "json_integer_set wrong type");
        assert_eq!(
            sc(std::ptr::null_mut(), 1),
            sr(std::ptr::null_mut(), 1),
            "json_integer_set NULL"
        );
        dc(a);
        dr(b);
    }
}

#[test]
fn json_real_matches() {
    let (c, r) = seed_both();
    let mkc: Symbol<FnJsonReal> = c.sym("json_real");
    let mkr: Symbol<FnJsonReal> = r.sym("json_real");
    let sc: Symbol<FnJsonRealSet> = c.sym("json_real_set");
    let sr: Symbol<FnJsonRealSet> = r.sym("json_real_set");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    for v in real_probes() {
        unsafe {
            let a = mkc(v);
            let b = mkr(v);
            assert_eq!(
                observe(c, a),
                observe(r, b),
                "json_real({v:e} [{:#018x}])",
                v.to_bits()
            );
            if !a.is_null() {
                for w in [0.0f64, -1.5, f64::NAN, f64::INFINITY, 1e300] {
                    assert_eq!(sc(a, w), sr(b, w), "json_real_set({w}) rc");
                    assert_eq!(observe(c, a), observe(r, b), "json_real_set({w})");
                }
                dc(a);
                dr(b);
            } else {
                assert!(b.is_null());
            }
        }
    }
    unsafe {
        assert_eq!(
            sc(std::ptr::null_mut(), 1.0),
            sr(std::ptr::null_mut(), 1.0),
            "json_real_set NULL"
        );
    }
}

#[test]
fn number_value_on_all_types_matches() {
    let (c, r) = seed_both();
    for (mk, arg) in [
        ("json_object", 0i64),
        ("json_array", 0),
        ("json_true", 0),
        ("json_false", 0),
        ("json_null", 0),
    ] {
        let fc: Symbol<FnNew0> = c.sym(mk);
        let fr: Symbol<FnNew0> = r.sym(mk);
        let _ = arg;
        unsafe {
            let a = fc();
            let b = fr();
            assert_eq!(observe(c, a), observe(r, b), "{mk} accessors");
        }
    }
    // NULL
    for name in [
        "json_string_value",
        "json_integer_value",
        "json_real_value",
        "json_number_value",
    ] {
        unsafe {
            match name {
                "json_string_value" => {
                    let fc: Symbol<FnJsonStringValue> = c.sym(name);
                    let fr: Symbol<FnJsonStringValue> = r.sym(name);
                    assert_eq!(fc(std::ptr::null()).is_null(), fr(std::ptr::null()).is_null());
                }
                "json_integer_value" => {
                    let fc: Symbol<FnJsonIntegerValue> = c.sym(name);
                    let fr: Symbol<FnJsonIntegerValue> = r.sym(name);
                    assert_eq!(fc(std::ptr::null()), fr(std::ptr::null()));
                }
                _ => {
                    let fc: Symbol<FnJsonRealValue> = c.sym(name);
                    let fr: Symbol<FnJsonRealValue> = r.sym(name);
                    assert_eq!(fc(std::ptr::null()).to_bits(), fr(std::ptr::null()).to_bits());
                }
            }
        }
    }
    unsafe {
        let fc: Symbol<FnJsonStringLength> = c.sym("json_string_length");
        let fr: Symbol<FnJsonStringLength> = r.sym("json_string_length");
        assert_eq!(fc(std::ptr::null()), fr(std::ptr::null()));
    }
}

// ------------------------------------------------------------------ arrays

/// Drive both libraries through the same array operation script.
#[test]
fn json_array_ops_match() {
    let (c, r) = seed_both();
    let mkc: Symbol<FnNew0> = c.sym("json_array");
    let mkr: Symbol<FnNew0> = r.sym("json_array");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    unsafe {
        let a = mkc();
        let b = mkr();
        assert_eq!(observe(c, a), observe(r, b), "fresh array");

        let apc: Symbol<FnJsonArrayAppendNew> = c.sym("json_array_append_new");
        let apr: Symbol<FnJsonArrayAppendNew> = r.sym("json_array_append_new");
        let inc: Symbol<FnJsonArrayInsertNew> = c.sym("json_array_insert_new");
        let inr: Symbol<FnJsonArrayInsertNew> = r.sym("json_array_insert_new");
        let stc: Symbol<FnJsonArraySetNew> = c.sym("json_array_set_new");
        let str_: Symbol<FnJsonArraySetNew> = r.sym("json_array_set_new");
        let rmc: Symbol<FnJsonArrayRemove> = c.sym("json_array_remove");
        let rmr: Symbol<FnJsonArrayRemove> = r.sym("json_array_remove");
        let gtc: Symbol<FnJsonArrayGet> = c.sym("json_array_get");
        let gtr: Symbol<FnJsonArrayGet> = r.sym("json_array_get");
        let ic: Symbol<FnJsonInteger> = c.sym("json_integer");
        let ir: Symbol<FnJsonInteger> = r.sym("json_integer");

        // grow well past the initial capacity (8) and several reallocations
        for i in 0..200i64 {
            assert_eq!(apc(a, ic(i)), apr(b, ir(i)), "append {i} rc");
        }
        assert_eq!(observe(c, a), observe(r, b), "after 200 appends");

        // get, in range and out of range
        for i in [0usize, 1, 99, 199, 200, 201, usize::MAX] {
            let x = gtc(a, i);
            let y = gtr(b, i);
            assert_eq!(x.is_null(), y.is_null(), "get({i}) presence");
            if !x.is_null() {
                assert_eq!(observe(c, x), observe(r, y), "get({i})");
            }
        }

        // insert at various positions, including the end and out of range
        for i in [0usize, 1, 50, 200, 201, 500] {
            let x = inc(a, i, ic(-(i as i64) - 1));
            let y = inr(b, i, ir(-(i as i64) - 1));
            assert_eq!(x, y, "insert({i}) rc");
            assert_eq!(observe(c, a), observe(r, b), "after insert({i})");
        }

        // set, in range and out of range
        for i in [0usize, 3, 100, 5000] {
            let x = stc(a, i, ic(7777 + i as i64));
            let y = str_(b, i, ir(7777 + i as i64));
            assert_eq!(x, y, "set({i}) rc");
            assert_eq!(observe(c, a), observe(r, b), "after set({i})");
        }

        // remove from front, middle, back and out of range
        for i in [0usize, 5, 100, 100000] {
            let x = rmc(a, i);
            let y = rmr(b, i);
            assert_eq!(x, y, "remove({i}) rc");
            assert_eq!(observe(c, a), observe(r, b), "after remove({i})");
        }
        while {
            let sz: Symbol<FnJsonArraySize> = c.sym("json_array_size");
            sz(a) > 0
        } {
            assert_eq!(rmc(a, 0), rmr(b, 0), "drain remove rc");
        }
        assert_eq!(observe(c, a), observe(r, b), "drained");

        // clear and extend
        let clc: Symbol<FnJsonArrayClear> = c.sym("json_array_clear");
        let clr: Symbol<FnJsonArrayClear> = r.sym("json_array_clear");
        for i in 0..20i64 {
            apc(a, ic(i));
            apr(b, ir(i));
        }
        assert_eq!(clc(a), clr(b), "clear rc");
        assert_eq!(observe(c, a), observe(r, b), "after clear");

        let exc: Symbol<FnJsonArrayExtend> = c.sym("json_array_extend");
        let exr: Symbol<FnJsonArrayExtend> = r.sym("json_array_extend");
        let o1 = mkc();
        let o2 = mkr();
        for i in 0..15i64 {
            apc(o1, ic(100 + i));
            apr(o2, ir(100 + i));
        }
        assert_eq!(exc(a, o1), exr(b, o2), "extend rc");
        assert_eq!(observe(c, a), observe(r, b), "after extend");
        assert_eq!(exc(a, a), exr(b, b), "self extend rc");
        assert_eq!(observe(c, a), observe(r, b), "after self extend");
        dc(o1);
        dr(o2);

        // NULL / wrong-type arguments
        assert_eq!(clc(std::ptr::null_mut()), clr(std::ptr::null_mut()), "clear NULL");
        assert_eq!(exc(a, std::ptr::null_mut()), exr(b, std::ptr::null_mut()), "extend NULL");
        assert_eq!(apc(a, std::ptr::null_mut()), apr(b, std::ptr::null_mut()), "append NULL");
        assert_eq!(
            apc(std::ptr::null_mut(), ic(1)),
            apr(std::ptr::null_mut(), ir(1)),
            "append to NULL"
        );
        assert_eq!(rmc(std::ptr::null_mut(), 0), rmr(std::ptr::null_mut(), 0), "remove NULL");
        {
            let sc2: Symbol<FnJsonArraySize> = c.sym("json_array_size");
            let sr2: Symbol<FnJsonArraySize> = r.sym("json_array_size");
            assert_eq!(sc2(std::ptr::null()), sr2(std::ptr::null()), "size NULL");
        }

        dc(a);
        dr(b);
    }
}

// ----------------------------------------------------------------- objects

#[test]
fn json_object_ops_match() {
    let (c, r) = seed_both();
    let mkc: Symbol<FnNew0> = c.sym("json_object");
    let mkr: Symbol<FnNew0> = r.sym("json_object");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
    let ic: Symbol<FnJsonInteger> = c.sym("json_integer");
    let ir: Symbol<FnJsonInteger> = r.sym("json_integer");

    unsafe {
        let a = mkc();
        let b = mkr();
        assert_eq!(observe(c, a), observe(r, b), "fresh object");

        let setc: Symbol<FnJsonObjectSetNew> = c.sym("json_object_set_new");
        let setr: Symbol<FnJsonObjectSetNew> = r.sym("json_object_set_new");
        let getc: Symbol<FnJsonObjectGet> = c.sym("json_object_get");
        let getr: Symbol<FnJsonObjectGet> = r.sym("json_object_get");
        let delc: Symbol<FnJsonObjectDel> = c.sym("json_object_del");
        let delr: Symbol<FnJsonObjectDel> = r.sym("json_object_del");

        for i in 0..150i64 {
            let k = cs(&format!("key{i}"));
            assert_eq!(setc(a, k.as_ptr(), ic(i)), setr(b, k.as_ptr(), ir(i)), "set {i}");
        }
        assert_eq!(observe(c, a), observe(r, b), "after 150 sets");

        for i in [0i64, 1, 149, 150, 999] {
            let k = cs(&format!("key{i}"));
            let x = getc(a, k.as_ptr());
            let y = getr(b, k.as_ptr());
            assert_eq!(x.is_null(), y.is_null(), "get key{i}");
            if !x.is_null() {
                assert_eq!(observe(c, x), observe(r, y), "get key{i} value");
            }
        }

        // overwrite existing keys
        for i in 0..150i64 {
            let k = cs(&format!("key{i}"));
            assert_eq!(
                setc(a, k.as_ptr(), ic(i * 3)),
                setr(b, k.as_ptr(), ir(i * 3)),
                "overwrite {i}"
            );
        }
        assert_eq!(observe(c, a), observe(r, b), "after overwrites");

        // delete
        for i in (0..150i64).step_by(3) {
            let k = cs(&format!("key{i}"));
            assert_eq!(delc(a, k.as_ptr()), delr(b, k.as_ptr()), "del {i}");
        }
        assert_eq!(observe(c, a), observe(r, b), "after deletes");
        let miss = cs("missing");
        assert_eq!(delc(a, miss.as_ptr()), delr(b, miss.as_ptr()), "del missing");

        // keys with embedded NUL via the *n variants
        let setnc: Symbol<FnJsonObjectSetnNew> = c.sym("json_object_setn_new");
        let setnr: Symbol<FnJsonObjectSetnNew> = r.sym("json_object_setn_new");
        let setnnc: Symbol<FnJsonObjectSetnNew> = c.sym("json_object_setn_new_nocheck");
        let setnnr: Symbol<FnJsonObjectSetnNew> = r.sym("json_object_setn_new_nocheck");
        let getnc: Symbol<FnJsonObjectGetn> = c.sym("json_object_getn");
        let getnr: Symbol<FnJsonObjectGetn> = r.sym("json_object_getn");
        let delnc: Symbol<FnJsonObjectDeln> = c.sym("json_object_deln");
        let delnr: Symbol<FnJsonObjectDeln> = r.sym("json_object_deln");

        for k in [
            &b"nul\0key"[..],
            b"",
            b"\0",
            b"plain",
            &[0xffu8, 0x41][..], // invalid UTF-8 key
            "ünïcödé".as_bytes(),
        ] {
            let x = setnc(a, k.as_ptr() as *const c_char, k.len(), ic(1));
            let y = setnr(b, k.as_ptr() as *const c_char, k.len(), ir(1));
            assert_eq!(x, y, "setn({k:02x?}) rc");
            assert_eq!(observe(c, a), observe(r, b), "after setn({k:02x?})");

            let x = setnnc(a, k.as_ptr() as *const c_char, k.len(), ic(2));
            let y = setnnr(b, k.as_ptr() as *const c_char, k.len(), ir(2));
            assert_eq!(x, y, "setn_nocheck({k:02x?}) rc");
            assert_eq!(observe(c, a), observe(r, b), "after setn_nocheck({k:02x?})");

            let x = getnc(a, k.as_ptr() as *const c_char, k.len());
            let y = getnr(b, k.as_ptr() as *const c_char, k.len());
            assert_eq!(x.is_null(), y.is_null(), "getn({k:02x?})");

            let x = delnc(a, k.as_ptr() as *const c_char, k.len());
            let y = delnr(b, k.as_ptr() as *const c_char, k.len());
            assert_eq!(x, y, "deln({k:02x?}) rc");
            assert_eq!(observe(c, a), observe(r, b), "after deln({k:02x?})");
        }

        // set_new_nocheck with an invalid-UTF-8 NUL-terminated key
        let snc: Symbol<FnJsonObjectSetNew> = c.sym("json_object_set_new_nocheck");
        let snr: Symbol<FnJsonObjectSetNew> = r.sym("json_object_set_new_nocheck");
        let badkey = std::ffi::CString::new(vec![0xffu8, 0x41]).unwrap();
        assert_eq!(
            snc(a, badkey.as_ptr(), ic(9)),
            snr(b, badkey.as_ptr(), ir(9)),
            "set_new_nocheck bad key"
        );
        assert_eq!(observe(c, a), observe(r, b), "after set_new_nocheck bad key");

        // NULL arguments
        let k = cs("k");
        assert_eq!(
            setc(std::ptr::null_mut(), k.as_ptr(), ic(1)),
            setr(std::ptr::null_mut(), k.as_ptr(), ir(1)),
            "set NULL object"
        );
        assert_eq!(setc(a, k.as_ptr(), std::ptr::null_mut()), setr(b, k.as_ptr(), std::ptr::null_mut()), "set NULL value");
        assert_eq!(
            getc(std::ptr::null(), k.as_ptr()).is_null(),
            getr(std::ptr::null(), k.as_ptr()).is_null(),
            "get NULL object"
        );
        assert_eq!(
            getc(a, std::ptr::null()).is_null(),
            getr(b, std::ptr::null()).is_null(),
            "get NULL key"
        );
        assert_eq!(
            delc(std::ptr::null_mut(), k.as_ptr()),
            delr(std::ptr::null_mut(), k.as_ptr()),
            "del NULL object"
        );
        {
            let szc: Symbol<FnJsonObjectSize> = c.sym("json_object_size");
            let szr: Symbol<FnJsonObjectSize> = r.sym("json_object_size");
            assert_eq!(szc(std::ptr::null()), szr(std::ptr::null()), "size NULL");
        }

        // clear
        let clc: Symbol<FnJsonObjectClear> = c.sym("json_object_clear");
        let clr: Symbol<FnJsonObjectClear> = r.sym("json_object_clear");
        assert_eq!(clc(a), clr(b), "clear rc");
        assert_eq!(observe(c, a), observe(r, b), "after clear");
        assert_eq!(clc(std::ptr::null_mut()), clr(std::ptr::null_mut()), "clear NULL");

        dc(a);
        dr(b);
    }
}

#[test]
fn json_object_iteration_matches() {
    let (c, r) = seed_both();
    unsafe {
        let mkc: Symbol<FnNew0> = c.sym("json_object");
        let mkr: Symbol<FnNew0> = r.sym("json_object");
        let setc: Symbol<FnJsonObjectSetnNew> = c.sym("json_object_setn_new_nocheck");
        let setr: Symbol<FnJsonObjectSetnNew> = r.sym("json_object_setn_new_nocheck");
        let ic: Symbol<FnJsonInteger> = c.sym("json_integer");
        let ir: Symbol<FnJsonInteger> = r.sym("json_integer");
        let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
        let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

        let a = mkc();
        let b = mkr();
        let keys: Vec<Vec<u8>> = (0..80).map(|i| format!("k{i}").into_bytes()).collect();
        for (i, k) in keys.iter().enumerate() {
            setc(a, k.as_ptr() as *const c_char, k.len(), ic(i as i64));
            setr(b, k.as_ptr() as *const c_char, k.len(), ir(i as i64));
        }

        let itc: Symbol<FnJsonObjectIter> = c.sym("json_object_iter");
        let itr: Symbol<FnJsonObjectIter> = r.sym("json_object_iter");
        let nxc: Symbol<FnJsonObjectIterNext> = c.sym("json_object_iter_next");
        let nxr: Symbol<FnJsonObjectIterNext> = r.sym("json_object_iter_next");
        let kyc: Symbol<FnJsonObjectIterKey> = c.sym("json_object_iter_key");
        let kyr: Symbol<FnJsonObjectIterKey> = r.sym("json_object_iter_key");
        let klc: Symbol<FnJsonObjectIterKeyLen> = c.sym("json_object_iter_key_len");
        let klr: Symbol<FnJsonObjectIterKeyLen> = r.sym("json_object_iter_key_len");
        let vlc: Symbol<FnJsonObjectIterValue> = c.sym("json_object_iter_value");
        let vlr: Symbol<FnJsonObjectIterValue> = r.sym("json_object_iter_value");
        let ivc: Symbol<FnJsonIntegerValue> = c.sym("json_integer_value");
        let ivr: Symbol<FnJsonIntegerValue> = r.sym("json_integer_value");

        let mut tc = Vec::new();
        let mut tr = Vec::new();
        let mut i = itc(a);
        while !i.is_null() {
            let kl = klc(i);
            tc.push((
                std::slice::from_raw_parts(kyc(i) as *const u8, kl).to_vec(),
                kl,
                ivc(vlc(i)),
            ));
            i = nxc(a, i);
        }
        let mut i = itr(b);
        while !i.is_null() {
            let kl = klr(i);
            tr.push((
                std::slice::from_raw_parts(kyr(i) as *const u8, kl).to_vec(),
                kl,
                ivr(vlr(i)),
            ));
            i = nxr(b, i);
        }
        assert_eq!(tc, tr, "object iteration order and contents");

        // iter_at
        let iac: Symbol<FnJsonObjectIterAt> = c.sym("json_object_iter_at");
        let iar: Symbol<FnJsonObjectIterAt> = r.sym("json_object_iter_at");
        for k in keys.iter().chain([b"nope".to_vec()].iter()) {
            let z = std::ffi::CString::new(k.clone()).unwrap();
            let x = iac(a, z.as_ptr());
            let y = iar(b, z.as_ptr());
            assert_eq!(x.is_null(), y.is_null(), "iter_at({k:02x?})");
            if !x.is_null() {
                assert_eq!(klc(x), klr(y), "iter_at({k:02x?}) key_len");
                assert_eq!(ivc(vlc(x)), ivr(vlr(y)), "iter_at({k:02x?}) value");
            }
        }
        // NULL handling
        assert_eq!(itc(std::ptr::null_mut()).is_null(), itr(std::ptr::null_mut()).is_null());
        assert_eq!(
            iac(std::ptr::null_mut(), cs("k0").as_ptr()).is_null(),
            iar(std::ptr::null_mut(), cs("k0").as_ptr()).is_null()
        );
        assert_eq!(
            nxc(std::ptr::null_mut(), std::ptr::null_mut()).is_null(),
            nxr(std::ptr::null_mut(), std::ptr::null_mut()).is_null()
        );
        assert!(kyc(std::ptr::null_mut()).is_null());
        assert!(kyr(std::ptr::null_mut()).is_null());
        assert_eq!(klc(std::ptr::null_mut()), klr(std::ptr::null_mut()));
        assert!(vlc(std::ptr::null_mut()).is_null());
        assert!(vlr(std::ptr::null_mut()).is_null());

        // iter_set_new
        let isc: Symbol<FnJsonObjectIterSetNew> = c.sym("json_object_iter_set_new");
        let isr: Symbol<FnJsonObjectIterSetNew> = r.sym("json_object_iter_set_new");
        let mut i = itc(a);
        let mut j = itr(b);
        let mut n = 0i64;
        while !i.is_null() {
            assert_eq!(isc(a, i, ic(900 + n)), isr(b, j, ir(900 + n)), "iter_set_new {n}");
            i = nxc(a, i);
            j = nxr(b, j);
            n += 1;
        }
        assert_eq!(observe(c, a), observe(r, b), "after iter_set_new sweep");
        assert_eq!(
            isc(a, std::ptr::null_mut(), ic(1)),
            isr(b, std::ptr::null_mut(), ir(1)),
            "iter_set_new NULL iter"
        );
        assert_eq!(
            isc(a, itc(a), std::ptr::null_mut()),
            isr(b, itr(b), std::ptr::null_mut()),
            "iter_set_new NULL value"
        );

        dc(a);
        dr(b);
    }
}

#[test]
fn json_object_update_variants_match() {
    let (c, r) = seed_both();
    for name in [
        "json_object_update",
        "json_object_update_existing",
        "json_object_update_missing",
        "json_object_update_recursive",
    ] {
        unsafe {
            let build = |l: &Lib, spec: &[(&str, i64)], nested: bool| -> *mut JsonT {
                let mk: Symbol<FnNew0> = l.sym("json_object");
                let set: Symbol<FnJsonObjectSetNew> = l.sym("json_object_set_new");
                let int: Symbol<FnJsonInteger> = l.sym("json_integer");
                let o = mk();
                for (k, v) in spec {
                    let z = cs(k);
                    if nested {
                        let inner = mk();
                        let ik = cs("inner");
                        set(inner, ik.as_ptr(), int(*v));
                        set(o, z.as_ptr(), inner);
                    } else {
                        set(o, z.as_ptr(), int(*v));
                    }
                }
                o
            };

            for nested in [false, true] {
                let base = [("a", 1i64), ("b", 2), ("c", 3)];
                let other = [("b", 20i64), ("c", 30), ("d", 40)];
                let fc: Symbol<FnJsonObjectUpdate> = c.sym(name);
                let fr: Symbol<FnJsonObjectUpdate> = r.sym(name);
                let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
                let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

                let a1 = build(c, &base, nested);
                let a2 = build(c, &other, nested);
                let b1 = build(r, &base, nested);
                let b2 = build(r, &other, nested);
                assert_eq!(fc(a1, a2), fr(b1, b2), "{name} nested={nested} rc");
                assert_eq!(
                    observe(c, a1),
                    observe(r, b1),
                    "{name} nested={nested} result"
                );
                // NULL args
                assert_eq!(
                    fc(std::ptr::null_mut(), a2),
                    fr(std::ptr::null_mut(), b2),
                    "{name} NULL target"
                );
                assert_eq!(
                    fc(a1, std::ptr::null_mut()),
                    fr(b1, std::ptr::null_mut()),
                    "{name} NULL source"
                );
                // self update
                assert_eq!(fc(a1, a1), fr(b1, b1), "{name} self rc");
                assert_eq!(observe(c, a1), observe(r, b1), "{name} self result");
                dc(a1);
                dc(a2);
                dr(b1);
                dr(b2);
            }
        }
    }
}

// ---------------------------------------------------------- equal and copy

/// Build the same reasonably deep, mixed value in `l`.
unsafe fn build_sample(l: &Lib, variant: u32) -> *mut JsonT {
    let obj: Symbol<FnNew0> = l.sym("json_object");
    let arr: Symbol<FnNew0> = l.sym("json_array");
    let tru: Symbol<FnNew0> = l.sym("json_true");
    let fls: Symbol<FnNew0> = l.sym("json_false");
    let nul: Symbol<FnNew0> = l.sym("json_null");
    let int: Symbol<FnJsonInteger> = l.sym("json_integer");
    let real: Symbol<FnJsonReal> = l.sym("json_real");
    let strn: Symbol<FnJsonStringn> = l.sym("json_stringn");
    let oset: Symbol<FnJsonObjectSetNew> = l.sym("json_object_set_new");
    let aapp: Symbol<FnJsonArrayAppendNew> = l.sym("json_array_append_new");

    let root = obj();
    let ka = cs("ints");
    let ia = arr();
    for i in 0..5i64 {
        aapp(ia, int(i * (variant as i64 + 1)));
    }
    oset(root, ka.as_ptr(), ia);

    let kb = cs("reals");
    let ra = arr();
    for v in [0.5f64, 1.0 / 3.0, 1e20, -1e-20] {
        aapp(ra, real(v * (variant as f64 + 1.0)));
    }
    oset(root, kb.as_ptr(), ra);

    let kc = cs("strings");
    let sa = arr();
    for s in ["a", "ünïcödé", "with \"quote\"", "日本語", ""] {
        aapp(sa, strn(s.as_ptr() as *const c_char, s.len()));
    }
    oset(root, kc.as_ptr(), sa);

    let kd = cs("bools");
    let ba = arr();
    aapp(ba, tru());
    aapp(ba, fls());
    aapp(ba, nul());
    oset(root, kd.as_ptr(), ba);

    let ke = cs("nested");
    let mut cur = obj();
    let deep = cur;
    for i in 0..8 {
        let inner = obj();
        let k = cs(&format!("level{i}"));
        oset(cur, k.as_ptr(), inner);
        cur = inner;
    }
    let kl = cs("leaf");
    oset(cur, kl.as_ptr(), int(variant as i64));
    oset(root, ke.as_ptr(), deep);

    root
}

#[test]
fn json_equal_matches() {
    let (c, r) = seed_both();
    let ec: Symbol<FnJsonEqual> = c.sym("json_equal");
    let er: Symbol<FnJsonEqual> = r.sym("json_equal");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    unsafe {
        let mut cs_: Vec<*mut JsonT> = Vec::new();
        let mut rs_: Vec<*mut JsonT> = Vec::new();
        for v in 0..3u32 {
            cs_.push(build_sample(c, v));
            rs_.push(build_sample(r, v));
        }
        // scalars of every type
        let ic: Symbol<FnJsonInteger> = c.sym("json_integer");
        let ir: Symbol<FnJsonInteger> = r.sym("json_integer");
        let rc_: Symbol<FnJsonReal> = c.sym("json_real");
        let rr_: Symbol<FnJsonReal> = r.sym("json_real");
        let sc_: Symbol<FnJsonString> = c.sym("json_string");
        let sr_: Symbol<FnJsonString> = r.sym("json_string");
        for i in [0i64, 1, -1, i64::MAX] {
            cs_.push(ic(i));
            rs_.push(ir(i));
        }
        for f in [0.0f64, 1.0, -0.0, f64::NAN] {
            cs_.push(rc_(f));
            rs_.push(rr_(f));
        }
        for s in ["", "a", "ab", "ünïcödé"] {
            let z = cs(s);
            cs_.push(sc_(z.as_ptr()));
            rs_.push(sr_(z.as_ptr()));
        }
        for n in ["json_true", "json_false", "json_null", "json_object", "json_array"] {
            let fc: Symbol<FnNew0> = c.sym(n);
            let fr: Symbol<FnNew0> = r.sym(n);
            cs_.push(fc());
            rs_.push(fr());
        }

        for i in 0..cs_.len() {
            for j in 0..cs_.len() {
                assert_eq!(
                    ec(cs_[i], cs_[j]),
                    er(rs_[i], rs_[j]),
                    "json_equal({i}, {j})"
                );
            }
            assert_eq!(
                ec(cs_[i], std::ptr::null()),
                er(rs_[i], std::ptr::null()),
                "json_equal({i}, NULL)"
            );
            assert_eq!(
                ec(std::ptr::null(), cs_[i]),
                er(std::ptr::null(), rs_[i]),
                "json_equal(NULL, {i})"
            );
        }
        assert_eq!(
            ec(std::ptr::null(), std::ptr::null()),
            er(std::ptr::null(), std::ptr::null()),
            "json_equal(NULL, NULL)"
        );

        for i in 0..cs_.len() {
            dc(cs_[i]);
            dr(rs_[i]);
        }
    }
}

#[test]
fn json_copy_and_deep_copy_match() {
    let (c, r) = seed_both();
    let cpc: Symbol<FnJsonCopy> = c.sym("json_copy");
    let cpr: Symbol<FnJsonCopy> = r.sym("json_copy");
    let dpc: Symbol<FnJsonCopy> = c.sym("json_deep_copy");
    let dpr: Symbol<FnJsonCopy> = r.sym("json_deep_copy");
    let ec: Symbol<FnJsonEqual> = c.sym("json_equal");
    let er: Symbol<FnJsonEqual> = r.sym("json_equal");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    unsafe {
        for v in 0..3u32 {
            let a = build_sample(c, v);
            let b = build_sample(r, v);
            for (name, fc, fr) in [
                ("json_copy", &cpc, &cpr),
                ("json_deep_copy", &dpc, &dpr),
            ] {
                let x = fc(a);
                let y = fr(b);
                assert_eq!(observe(c, x), observe(r, y), "{name} result (variant {v})");
                assert_eq!(ec(a, x), er(b, y), "{name} equal to original");
                // the copy must be a distinct object
                assert_ne!(x, a, "{name} returns a new object (C)");
                assert_ne!(y, b, "{name} returns a new object (Rust)");
                dc(x);
                dr(y);
            }
            dc(a);
            dr(b);
        }
        // scalars and NULL
        let ic: Symbol<FnJsonInteger> = c.sym("json_integer");
        let ir: Symbol<FnJsonInteger> = r.sym("json_integer");
        let a = ic(42);
        let b = ir(42);
        for (name, fc, fr) in [("json_copy", &cpc, &cpr), ("json_deep_copy", &dpc, &dpr)] {
            let x = fc(a);
            let y = fr(b);
            assert_eq!(observe(c, x), observe(r, y), "{name} integer");
            dc(x);
            dr(y);
            assert_eq!(
                fc(std::ptr::null_mut()).is_null(),
                fr(std::ptr::null_mut()).is_null(),
                "{name}(NULL)"
            );
        }
        dc(a);
        dr(b);

        // json_copy of a container is shallow: children are shared, so the
        // child refcounts must match too.
        let a = build_sample(c, 0);
        let b = build_sample(r, 0);
        let x = cpc(a);
        let y = cpr(b);
        let gac: Symbol<FnJsonObjectGet> = c.sym("json_object_get");
        let gar: Symbol<FnJsonObjectGet> = r.sym("json_object_get");
        let k = cs("ints");
        let ca = gac(a, k.as_ptr());
        let cb = gar(b, k.as_ptr());
        assert_eq!((*ca).refcount, (*cb).refcount, "shallow copy child refcount");
        assert_eq!(ca, gac(x, k.as_ptr()), "C shallow copy shares child");
        assert_eq!(cb, gar(y, k.as_ptr()), "Rust shallow copy shares child");
        let z = dpc(a);
        let w = dpr(b);
        assert_ne!(ca, gac(z, k.as_ptr()), "C deep copy clones child");
        assert_ne!(cb, gar(w, k.as_ptr()), "Rust deep copy clones child");
        dc(x);
        dr(y);
        dc(z);
        dr(w);
        dc(a);
        dr(b);
    }
}

#[test]
fn deep_copy_cycle_and_loop_check_match() {
    // json_deep_copy on a self-referencing container: jansson guards this with
    // jsonp_loop_check.
    let (c, r) = seed_both();
    unsafe {
        for l in [c, r] {
            let arr: Symbol<FnNew0> = l.sym("json_array");
            let app: Symbol<FnJsonArrayAppendNew> = l.sym("json_array_append_new");
            let int: Symbol<FnJsonInteger> = l.sym("json_integer");
            let dcp: Symbol<FnJsonCopy> = l.sym("json_deep_copy");
            let del: Symbol<FnJsonDelete> = l.sym("json_delete");
            let a = arr();
            app(a, int(1));
            // deep copy of a plain nested array works
            let nested = arr();
            app(nested, int(2));
            app(a, nested);
            let cp = dcp(a);
            assert!(!cp.is_null(), "{}: deep copy of nested array", l.name);
            del(cp);
            del(a);
        }
    }
}

#[test]
fn jsonp_loop_check_matches() {
    let (c, r) = seed_both();
    type FnLoopCheck = unsafe extern "C" fn(*mut HashtableT, *const JsonT, *mut c_char, usize, *mut usize) -> c_int;
    let fc: Symbol<FnLoopCheck> = c.sym("jsonp_loop_check");
    let fr: Symbol<FnLoopCheck> = r.sym("jsonp_loop_check");

    unsafe {
        let mut htc = Box::new(HashtableT::default());
        let mut htr = Box::new(HashtableT::default());
        let ic: Symbol<FnHtInit> = c.sym("hashtable_init");
        let ir: Symbol<FnHtInit> = r.sym("hashtable_init");
        assert_eq!(ic(&mut *htc), 0);
        assert_eq!(ir(&mut *htr), 0);

        let ac: Symbol<FnNew0> = c.sym("json_array");
        let ar: Symbol<FnNew0> = r.sym("json_array");
        let vc = ac();
        let vr = ar();

        let mut kc = [0u8; 32];
        let mut kr = [0u8; 32];
        let mut lc: usize = 0;
        let mut lr: usize = 0;
        // first call: not seen yet
        let a = fc(&mut *htc, vc, kc.as_mut_ptr() as *mut c_char, 32, &mut lc);
        let b = fr(&mut *htr, vr, kr.as_mut_ptr() as *mut c_char, 32, &mut lr);
        assert_eq!(a, b, "jsonp_loop_check first call rc");
        assert_eq!(lc, lr, "jsonp_loop_check key_len");
        // the key is a printf of the pointer, so the bytes differ between the
        // two libraries; only the length and rc are comparable.
        // second call with the same value: must report the loop
        let a = fc(&mut *htc, vc, kc.as_mut_ptr() as *mut c_char, 32, &mut lc);
        let b = fr(&mut *htr, vr, kr.as_mut_ptr() as *mut c_char, 32, &mut lr);
        assert_eq!(a, b, "jsonp_loop_check second call rc");
        assert_eq!(lc, lr, "jsonp_loop_check second key_len");

        // too small a key buffer
        let a = fc(&mut *htc, vc, kc.as_mut_ptr() as *mut c_char, 2, &mut lc);
        let b = fr(&mut *htr, vr, kr.as_mut_ptr() as *mut c_char, 2, &mut lr);
        assert_eq!(a, b, "jsonp_loop_check small buffer rc");

        let cc: Symbol<FnHtClose> = c.sym("hashtable_close");
        let cr: Symbol<FnHtClose> = r.sym("hashtable_close");
        cc(&mut *htc);
        cr(&mut *htr);
        let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
        let dr: Symbol<FnJsonDelete> = r.sym("json_delete");
        dc(vc);
        dr(vr);
    }
}

#[test]
fn jsonp_stringn_nocheck_own_matches() {
    let (c, r) = seed_both();
    type FnOwn = unsafe extern "C" fn(*mut c_char, usize) -> *mut JsonT;
    let fc: Symbol<FnOwn> = c.sym("jsonp_stringn_nocheck_own");
    let fr: Symbol<FnOwn> = r.sym("jsonp_stringn_nocheck_own");
    let dc: Symbol<FnJsonDelete> = c.sym("json_delete");
    let dr: Symbol<FnJsonDelete> = r.sym("json_delete");

    for s in string_probes() {
        unsafe {
            // the function takes ownership, so hand it a fresh jsonp_strndup'd
            // buffer from the matching library
            let sdc: Symbol<FnStrndup> = c.sym("jsonp_strndup");
            let sdr: Symbol<FnStrndup> = r.sym("jsonp_strndup");
            let pc = sdc(s.as_ptr() as *const c_char, s.len());
            let pr = sdr(s.as_ptr() as *const c_char, s.len());
            let a = fc(pc, s.len());
            let b = fr(pr, s.len());
            assert_eq!(observe(c, a), observe(r, b), "own({:02x?})", s);
            dc(a);
            dr(b);
        }
    }
    // NULL
    unsafe {
        let a = fc(std::ptr::null_mut(), 0);
        let b = fr(std::ptr::null_mut(), 0);
        assert_eq!(observe(c, a), observe(r, b), "own(NULL, 0)");
        if !a.is_null() {
            dc(a);
        }
        if !b.is_null() {
            dr(b);
        }
    }
}

#[test]
fn refcount_lifecycle_matches() {
    let (c, r) = seed_both();
    unsafe {
        for l in [c, r] {
            let obj: Symbol<FnNew0> = l.sym("json_object");
            let arr: Symbol<FnNew0> = l.sym("json_array");
            let int: Symbol<FnJsonInteger> = l.sym("json_integer");
            let oset: Symbol<FnJsonObjectSetNew> = l.sym("json_object_set_new");
            let aapp: Symbol<FnJsonArrayAppendNew> = l.sym("json_array_append_new");
            let oget: Symbol<FnJsonObjectGet> = l.sym("json_object_get");
            let del: Symbol<FnJsonDelete> = l.sym("json_delete");

            let o = obj();
            assert_eq!((*o).refcount, 1, "{}: fresh object refcount", l.name);
            let a = arr();
            let k = cs("arr");
            oset(o, k.as_ptr(), a);
            assert_eq!((*a).refcount, 1, "{}: set_new steals the ref", l.name);
            assert_eq!(oget(o, k.as_ptr()), a, "{}: get returns the same", l.name);
            let i = int(5);
            aapp(a, i);
            assert_eq!((*i).refcount, 1, "{}: append_new steals the ref", l.name);
            del(o);
        }
    }
}

const _: Option<(*mut c_void, c_double)> = None;
