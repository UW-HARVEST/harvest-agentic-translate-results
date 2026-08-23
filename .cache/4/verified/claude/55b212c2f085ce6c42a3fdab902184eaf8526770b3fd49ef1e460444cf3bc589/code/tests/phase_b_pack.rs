//! Phase B — differential tests for pack/unpack (CONFIGS.md rows 114..129).

mod common;
use common::tree::*;
use common::*;
use std::os::raw::{c_char, c_int, c_longlong, c_void};
use std::ptr;

/// `json_pack*` ignores its `flags` argument entirely, so every bit pattern is
/// valid there.
const PACK_FLAGS: &[(&str, usize)] = &[
    ("0", 0),
    ("validate", JSON_VALIDATE_ONLY),
    ("strict", JSON_STRICT),
    ("both", JSON_VALIDATE_ONLY | JSON_STRICT),
    ("allbits", usize::MAX),
];

/// For `json_unpack*`, `JSON_VALIDATE_ONLY` changes how many varargs are
/// consumed, so formats that carry value arguments may only be used without it
/// (this mirrors the documented contract).  `JSON_VALIDATE_ONLY` gets its own
/// test with keys-only argument lists.
const UNPACK_FLAGS: &[(&str, usize)] = &[("0", 0), ("strict", JSON_STRICT)];

unsafe fn rec_packed(api: &Api, rec: &mut Rec, tag: &str, j: *mut Json, e: &JsonError) {
    rec.json(tag, j);
    rec.error(&format!("{tag}.err"), e);
    rec_dump_all(api, rec, tag, j);
    decref(api, j);
}

/* --------------------------------- rows 114..120: json_pack / json_pack_ex */

#[test]
fn cfg114to120_pack_scalars_and_containers() {
    diff("cfg114-120 pack", |api, rec| unsafe {
        let mut rng = Rng::new(0x1140);
        for _ in 0..120 {
            let iv = rng.range_i64(i32::MIN as i64, i32::MAX as i64) as c_int;
            let llv = rng.next_u64() as c_longlong;
            let dv = rng.f64_interesting();
            let n1 = rng.below(10);
            let s1 = cs(&rng.utf8(n1));
            let n2 = rng.below(10);
            let s2 = cs(&rng.utf8(n2));
            let k1 = cs(&format!("k{}", rng.below(5)));
            let k2 = cs(&format!("k{}", rng.below(5)));
            let bv = (rng.below(2) as c_int) * 7;

            for (fname, flags) in PACK_FLAGS {
                macro_rules! p {
                    ($tag:expr, $fmt:expr $(, $arg:expr)*) => {{
                        let f = cs($fmt);
                        let mut e = JsonError::patterned();
                        let j = (api.json_pack_ex)(&mut e, *flags, f.as_ptr() $(, $arg)*);
                        rec_packed(api, rec, &format!("{}.{}", fname, $tag), j, &e);
                    }};
                }
                p!("n", "n");
                p!("b", "b", bv);
                p!("i", "i", iv);
                p!("I", "I", llv);
                p!("f", "f", dv);
                p!("s", "s", s1.as_ptr());
                p!("arr_empty", "[]");
                p!("obj_empty", "{}");
                p!("arr1", "[i]", iv);
                p!("arr_many", "[i,I,f,s,b,n]", iv, llv, dv, s1.as_ptr(), bv);
                p!("obj1", "{s:i}", k1.as_ptr(), iv);
                p!(
                    "obj_many",
                    "{s:i,s:s}",
                    k1.as_ptr(),
                    iv,
                    k2.as_ptr(),
                    s1.as_ptr()
                );
                p!(
                    "nested",
                    "{s:[i,{s:s},[f]],s:n}",
                    k1.as_ptr(),
                    iv,
                    k2.as_ptr(),
                    s1.as_ptr(),
                    dv,
                    k2.as_ptr()
                );
                // row 117: length modifiers and concatenation
                p!("s_hash", "s#", s1.as_ptr(), n1 as c_int);
                p!("s_hash0", "s#", s1.as_ptr(), 0 as c_int);
                p!("s_pct", "s%", s1.as_ptr(), n1);
                p!("s_plus", "s+", s1.as_ptr(), s2.as_ptr());
                p!(
                    "s_plus_hash",
                    "s+#",
                    s1.as_ptr(),
                    s2.as_ptr(),
                    n2 as c_int
                );
                p!(
                    "s_hash_plus_pct",
                    "s#+%",
                    s1.as_ptr(),
                    n1 as c_int,
                    s2.as_ptr(),
                    n2
                );
                p!(
                    "obj_key_hash",
                    "{s#:i}",
                    k1.as_ptr(),
                    2 as c_int,
                    iv
                );
                // row 119: whitespace and separators inside the format
                p!("ws", " [ i , i ] ", iv, iv);
                p!("ws_obj", "{ s : i , s : i }", k1.as_ptr(), iv, k2.as_ptr(), iv);
                p!("newlines", "[\n\ti,\n\ti\n]", iv, iv);
            }
        }
    });
}

#[test]
fn cfg115and118_pack_object_refs_and_optionals() {
    diff("cfg115+118 pack O/o/optionals", |api, rec| unsafe {
        let mut rng = Rng::new(0x1150);
        for _ in 0..80 {
            let spec = rand_container(&mut rng, 2);
            for (fname, flags) in PACK_FLAGS {
                // 'O' increfs, so the caller keeps its own reference
                let src = build(api, &spec);
                let f = cs("[O]");
                let mut e = JsonError::patterned();
                let j = (api.json_pack_ex)(&mut e, *flags, f.as_ptr(), src);
                rec.json(&format!("{fname}.O.src"), src);
                rec_packed(api, rec, &format!("{fname}.O"), j, &e);
                rec.json(&format!("{fname}.O.src_after"), src);
                decref(api, src);

                // 'o' steals
                let src = build(api, &spec);
                let f = cs("[o]");
                let mut e = JsonError::patterned();
                let j = (api.json_pack_ex)(&mut e, *flags, f.as_ptr(), src);
                rec_packed(api, rec, &format!("{fname}.o"), j, &e);

                // optionals with a NULL argument
                for fmt in ["[s?]", "[s*]", "[O?]", "[O*]", "[o?]", "[o*]", "{s:s?}", "{s:s*}",
                            "{s:O?}", "{s:O*}", "{s:o?}", "{s:o*}"] {
                    let f = cs(fmt);
                    let mut e = JsonError::patterned();
                    let key = cs("k");
                    let j = if fmt.starts_with('{') {
                        (api.json_pack_ex)(
                            &mut e,
                            *flags,
                            f.as_ptr(),
                            key.as_ptr(),
                            ptr::null::<c_void>(),
                        )
                    } else {
                        (api.json_pack_ex)(&mut e, *flags, f.as_ptr(), ptr::null::<c_void>())
                    };
                    rec_packed(api, rec, &format!("{fname}.{fmt}.null"), j, &e);

                    // and with a real value
                    let val = build(api, &spec);
                    let sval = cs("text");
                    let mut e = JsonError::patterned();
                    let j = if fmt.starts_with('{') {
                        if fmt.contains('s') && fmt[3..].starts_with('s') {
                            (api.json_pack_ex)(
                                &mut e,
                                *flags,
                                f.as_ptr(),
                                key.as_ptr(),
                                sval.as_ptr(),
                            )
                        } else {
                            (api.json_pack_ex)(&mut e, *flags, f.as_ptr(), key.as_ptr(), val)
                        }
                    } else if fmt.starts_with("[s") {
                        (api.json_pack_ex)(&mut e, *flags, f.as_ptr(), sval.as_ptr())
                    } else {
                        (api.json_pack_ex)(&mut e, *flags, f.as_ptr(), val)
                    };
                    rec_packed(api, rec, &format!("{fname}.{fmt}.value"), j, &e);
                    // 'o' variants stole `val`; 'O' variants and the 's' ones did not
                    if fmt.contains('O') {
                        decref(api, val);
                    }
                }
            }
        }
    });
}

#[test]
fn cfg114_json_pack_plain_wrapper() {
    // json_pack() forwards to json_vpack_ex(NULL, 0, ...) — no error struct.
    diff("cfg114 json_pack", |api, rec| unsafe {
        let mut rng = Rng::new(0x1141);
        for _ in 0..200 {
            let iv = rng.range_i64(-1000, 1000) as c_int;
            let dv = rng.f64_interesting();
            let n = rng.below(8);
            let s = cs(&rng.utf8(n));
            let k = cs(&format!("k{}", rng.below(4)));
            for (tag, fmt) in [("i", "i"), ("s", "s"), ("f", "f")] {
                let f = cs(fmt);
                let j = match tag {
                    "i" => (api.json_pack)(f.as_ptr(), iv),
                    "s" => (api.json_pack)(f.as_ptr(), s.as_ptr()),
                    _ => (api.json_pack)(f.as_ptr(), dv),
                };
                rec.json(tag, j);
                rec_dump_all(api, rec, tag, j);
                decref(api, j);
            }
            let f = cs("{s:[i,s,f],s:n}");
            let j = (api.json_pack)(f.as_ptr(), k.as_ptr(), iv, s.as_ptr(), dv, k.as_ptr());
            rec.json("nested", j);
            rec_dump_all(api, rec, "nested", j);
            decref(api, j);
        }
    });
}

/* --------------------------------- rows 121..128: unpack ---------------- */

#[test]
fn cfg121to128_unpack() {
    diff("cfg121-128 unpack", |api, rec| unsafe {
        let mut rng = Rng::new(0x1210);
        for _ in 0..100 {
            let iv = rng.range_i64(i32::MIN as i64, i32::MAX as i64);
            let dv = rng.f64_interesting();
            let n = rng.below(10);
            let sv = rng.utf8(n);
            let mut doc = String::new();
            spec_to_text(
                &Spec::Obj(vec![
                    (b"i".to_vec(), Spec::Int(iv)),
                    (b"f".to_vec(), Spec::Real(dv)),
                    (b"s".to_vec(), Spec::Str(sv.clone())),
                    (b"b".to_vec(), Spec::True),
                    (b"b2".to_vec(), Spec::False),
                    (b"n".to_vec(), Spec::Null),
                    (
                        b"a".to_vec(),
                        Spec::Arr(vec![
                            Spec::Int(1),
                            Spec::Str("two".into()),
                            Spec::Real(3.5),
                            Spec::Null,
                        ]),
                    ),
                    (b"o".to_vec(), Spec::Obj(vec![(b"x".to_vec(), Spec::Int(9))])),
                ]),
                &mut doc,
            );
            let cdoc = cs(&doc);
            for (fname, flags) in UNPACK_FLAGS {
                let root = (api.json_loads)(cdoc.as_ptr(), 0, ptr::null_mut());
                assert!(!root.is_null());

                // row 121/122: every scalar format
                let mut si: c_int = -1;
                let mut sl: c_longlong = -1;
                let mut sf: f64 = -1.0;
                let mut s_f: f64 = -1.0;
                let mut sb: c_int = -1;
                let mut sb2: c_int = -1;
                let mut sstr: *const c_char = ptr::null();
                let mut slen: usize = 12345;
                let mut sobj: *mut Json = ptr::null_mut();
                let mut sarr: *mut Json = ptr::null_mut();
                let ki = cs("i");
                let kf = cs("f");
                let ks = cs("s");
                let kb = cs("b");
                let kb2 = cs("b2");
                let kn = cs("n");
                let ka = cs("a");
                let ko = cs("o");
                let fmt = cs("{s:i,s:f,s:F,s:s%,s:b,s:b,s:n,s:o,s:O}");
                let mut e = JsonError::patterned();
                let r = (api.json_unpack_ex)(
                    root,
                    &mut e,
                    *flags,
                    fmt.as_ptr(),
                    ki.as_ptr(),
                    &mut si,
                    kf.as_ptr(),
                    &mut sf,
                    kf.as_ptr(),
                    &mut s_f,
                    ks.as_ptr(),
                    &mut sstr,
                    &mut slen,
                    kb.as_ptr(),
                    &mut sb,
                    kb2.as_ptr(),
                    &mut sb2,
                    kn.as_ptr(),
                    ka.as_ptr(),
                    &mut sarr,
                    ko.as_ptr(),
                    &mut sobj,
                );
                rec.tag_i(&format!("{fname}.ret"), r as i64);
                rec.error(&format!("{fname}.err"), &e);
                rec.tag_i(&format!("{fname}.i"), si as i64);
                rec.tag_f(&format!("{fname}.f"), sf);
                rec.tag_f(&format!("{fname}.F"), s_f);
                rec.tag_i(&format!("{fname}.b"), sb as i64);
                rec.tag_i(&format!("{fname}.b2"), sb2 as i64);
                rec.cstring(&format!("{fname}.s"), sstr);
                rec.tag_u(&format!("{fname}.slen"), slen);
                rec.json(&format!("{fname}.arr"), sarr);
                rec.json(&format!("{fname}.obj"), sobj);
                if !sobj.is_null() {
                    // 'O' increffed it
                    decref(api, sobj);
                }

                // 'I' variant
                let mut e = JsonError::patterned();
                let fmt = cs("{s:I}");
                let r = (api.json_unpack_ex)(root, &mut e, *flags, fmt.as_ptr(), ki.as_ptr(), &mut sl);
                rec.tag_i(&format!("{fname}.Iret"), r as i64);
                rec.tag_i(&format!("{fname}.I"), sl as i64);
                rec.error(&format!("{fname}.Ierr"), &e);

                // row 123: optional keys, subset, missing
                for f in ["{s:i}", "{s?i}", "{s?i,s?i}", "{s:i,s:s%}", "{s:i!}", "{s:i*}"] {
                    let cf = cs(f);
                    let kmiss = cs("zzz");
                    let mut e = JsonError::patterned();
                    let mut a1: c_int = 0;
                    let mut a2: c_int = 0;
                    let mut st: *const c_char = ptr::null();
                    let mut sl2: usize = 0;
                    let r = match f {
                        "{s:i}" | "{s:i!}" | "{s:i*}" => (api.json_unpack_ex)(
                            root, &mut e, *flags, cf.as_ptr(), ki.as_ptr(), &mut a1,
                        ),
                        "{s?i}" => (api.json_unpack_ex)(
                            root, &mut e, *flags, cf.as_ptr(), kmiss.as_ptr(), &mut a1,
                        ),
                        "{s?i,s?i}" => (api.json_unpack_ex)(
                            root,
                            &mut e,
                            *flags,
                            cf.as_ptr(),
                            ki.as_ptr(),
                            &mut a1,
                            kmiss.as_ptr(),
                            &mut a2,
                        ),
                        _ => (api.json_unpack_ex)(
                            root,
                            &mut e,
                            *flags,
                            cf.as_ptr(),
                            ki.as_ptr(),
                            &mut a1,
                            ks.as_ptr(),
                            &mut st,
                            &mut sl2,
                        ),
                    };
                    rec.tag_i(&format!("{fname}.{f}.ret"), r as i64);
                    rec.tag_i(&format!("{fname}.{f}.a1"), a1 as i64);
                    rec.tag_i(&format!("{fname}.{f}.a2"), a2 as i64);
                    rec.cstring(&format!("{fname}.{f}.st"), st);
                    rec.tag_u(&format!("{fname}.{f}.sl"), sl2);
                    rec.error(&format!("{fname}.{f}.err"), &e);
                }

                // row 124/127: array formats
                let arr = (api.json_object_get)(root, ka.as_ptr());
                for f in ["[i]", "[i,s]", "[i,s,f]", "[i,s,f,n]", "[i,s,f,n,i]", "[i*]", "[i!]",
                          "[i,s,f,n!]", "[i,s,f,n*]", "[]", "[!]", "[*]"] {
                    let cf = cs(f);
                    let mut e = JsonError::patterned();
                    let mut a1: c_int = 0;
                    let mut a2: *const c_char = ptr::null();
                    let mut a3: f64 = 0.0;
                    let mut a4: c_int = 0;
                    let r = match f {
                        "[]" | "[!]" | "[*]" => {
                            (api.json_unpack_ex)(arr, &mut e, *flags, cf.as_ptr())
                        }
                        "[i]" | "[i*]" | "[i!]" => {
                            (api.json_unpack_ex)(arr, &mut e, *flags, cf.as_ptr(), &mut a1)
                        }
                        "[i,s]" => (api.json_unpack_ex)(
                            arr, &mut e, *flags, cf.as_ptr(), &mut a1, &mut a2,
                        ),
                        "[i,s,f]" => (api.json_unpack_ex)(
                            arr, &mut e, *flags, cf.as_ptr(), &mut a1, &mut a2, &mut a3,
                        ),
                        "[i,s,f,n,i]" => (api.json_unpack_ex)(
                            arr,
                            &mut e,
                            *flags,
                            cf.as_ptr(),
                            &mut a1,
                            &mut a2,
                            &mut a3,
                            &mut a4,
                        ),
                        _ => (api.json_unpack_ex)(
                            arr, &mut e, *flags, cf.as_ptr(), &mut a1, &mut a2, &mut a3,
                        ),
                    };
                    rec.tag_i(&format!("{fname}.arr{f}.ret"), r as i64);
                    rec.tag_i(&format!("{fname}.arr{f}.a1"), a1 as i64);
                    rec.cstring(&format!("{fname}.arr{f}.a2"), a2);
                    rec.tag_f(&format!("{fname}.arr{f}.a3"), a3);
                    rec.error(&format!("{fname}.arr{f}.err"), &e);
                }
                decref(api, root);
            }
        }
    });
}

#[test]
fn cfg121_json_unpack_plain_wrapper() {
    diff("cfg121 json_unpack", |api, rec| unsafe {
        let doc = cs(r#"{"a":1,"b":"two","c":[3,4.5]}"#);
        for _ in 0..50 {
            let root = (api.json_loads)(doc.as_ptr(), 0, ptr::null_mut());
            let ka = cs("a");
            let kb = cs("b");
            let f = cs("{s:i,s:s}");
            let mut i: c_int = 0;
            let mut s: *const c_char = ptr::null();
            let r = (api.json_unpack)(root, f.as_ptr(), ka.as_ptr(), &mut i, kb.as_ptr(), &mut s);
            rec.tag_i("ret", r as i64);
            rec.tag_i("i", i as i64);
            rec.cstring("s", s);
            // wrong type through the plain wrapper (no error struct)
            let f2 = cs("{s:s}");
            let r2 = (api.json_unpack)(root, f2.as_ptr(), ka.as_ptr(), &mut s);
            rec.tag_i("ret2", r2 as i64);
            decref(api, root);
        }
    });
}

#[test]
fn cfg125to128_strict_and_validate() {
    diff("cfg125-128 strict/validate", |api, rec| unsafe {
        let docs = [
            r#"{"a":1}"#,
            r#"{"a":1,"b":2}"#,
            r#"{"a":1,"b":2,"c":3}"#,
            r#"[1]"#,
            r#"[1,2]"#,
            r#"[1,2,3]"#,
            r#"{}"#,
            r#"[]"#,
        ];
        let objfmts = ["{s:i}", "{s:i!}", "{s:i*}", "{s?i}", "{s?i!}", "{s:i,s:i}",
                       "{s:i,s:i!}", "{s?i,s?i!}"];
        let arrfmts = ["[i]", "[i!]", "[i*]", "[i,i]", "[i,i!]", "[i,i,i]", "[i,i,i!]"];
        for d in docs {
            let cd = cs(d);
            for flags in [0usize, JSON_STRICT] {
                let root = (api.json_loads)(cd.as_ptr(), 0, ptr::null_mut());
                if root.is_null() {
                    rec.line("root=NULL");
                    continue;
                }
                for f in objfmts {
                    let cf = cs(f);
                    let ka = cs("a");
                    let kb = cs("b");
                    let mut i1: c_int = -5;
                    let mut i2: c_int = -6;
                    let mut e = JsonError::patterned();
                    let nkeys = f.matches('s').count();
                    let r = if nkeys == 1 {
                        (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr(), ka.as_ptr(), &mut i1)
                    } else {
                        (api.json_unpack_ex)(
                            root,
                            &mut e,
                            flags,
                            cf.as_ptr(),
                            ka.as_ptr(),
                            &mut i1,
                            kb.as_ptr(),
                            &mut i2,
                        )
                    };
                    rec.tag_i(&format!("{d}.{flags}.{f}.ret"), r as i64);
                    rec.tag_i(&format!("{d}.{flags}.{f}.i1"), i1 as i64);
                    rec.tag_i(&format!("{d}.{flags}.{f}.i2"), i2 as i64);
                    rec.error(&format!("{d}.{flags}.{f}.err"), &e);
                }
                for f in arrfmts {
                    let cf = cs(f);
                    let mut i1: c_int = -5;
                    let mut i2: c_int = -6;
                    let mut i3: c_int = -7;
                    let mut e = JsonError::patterned();
                    let n = f.matches('i').count();
                    let r = match n {
                        1 => (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr(), &mut i1),
                        2 => (api.json_unpack_ex)(
                            root, &mut e, flags, cf.as_ptr(), &mut i1, &mut i2,
                        ),
                        _ => (api.json_unpack_ex)(
                            root, &mut e, flags, cf.as_ptr(), &mut i1, &mut i2, &mut i3,
                        ),
                    };
                    rec.tag_i(&format!("{d}.{flags}.a{f}.ret"), r as i64);
                    rec.tag_i(&format!("{d}.{flags}.a{f}.i1"), i1 as i64);
                    rec.tag_i(&format!("{d}.{flags}.a{f}.i2"), i2 as i64);
                    rec.tag_i(&format!("{d}.{flags}.a{f}.i3"), i3 as i64);
                    rec.error(&format!("{d}.{flags}.a{f}.err"), &e);
                }
                decref(api, root);
            }
        }
    });
}

/* ------------------------------------------- row 129: pack/unpack round trip */

#[test]
fn cfg129_pack_unpack_roundtrip() {
    diff("cfg129 pack/unpack round trip", |api, rec| unsafe {
        let mut rng = Rng::new(0x1290);
        for _ in 0..200 {
            let spec = rand_container(&mut rng, 3);
            let src = build(api, &spec);
            // pack it into a wrapper object using 'O'
            let f = cs("{s:O,s:[O,O]}");
            let k1 = cs("one");
            let k2 = cs("two");
            let mut e = JsonError::patterned();
            let packed = (api.json_pack_ex)(
                &mut e,
                0,
                f.as_ptr(),
                k1.as_ptr(),
                src,
                k2.as_ptr(),
                src,
                src,
            );
            rec.json("packed", packed);
            rec.error("pack_err", &e);
            rec_dump_all(api, rec, "packed", packed);

            // unpack it back
            let uf = cs("{s:o,s:[o,o]}");
            let mut a: *mut Json = ptr::null_mut();
            let mut b: *mut Json = ptr::null_mut();
            let mut c: *mut Json = ptr::null_mut();
            let mut e2 = JsonError::patterned();
            let r = (api.json_unpack_ex)(
                packed,
                &mut e2,
                0,
                uf.as_ptr(),
                k1.as_ptr(),
                &mut a,
                k2.as_ptr(),
                &mut b,
                &mut c,
            );
            rec.tag_i("unpack_ret", r as i64);
            rec.error("unpack_err", &e2);
            rec.tag_i("a_eq_src", (api.json_equal)(a, src) as i64);
            rec.tag_i("b_eq_src", (api.json_equal)(b, src) as i64);
            rec.tag_i("c_eq_src", (api.json_equal)(c, src) as i64);
            decref(api, packed);
            decref(api, src);
        }
    });
}

/* ------------------- row 126: JSON_VALIDATE_ONLY (keys-only vararg lists) */

#[test]
fn cfg126_validate_only() {
    // With JSON_VALIDATE_ONLY, `unpack` consumes *no* value arguments — only
    // object keys — so the vararg list contains keys and nothing else.
    diff("cfg126 JSON_VALIDATE_ONLY", |api, rec| unsafe {
        let docs = [
            r#"{"a":1,"b":"s","c":1.5,"d":true,"e":null,"f":[1,2],"g":{"h":3}}"#,
            r#"{"a":"not an int"}"#,
            r#"[1,"s",1.5,true,null,[1],{"x":1}]"#,
            r#"[]"#,
            r#"{}"#,
        ];
        let ka = cs("a");
        let kb = cs("b");
        let kc = cs("c");
        let kd = cs("d");
        let ke = cs("e");
        let kf = cs("f");
        let kg = cs("g");
        let kz = cs("zzz");
        for d in docs {
            let cd = cs(d);
            for flags in [
                JSON_VALIDATE_ONLY,
                JSON_VALIDATE_ONLY | JSON_STRICT,
                usize::MAX,
            ] {
                let root = (api.json_loads)(cd.as_ptr(), 0, ptr::null_mut());
                if root.is_null() {
                    rec.line("root=NULL");
                    continue;
                }
                // object formats: only the keys are passed
                for f in [
                    "{s:i}", "{s:s}", "{s:f}", "{s:F}", "{s:b}", "{s:n}", "{s:o}", "{s:O}",
                    "{s:i,s:s,s:f,s:b,s:n,s:o,s:O}", "{s?i}", "{s:i!}", "{s:i*}", "{s:s%}",
                ] {
                    let cf = cs(f);
                    let mut e = JsonError::patterned();
                    let nkeys = f.matches("s:").count() + f.matches("s?").count();
                    let r = match nkeys {
                        0 => (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr()),
                        1 => {
                            let k = if f == "{s?i}" { kz.as_ptr() } else { ka.as_ptr() };
                            (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr(), k)
                        }
                        _ => (api.json_unpack_ex)(
                            root,
                            &mut e,
                            flags,
                            cf.as_ptr(),
                            ka.as_ptr(),
                            kb.as_ptr(),
                            kc.as_ptr(),
                            kd.as_ptr(),
                            ke.as_ptr(),
                            kf.as_ptr(),
                            kg.as_ptr(),
                        ),
                    };
                    rec.tag_i(&format!("{d}.{flags}.{f}.ret"), r as i64);
                    rec.error(&format!("{d}.{flags}.{f}.err"), &e);
                }
                // array formats need no varargs at all
                for f in [
                    "[i]", "[i,s]", "[i,s,f,b,n,o,O]", "[i!]", "[i*]", "[]", "[i,i,i,i,i,i,i,i]",
                ] {
                    let cf = cs(f);
                    let mut e = JsonError::patterned();
                    let r = (api.json_unpack_ex)(root, &mut e, flags, cf.as_ptr());
                    rec.tag_i(&format!("{d}.{flags}.a{f}.ret"), r as i64);
                    rec.error(&format!("{d}.{flags}.a{f}.err"), &e);
                }
                decref(api, root);
            }
        }
    });
}
