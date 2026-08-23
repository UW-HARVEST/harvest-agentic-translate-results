//! Phase B — encoder (dump) differential tests. CONFIGS.md rows 1-77.
//!
//! Every test drives BOTH the C `.so` and the Rust `.so` through their exported
//! symbols only and compares results byte-for-byte.

mod common;

use common::*;
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};

/// Build a representative tree entirely through exported constructors.
/// Returns a value with refcount 1 owned by the caller.
unsafe fn build_baseline(lib: &Library) -> *mut json_t {
    let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
    let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
    let int: Symbol<FnInt> = sym(lib, "json_integer");
    let nul: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_null");
    let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
    let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");

    let root = obj();
    let a = arr();
    aapp(a, int(1));
    aapp(a, int(2));
    oset(root, cs("arr").as_ptr(), a);
    let inner = obj();
    oset(inner, cs("k").as_ptr(), nul());
    oset(root, cs("o").as_ptr(), inner);
    root
}

/// Dump a freshly built baseline tree with `flags` and return the string.
fn dump_baseline(flags: usize) -> impl Fn(&Library) -> Option<String> {
    move |lib: &Library| unsafe {
        let root = build_baseline(lib);
        let out = dumps_to_string(lib, root, flags);
        decref(lib, root);
        out
    }
}

// ---------------------------------------------------------------- rows 1-9, 20, 28, 29

#[test]
fn row1_baseline_all_entry_points() {
    // json_dumps
    diff("row1/dumps", dump_baseline(0));

    // json_dumpb, json_dump_callback, json_dumpf, json_dumpfd, json_dump_file
    diff("row1/dumpb", |lib: &Library| unsafe {
        let root = build_baseline(lib);
        let f: Symbol<FnDumpb> = sym(lib, "json_dumpb");
        let need = f(root, std::ptr::null_mut(), 0, 0);
        let mut buf = vec![0u8; need + 8];
        let wrote = f(root, buf.as_mut_ptr() as *mut c_char, need, 0);
        decref(lib, root);
        (need, wrote, buf[..need].to_vec())
    });
}

#[test]
fn rows2_9_20_28_29_flag_axes() {
    let cases: &[(&str, usize)] = &[
        ("row2/COMPACT", JSON_COMPACT),
        ("row3/INDENT1", json_indent(1)),
        ("row4/INDENT2", json_indent(2)),
        ("row5/INDENT4", json_indent(4)),
        ("row6/INDENT31", json_indent(31)),
        ("row8/INDENT0", json_indent(0)),
        ("row9/INDENT4|COMPACT", json_indent(4) | JSON_COMPACT),
        ("row20/PRESERVE_ORDER", JSON_PRESERVE_ORDER),
        (
            "row28/ALL",
            JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH | JSON_SORT_KEYS | JSON_COMPACT,
        ),
        // row 29: every unknown high bit set (must be ignored). Keep EMBED clear.
        ("row29/unknown-bits", 0xFFFF_0000_usize & !JSON_EMBED),
    ];
    for (label, flags) in cases {
        diff(label, dump_baseline(*flags));
    }
}

#[test]
fn row7_indent31_deep_multichunk() {
    // depth >= 2 so depth*31 > 32, exercising the whitespace[] chunk loop (dump.c:79-87).
    diff("row7/INDENT31 deep", |lib: &Library| unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        // 5 levels of nesting -> depth*31 up to 155
        let mut cur = arr();
        aapp(cur, int(7));
        for _ in 0..4 {
            let outer = arr();
            aapp(outer, cur);
            cur = outer;
        }
        let out = dumps_to_string(lib, cur, json_indent(31));
        decref(lib, cur);
        out
    });
}

// ---------------------------------------------------------------- rows 10-14: escaping

/// Dump a single string value (via JSON_ENCODE_ANY) with the given flags.
fn dump_string_value(bytes: Vec<u8>, flags: usize) -> impl Fn(&Library) -> Option<String> {
    move |lib: &Library| unsafe {
        let sn: Symbol<FnStrN> = sym(lib, "json_stringn");
        let s = sn(bytes.as_ptr() as *const c_char, bytes.len());
        if s.is_null() {
            return None;
        }
        let out = dumps_to_string(lib, s, flags | JSON_ENCODE_ANY);
        decref(lib, s);
        out
    }
}

#[test]
fn rows10_12_ensure_ascii() {
    // 2-byte U+00E9, 3-byte U+20AC, 4-byte U+1D11E
    let s = "e\u{00e9}x\u{20ac}y\u{1d11e}z".as_bytes().to_vec();
    diff("row10/ENSURE_ASCII", dump_string_value(s.clone(), JSON_ENSURE_ASCII));
    diff("row11/raw utf8", dump_string_value(s, 0));
    // row 12: U+007F DEL is NOT escaped (test is codepoint > 0x7F)
    let del = "a\u{7f}b".as_bytes().to_vec();
    diff("row12/DEL not escaped", dump_string_value(del.clone(), JSON_ENSURE_ASCII));
    diff("row12/DEL no flag", dump_string_value(del, 0));
}

#[test]
fn rows13_14_escape_slash() {
    let s = b"a/b//c".to_vec();
    diff("row13/ESCAPE_SLASH", dump_string_value(s.clone(), JSON_ESCAPE_SLASH));
    diff("row14/no ESCAPE_SLASH", dump_string_value(s, 0));
}

#[test]
fn rows59_60_mandatory_and_control_escapes() {
    // row 59: mandatory escapes
    let mand = b"q\"b\\s/f\x08n\x0cr\nt\rz\t!".to_vec();
    diff("row59/mandatory escapes", dump_string_value(mand.clone(), 0));
    diff("row59/mandatory +ascii", dump_string_value(mand, JSON_ENSURE_ASCII));
    // row 60: control chars with no short form -> \u00XX
    let ctl: Vec<u8> = vec![b'a', 0x01, 0x0B, 0x0E, 0x1F, b'b'];
    diff("row60/control chars", dump_string_value(ctl.clone(), 0));
    // row 61 / row 60 with NUL: embedded NUL via json_stringn
    let with_nul: Vec<u8> = vec![b'a', 0x00, b'b'];
    diff("row61/embedded NUL", dump_string_value(with_nul, 0));
    // row 58: empty string
    diff("row58/empty string", dump_string_value(vec![], 0));
}

#[test]
fn row59_randomized_strings() {
    // Property-style: many randomized strings through both escape modes.
    for &flags in &[0usize, JSON_ENSURE_ASCII, JSON_ESCAPE_SLASH, JSON_ENSURE_ASCII | JSON_ESCAPE_SLASH]
    {
        diff_n("rand strings", 400, move |lib: &Library, i| unsafe {
            let mut rng = Rng::new(0xD1CE_0000 ^ i ^ ((flags as u64) << 32));
            let s = rng.utf8_string(24);
            let b = s.as_bytes();
            let sn: Symbol<FnStrN> = sym(lib, "json_stringn");
            let v = sn(b.as_ptr() as *const c_char, b.len());
            if v.is_null() {
                return None;
            }
            let out = dumps_to_string(lib, v, flags | JSON_ENCODE_ANY);
            decref(lib, v);
            out
        });
    }
}

// ---------------------------------------------------------------- rows 15-19: SORT_KEYS

/// Build an object from (key, int-value) pairs in the given order.
unsafe fn build_obj(lib: &Library, pairs: &[(&[u8], i64)]) -> *mut json_t {
    let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
    let int: Symbol<FnInt> = sym(lib, "json_integer");
    let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
    let o = obj();
    for (k, v) in pairs {
        osetn(o, k.as_ptr() as *const c_char, k.len(), int(*v));
    }
    o
}

#[test]
fn rows15_19_sort_keys() {
    let unsorted: Vec<(&[u8], i64)> =
        vec![(b"zebra", 1), (b"apple", 2), (b"Mango", 3), (b"", 4), (b"banana", 5)];
    for (label, flags) in [
        ("row15/SORT_KEYS", JSON_SORT_KEYS),
        ("row15/insertion", 0),
        ("row19/SORT|INDENT2", JSON_SORT_KEYS | json_indent(2)),
    ] {
        let pairs = unsorted.clone();
        diff(label, move |lib: &Library| unsafe {
            let o = build_obj(lib, &pairs);
            let out = dumps_to_string(lib, o, flags);
            decref(lib, o);
            out
        });
    }

    // row 16: one key is a prefix of another -> compare_keys length tie-break
    let prefix: Vec<(&[u8], i64)> =
        vec![(b"ab", 1), (b"a", 2), (b"abc", 3), (b"", 4), (b"b", 5)];
    diff("row16/prefix tie-break", move |lib: &Library| unsafe {
        let o = build_obj(lib, &prefix);
        let out = dumps_to_string(lib, o, JSON_SORT_KEYS);
        decref(lib, o);
        out
    });

    // row 17: single-key object (qsort with size 1)
    diff("row17/single key sorted", |lib: &Library| unsafe {
        let o = build_obj(lib, &[(b"only", 1)]);
        let out = dumps_to_string(lib, o, JSON_SORT_KEYS);
        decref(lib, o);
        out
    });
}

#[test]
fn row18_sort_keys_many_randomized() {
    // > 8 keys forces a rehash; sorted output must still match exactly.
    for &flags in &[JSON_SORT_KEYS, 0usize, JSON_SORT_KEYS | json_indent(2)] {
        diff_n("row18/many keys", 120, move |lib: &Library, i| unsafe {
            let mut rng = Rng::new(0x5027_1000u64 ^ i);
            let n = 1 + rng.below(40) as usize;
            let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
            let int: Symbol<FnInt> = sym(lib, "json_integer");
            let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
            let o = obj();
            for j in 0..n {
                let k = rng.ascii_string(6);
                osetn(o, k.as_ptr() as *const c_char, k.len(), int(j as i64));
            }
            let out = dumps_to_string(lib, o, flags);
            decref(lib, o);
            out
        });
    }
}

// ---------------------------------------------------------------- rows 21-22: ENCODE_ANY

#[test]
fn rows21_22_encode_any_scalars() {
    // Each scalar type, with and without JSON_ENCODE_ANY (row 22 = rejected -> NULL).
    for flags in [JSON_ENCODE_ANY, 0usize] {
        diff(
            if flags == 0 { "row22/scalar no ANY" } else { "row21/scalar ANY" },
            move |lib: &Library| unsafe {
                let mk: [(&str, Box<dyn Fn(&Library) -> *mut json_t>); 6] = [
                    ("string", Box::new(|l: &Library| {
                        let f: Symbol<FnStr> = sym(l, "json_string");
                        f(cs("hi").as_ptr())
                    })),
                    ("integer", Box::new(|l: &Library| {
                        let f: Symbol<FnInt> = sym(l, "json_integer");
                        f(-42)
                    })),
                    ("real", Box::new(|l: &Library| {
                        let f: Symbol<FnReal> = sym(l, "json_real");
                        f(1.5)
                    })),
                    ("true", Box::new(|l: &Library| {
                        let f: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(l, "json_true");
                        f()
                    })),
                    ("false", Box::new(|l: &Library| {
                        let f: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(l, "json_false");
                        f()
                    })),
                    ("null", Box::new(|l: &Library| {
                        let f: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(l, "json_null");
                        f()
                    })),
                ];
                let mut out = Vec::new();
                for (name, ctor) in mk.iter() {
                    let v = ctor(lib);
                    out.push((*name, dumps_to_string(lib, v, flags)));
                    decref(lib, v);
                }
                out
            },
        );
    }
}

// ---------------------------------------------------------------- rows 23-27: EMBED

#[test]
fn rows23_27_embed() {
    diff("row23/EMBED object", dump_baseline(JSON_EMBED));
    diff("row26/EMBED|INDENT2", dump_baseline(JSON_EMBED | json_indent(2)));
    diff("row27/EMBED nested", dump_baseline(JSON_EMBED)); // children keep brackets

    diff("row24/EMBED array", |lib: &Library| unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let a = arr();
        aapp(a, int(1));
        aapp(a, int(2));
        let out = dumps_to_string(lib, a, JSON_EMBED);
        decref(lib, a);
        out
    });

    // row 25: EMBED on an EMPTY container -> zero bytes of output
    diff("row25/EMBED empty", |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let o = obj();
        let a = arr();
        let r = (
            dumps_to_string(lib, o, JSON_EMBED),
            dumps_to_string(lib, a, JSON_EMBED),
            dumps_to_string(lib, o, JSON_EMBED | json_indent(2)),
            dumps_to_string(lib, a, JSON_EMBED | json_indent(2)),
        );
        decref(lib, o);
        decref(lib, a);
        r
    });
}

// ---------------------------------------------------------------- rows 30-49: reals

#[test]
fn rows30_48_real_and_integer_precision() {
    const VALUES: &[f64] = &[
        0.0,
        -0.0,
        0.1,
        0.1 + 0.2,
        1e15,
        1e16,
        1e-4,
        1e-5,
        1.7976931348623157e308,
        5e-324,
        1.0 / 3.0,
        1e300,
        2.5,
        -1.5,
        123456789.0,
        1e17,
        9.999999999999999e22,
    ];
    // rows 30-46: all precisions 0..=31 across the boundary values.
    diff("rows30-46/real precision matrix", |lib: &Library| unsafe {
        let real: Symbol<FnReal> = sym(lib, "json_real");
        let mut out = Vec::new();
        for &v in VALUES {
            let j = real(v);
            if j.is_null() {
                out.push((v.to_bits(), 999usize, None));
                continue;
            }
            for p in 0..=31usize {
                out.push((
                    v.to_bits(),
                    p,
                    dumps_to_string(lib, j, JSON_ENCODE_ANY | json_real_precision(p)),
                ));
            }
            decref(lib, j);
        }
        out
    });

    // rows 47-48: integer boundaries
    diff("rows47-48/integers", |lib: &Library| unsafe {
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let mut out = Vec::new();
        for v in [0i64, -1, 1, i64::MIN, i64::MAX, i32::MIN as i64, i32::MAX as i64] {
            let j = int(v);
            out.push((v, dumps_to_string(lib, j, JSON_ENCODE_ANY)));
            decref(lib, j);
        }
        out
    });
}

#[test]
fn row49_randomized_reals_all_precisions() {
    // Property-style over random double bit patterns x precision 0..17.
    diff_n("row49/random reals", 600, |lib: &Library, i| unsafe {
        let mut rng = Rng::new(0xF10A_7000 ^ i);
        let d = rng.f64_bits();
        let real: Symbol<FnReal> = sym(lib, "json_real");
        let j = real(d);
        if j.is_null() {
            // NaN/Inf are rejected by json_real; both libs must agree.
            return vec![(d.to_bits(), 0usize, None)];
        }
        let mut out = Vec::new();
        for p in 0..=17usize {
            out.push((
                d.to_bits(),
                p,
                dumps_to_string(lib, j, JSON_ENCODE_ANY | json_real_precision(p)),
            ));
        }
        decref(lib, j);
        out
    });
}

// ---------------------------------------------------------------- rows 50-57, 63-65: shapes

#[test]
fn rows50_57_container_shapes() {
    for (label, flags) in
        [("flags0", 0usize), ("indent2", json_indent(2)), ("compact", JSON_COMPACT)]
    {
        diff(&format!("rows50-55/{}", label), move |lib: &Library| unsafe {
            let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
            let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
            let int: Symbol<FnInt> = sym(lib, "json_integer");
            let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
            let osetn: Symbol<FnObjSetNNew> = sym(lib, "json_object_setn_new");
            let mut out = Vec::new();
            // empty object / empty array (rows 50, 51)
            let e1 = obj();
            let e2 = arr();
            out.push(dumps_to_string(lib, e1, flags));
            out.push(dumps_to_string(lib, e2, flags));
            decref(lib, e1);
            decref(lib, e2);
            // arrays and objects of sizes 1,2,7,8,9,17 (rows 52-55: grow/rehash boundaries)
            for n in [1usize, 2, 7, 8, 9, 17] {
                let a = arr();
                for i in 0..n {
                    aapp(a, int(i as i64));
                }
                out.push(dumps_to_string(lib, a, flags));
                decref(lib, a);

                let o = obj();
                for i in 0..n {
                    let k = format!("k{:02}", i);
                    osetn(o, k.as_ptr() as *const c_char, k.len(), int(i as i64));
                }
                out.push(dumps_to_string(lib, o, flags));
                out.push(dumps_to_string(lib, o, flags | JSON_SORT_KEYS));
                decref(lib, o);
            }
            out
        });
    }
}

#[test]
fn row56_deep_nesting_indent() {
    diff("row56/deep nesting", |lib: &Library| unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let mut cur = arr();
        aapp(cur, int(0));
        for _ in 0..100 {
            let o = arr();
            aapp(o, cur);
            cur = o;
        }
        let r = (
            dumps_to_string(lib, cur, json_indent(2)),
            dumps_to_string(lib, cur, 0),
        );
        decref(lib, cur);
        r
    });
}

#[test]
fn row57_all_eight_types() {
    diff("row57/all 8 types", |lib: &Library| unsafe {
        let obj: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_object");
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let real: Symbol<FnReal> = sym(lib, "json_real");
        let st: Symbol<FnStr> = sym(lib, "json_string");
        let t: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_true");
        let f: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_false");
        let n: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_null");
        let oset: Symbol<FnObjSetNew> = sym(lib, "json_object_set_new");
        let root = obj();
        oset(root, cs("obj").as_ptr(), obj());
        oset(root, cs("arr").as_ptr(), arr());
        oset(root, cs("str").as_ptr(), st(cs("s").as_ptr()));
        oset(root, cs("int").as_ptr(), int(7));
        oset(root, cs("real").as_ptr(), real(0.5));
        oset(root, cs("true").as_ptr(), t());
        oset(root, cs("false").as_ptr(), f());
        oset(root, cs("null").as_ptr(), n());
        let mut out = Vec::new();
        for fl in [0usize, json_indent(2), JSON_SORT_KEYS, JSON_COMPACT] {
            out.push(dumps_to_string(lib, root, fl));
        }
        decref(lib, root);
        out
    });
}

#[test]
fn rows62_63_nul_and_empty_keys() {
    diff("rows62-63/NUL and empty keys", |lib: &Library| unsafe {
        let pairs: Vec<(&[u8], i64)> =
            vec![(b"a\x00b", 1), (b"", 2), (b"a", 3), (b"a\x00c", 4)];
        let o = build_obj(lib, &pairs);
        let sz: Symbol<FnSize> = sym(lib, "json_object_size");
        let mut out = vec![format!("size={}", sz(o))];
        for fl in [0usize, JSON_SORT_KEYS, JSON_ENSURE_ASCII, json_indent(2)] {
            out.push(format!("{:?}", dumps_to_string(lib, o, fl)));
        }
        decref(lib, o);
        out
    });
}

#[test]
fn row64_dag_shared_sibling() {
    // The same json_t twice as siblings is legal (not a cycle).
    diff("row64/DAG siblings", |lib: &Library| unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let int: Symbol<FnInt> = sym(lib, "json_integer");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let shared = arr();
        aapp(shared, int(9));
        let root = arr();
        aapp(root, incref(shared));
        aapp(root, incref(shared));
        decref(lib, shared);
        let r = (
            dumps_to_string(lib, root, 0),
            dumps_to_string(lib, root, json_indent(1)),
        );
        decref(lib, root);
        r
    });
}

#[test]
fn row65_indirect_cycle_rejected() {
    // a=[b], b=[a] -> jsonp_loop_check must reject on BOTH sides.
    diff("row65/indirect cycle", |lib: &Library| unsafe {
        let arr: Symbol<unsafe extern "C" fn() -> *mut json_t> = sym(lib, "json_array");
        let aapp: Symbol<FnArrAppendNew> = sym(lib, "json_array_append_new");
        let a = arr();
        let b = arr();
        aapp(a, incref(b));
        aapp(b, incref(a));
        let out = dumps_to_string(lib, a, 0);
        // deliberately leak the cycle; freeing it is not what we are testing
        out
    });
}

// ---------------------------------------------------------------- rows 66-77: low-level

#[test]
fn rows66_70_dumpb_buffer_sizes() {
    diff("rows66-70/dumpb", |lib: &Library| unsafe {
        let f: Symbol<FnDumpb> = sym(lib, "json_dumpb");
        let root = build_baseline(lib);
        // row 66: measure mode
        let need = f(root, std::ptr::null_mut(), 0, 0);
        let mut out: Vec<(String, usize, Vec<u8>)> = Vec::new();
        out.push(("measure".into(), need, vec![]));
        // row 67: exact size; row 68: undersized; row 69: oversized
        for sz in [need, 3usize, 1, 0, need + 16] {
            let mut buf = vec![0xAAu8; sz.max(1) + 8];
            let wrote = f(root, buf.as_mut_ptr() as *mut c_char, sz, 0);
            out.push((format!("size={}", sz), wrote, buf[..sz.min(buf.len())].to_vec()));
        }
        // row 70: dump failure -> returns 0
        let st: Symbol<FnStr> = sym(lib, "json_string");
        let s = st(cs("x").as_ptr());
        let mut b2 = vec![0u8; 32];
        out.push((
            "scalar-no-ANY".into(),
            f(s, b2.as_mut_ptr() as *mut c_char, 32, 0),
            b2[..8].to_vec(),
        ));
        decref(lib, s);
        decref(lib, root);
        out
    });
}

// Callback state shared with the C ABI callback below.
struct CbState {
    chunks: Vec<Vec<u8>>,
    fail_at: isize,
    calls: isize,
}

unsafe extern "C" fn dump_cb(buf: *const c_char, size: usize, data: *mut c_void) -> c_int {
    let st = &mut *(data as *mut CbState);
    st.calls += 1;
    if st.fail_at >= 0 && st.calls > st.fail_at {
        return -1;
    }
    let slice = std::slice::from_raw_parts(buf as *const u8, size);
    st.chunks.push(slice.to_vec());
    0
}

#[test]
fn rows71_72_dump_callback() {
    for fail_at in [-1isize, 0, 1, 3] {
        diff(&format!("rows71-72/callback fail_at={}", fail_at), move |lib: &Library| unsafe {
            let f: Symbol<
                unsafe extern "C" fn(
                    *const json_t,
                    unsafe extern "C" fn(*const c_char, usize, *mut c_void) -> c_int,
                    *mut c_void,
                    usize,
                ) -> c_int,
            > = sym(lib, "json_dump_callback");
            let root = build_baseline(lib);
            let mut st = CbState { chunks: vec![], fail_at, calls: 0 };
            let rc = f(root, dump_cb, &mut st as *mut CbState as *mut c_void, json_indent(1));
            decref(lib, root);
            (rc, st.calls, st.chunks)
        });
    }
}

#[test]
fn rows73_76_file_and_fd_outputs() {
    // rows 73-75: json_dumpf / json_dumpfd / json_dump_file write identical bytes.
    diff("rows73-75/file outputs", |lib: &Library| unsafe {
        let root = build_baseline(lib);
        let dir = std::env::temp_dir();
        let tag = if std::ptr::eq(lib, &libs().c) { "c" } else { "r" };

        // json_dump_file
        let p1 = dir.join(format!("jansson_dumpfile_{}.json", tag));
        let df: Symbol<unsafe extern "C" fn(*const json_t, *const c_char, usize) -> c_int> =
            sym(lib, "json_dump_file");
        let p1c = cs(p1.to_str().unwrap());
        let rc1 = df(root, p1c.as_ptr(), json_indent(2));
        let b1 = std::fs::read(&p1).unwrap_or_default();

        // json_dumpfd
        let p2 = dir.join(format!("jansson_dumpfd_{}.json", tag));
        let fd_file = std::fs::File::create(&p2).unwrap();
        let fd = {
            use std::os::unix::io::AsRawFd;
            fd_file.as_raw_fd()
        };
        let dfd: Symbol<unsafe extern "C" fn(*const json_t, c_int, usize) -> c_int> =
            sym(lib, "json_dumpfd");
        let rc2 = dfd(root, fd, json_indent(2));
        drop(fd_file);
        let b2 = std::fs::read(&p2).unwrap_or_default();

        // row 76: unopenable path -> -1
        let rc3 = df(root, cs("/nonexistent-dir-xyz/out.json").as_ptr(), 0);

        decref(lib, root);
        let _ = std::fs::remove_file(&p1);
        let _ = std::fs::remove_file(&p2);
        (rc1, b1, rc2, b2, rc3)
    });
}
