//! Phase B — CONFIGS.md rows 1..12: constructors.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};

/// Compare a freshly created item field-by-field plus both print forms.
unsafe fn cmp_item(label: &str, c: &Api, r: &Api, ci: Item, ri: Item) {
    assert_eq!(
        ci.is_null(),
        ri.is_null(),
        "{}: NULL-ness differs (C null={} RUST null={})",
        label,
        ci.is_null(),
        ri.is_null()
    );
    if ci.is_null() {
        return;
    }
    let a = &*ci;
    let b = &*ri;
    assert_eq!(a.type_, b.type_, "{}: type", label);
    assert_eq!(a.valueint, b.valueint, "{}: valueint", label);
    assert_eq!(
        a.valuedouble.to_bits(),
        b.valuedouble.to_bits(),
        "{}: valuedouble ({} vs {})",
        label,
        a.valuedouble,
        b.valuedouble
    );
    assert_eq!(
        read_cstr(a.valuestring),
        read_cstr(b.valuestring),
        "{}: valuestring",
        label
    );
    assert_eq!(read_cstr(a.string), read_cstr(b.string), "{}: string", label);
    assert_eq!(
        fingerprint(c, ci),
        fingerprint(r, ri),
        "{}: structure",
        label
    );
    assert_eq!(link_shape(ci), link_shape(ri), "{}: link bookkeeping", label);
    assert_eq!(
        show(&print_and_free(c, ci)),
        show(&print_and_free(r, ri)),
        "{}: cJSON_Print",
        label
    );
    assert_eq!(
        show(&print_unformatted_and_free(c, ci)),
        show(&print_unformatted_and_free(r, ri)),
        "{}: cJSON_PrintUnformatted",
        label
    );
}

unsafe fn build_cmp(label: &str, c: &Api, r: &Api, node: &Node) {
    let ci = build(c, node);
    let ri = build(r, node);
    cmp_item(label, c, r, ci, ri);
    (c.cJSON_Delete)(ci);
    (r.cJSON_Delete)(ri);
}

/// Row 1
#[test]
fn row01_version() {
    unsafe {
        let (c, r) = both();
        let a = read_cstr((c.cJSON_Version)());
        let b = read_cstr((r.cJSON_Version)());
        assert_eq!(a, b, "cJSON_Version");
        assert_eq!(a.as_deref(), Some(&b"1.7.19"[..]));
    }
}

/// Row 2
#[test]
fn row02_create_simple() {
    unsafe {
        let (c, r) = both();
        macro_rules! t {
            ($f:ident) => {{
                let ci = (c.$f)();
                let ri = (r.$f)();
                cmp_item(stringify!($f), &c, &r, ci, ri);
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }};
        }
        t!(cJSON_CreateNull);
        t!(cJSON_CreateTrue);
        t!(cJSON_CreateFalse);
        t!(cJSON_CreateArray);
        t!(cJSON_CreateObject);
    }
}

/// Row 3
#[test]
fn row03_create_bool() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(0x1234);
        let mut vals: Vec<c_int> = vec![0, 1, 2, -1, c_int::MAX, c_int::MIN, 256, 512];
        for _ in 0..200 {
            vals.push(rng.i32());
        }
        for v in vals {
            build_cmp(&format!("CreateBool({})", v), &c, &r, &Node::Bool(v));
        }
    }
}

/// Row 4
#[test]
fn row04_create_number() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(0xF00D);
        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            1.0,
            -1.0,
            0.5,
            1.0 / 3.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            -f64::NAN,
            f64::MIN,
            f64::MAX,
            f64::MIN_POSITIVE,
            5e-324,
            2147483647.0,
            2147483648.0,
            2147483646.5,
            -2147483648.0,
            -2147483649.0,
            1e308,
            1e-308,
            1e17,
            1e16,
            123456789012345.6,
            0.1 + 0.2,
            f64::EPSILON,
            f64::EPSILON / 2.0,
            -f64::EPSILON / 2.0,
        ];
        for _ in 0..2000 {
            vals.push(rng.f64());
        }
        for v in vals {
            build_cmp(&format!("CreateNumber({:?})", v), &c, &r, &Node::Number(v));
        }
    }
}

/// Row 5
#[test]
fn row05_create_string() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(0xBEEF);
        let mut vals: Vec<String> = vec![
            String::new(),
            "a".into(),
            "hello world".into(),
            "\"quoted\"".into(),
            "back\\slash".into(),
            "\u{1}\u{2}\u{1f}".into(),
            "tab\there".into(),
            "nl\nhere".into(),
            "cr\rhere".into(),
            "\u{8}\u{c}".into(),
            "é€😀".into(),
            "slash/es".into(),
            "\u{7f}".into(),
        ];
        for _ in 0..400 {
            vals.push(rng.string(64));
        }
        vals.push((0..1024).map(|i| ((i % 94) as u8 + 33) as char).collect());
        for v in &vals {
            build_cmp("CreateString", &c, &r, &Node::Str(v.clone()));
        }
    }
}

/// Row 6
#[test]
fn row06_create_raw() {
    unsafe {
        let (c, r) = both();
        for v in [
            "", "123", "{\"a\":1}", "[1,2,3]", "not json at all", "\"str\"", "null",
        ] {
            build_cmp("CreateRaw", &c, &r, &Node::Raw(v.to_string()));
        }
    }
}

/// Row 7
#[test]
fn row07_create_string_reference() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(7);
        for _ in 0..200 {
            let s = rng.string(20);
            build_cmp("CreateStringReference", &c, &r, &Node::StrRef(s));
        }
    }
}

/// Row 8 — object/array references must print their child and survive delete.
#[test]
fn row08_create_container_references() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(8);
        for _ in 0..60 {
            let inner = random_node(&mut rng, 2);
            for as_object in [false, true] {
                let cchild = build(&c, &inner);
                let rchild = build(&r, &inner);
                let cref = if as_object {
                    (c.cJSON_CreateObjectReference)(cchild)
                } else {
                    (c.cJSON_CreateArrayReference)(cchild)
                };
                let rref = if as_object {
                    (r.cJSON_CreateObjectReference)(rchild)
                } else {
                    (r.cJSON_CreateArrayReference)(rchild)
                };
                cmp_item("container reference", &c, &r, cref, rref);
                // Deleting the reference must NOT free the child.
                (c.cJSON_Delete)(cref);
                (r.cJSON_Delete)(rref);
                assert_eq!(
                    show(&print_and_free(&c, cchild)),
                    show(&print_and_free(&r, rchild)),
                    "child still printable after reference delete"
                );
                (c.cJSON_Delete)(cchild);
                (r.cJSON_Delete)(rchild);
            }
        }
    }
}

/// Row 9
#[test]
fn row09_create_int_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(9);
        for count in [0usize, 1, 2, 3, 17, 64] {
            for _ in 0..40 {
                let mut v: Vec<c_int> = (0..count).map(|_| rng.i32()).collect();
                if count > 0 {
                    v[0] = c_int::MAX;
                    v[count - 1] = c_int::MIN;
                }
                build_cmp(
                    &format!("CreateIntArray(count={})", count),
                    &c,
                    &r,
                    &Node::IntArray(v),
                );
            }
        }
    }
}

/// Row 10
#[test]
fn row10_create_float_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(10);
        for count in [0usize, 1, 5, 33] {
            for _ in 0..40 {
                let mut v: Vec<f32> = (0..count).map(|_| rng.f32()).collect();
                if count >= 4 {
                    v[0] = f32::INFINITY;
                    v[1] = f32::NEG_INFINITY;
                    v[2] = f32::NAN;
                    v[3] = f32::MIN_POSITIVE / 2.0;
                }
                build_cmp(
                    &format!("CreateFloatArray(count={})", count),
                    &c,
                    &r,
                    &Node::FloatArray(v),
                );
            }
        }
    }
}

/// Row 11
#[test]
fn row11_create_double_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(11);
        for count in [0usize, 1, 5, 33] {
            for _ in 0..40 {
                let mut v: Vec<f64> = (0..count).map(|_| rng.f64()).collect();
                if count >= 4 {
                    v[0] = f64::INFINITY;
                    v[1] = f64::NEG_INFINITY;
                    v[2] = f64::NAN;
                    v[3] = 5e-324;
                }
                build_cmp(
                    &format!("CreateDoubleArray(count={})", count),
                    &c,
                    &r,
                    &Node::DoubleArray(v),
                );
            }
        }
    }
}

/// Row 12
#[test]
fn row12_create_string_array() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(12);
        for count in [0usize, 1, 7, 20] {
            for _ in 0..40 {
                let mut v: Vec<String> = (0..count).map(|_| rng.string(10)).collect();
                if count > 0 {
                    v[0] = String::new();
                }
                build_cmp(
                    &format!("CreateStringArray(count={})", count),
                    &c,
                    &r,
                    &Node::StringArray(v),
                );
            }
        }
    }
}

/// Row 60 — cJSON_malloc / cJSON_free with the default hooks.
#[test]
fn row60_malloc_free() {
    unsafe {
        let (c, r) = both();
        for size in [0usize, 1, 7, 4096, 1 << 20] {
            let pc = (c.cJSON_malloc)(size);
            let pr = (r.cJSON_malloc)(size);
            assert_eq!(
                pc.is_null(),
                pr.is_null(),
                "cJSON_malloc({}) NULL-ness differs",
                size
            );
            if !pc.is_null() {
                // writable
                std::ptr::write_bytes(pc as *mut u8, 0xAB, size);
                std::ptr::write_bytes(pr as *mut u8, 0xAB, size);
            }
            (c.cJSON_free)(pc);
            (r.cJSON_free)(pr);
        }
        (c.cJSON_free)(std::ptr::null_mut::<c_void>());
        (r.cJSON_free)(std::ptr::null_mut::<c_void>());
    }
}

/// Row 57 — cJSON_SetNumberHelper.
#[test]
fn row57_set_number_helper() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(57);
        let mut vals: Vec<f64> = vec![
            0.0,
            -0.0,
            2147483647.0,
            2147483646.9,
            2147483648.0,
            -2147483648.0,
            -2147483648.5,
            -2147483649.0,
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            1e300,
            -1e300,
        ];
        for _ in 0..1000 {
            vals.push(rng.f64());
        }
        for v in vals {
            let ci = (c.cJSON_CreateNumber)(0.0);
            let ri = (r.cJSON_CreateNumber)(0.0);
            let rc = (c.cJSON_SetNumberHelper)(ci, v);
            let rr = (r.cJSON_SetNumberHelper)(ri, v);
            assert_eq!(rc.to_bits(), rr.to_bits(), "SetNumberHelper({:?}) ret", v);
            cmp_item(&format!("SetNumberHelper({:?})", v), &c, &r, ci, ri);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 58 — cJSON_SetValuestring: shorter / equal / longer replacement.
#[test]
fn row58_set_valuestring() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(58);
        let cases: Vec<(String, String)> = {
            let mut v = vec![
                ("hello".to_string(), "hi".to_string()),      // shorter
                ("hello".to_string(), "world".to_string()),   // equal
                ("hi".to_string(), "hello!!".to_string()),    // longer
                ("".to_string(), "".to_string()),             // empty/empty
                ("".to_string(), "x".to_string()),            // grow from empty
                ("abc".to_string(), "".to_string()),          // shrink to empty
            ];
            for _ in 0..300 {
                v.push((rng.string(12), rng.string(12)));
            }
            v
        };
        for (old, new) in cases {
            for kind in 0..2 {
                let node = if kind == 0 {
                    Node::Str(old.clone())
                } else {
                    Node::StrRef(old.clone())
                };
                let ci = build(&c, &node);
                let ri = build(&r, &node);
                let nc = cbytes(new.as_bytes());
                let pc = (c.cJSON_SetValuestring)(ci, nc.as_ptr() as *const c_char);
                let pr = (r.cJSON_SetValuestring)(ri, nc.as_ptr() as *const c_char);
                assert_eq!(
                    pc.is_null(),
                    pr.is_null(),
                    "SetValuestring({:?}->{:?}, kind={}) NULL-ness",
                    old,
                    new,
                    kind
                );
                assert_eq!(
                    read_cstr(pc),
                    read_cstr(pr),
                    "SetValuestring returned string"
                );
                cmp_item("after SetValuestring", &c, &r, ci, ri);
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
    }
}
