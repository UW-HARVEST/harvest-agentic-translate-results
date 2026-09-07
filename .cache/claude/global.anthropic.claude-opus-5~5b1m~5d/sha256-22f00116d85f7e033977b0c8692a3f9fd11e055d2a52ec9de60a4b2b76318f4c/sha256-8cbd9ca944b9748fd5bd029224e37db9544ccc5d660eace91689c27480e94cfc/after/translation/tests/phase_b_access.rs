//! Phase B — CONFIGS.md rows 29..35: accessors and predicates.

mod common;
use common::*;
use std::ffi::{c_char, c_int};

fn sized_array(n: usize) -> Node {
    Node::Array((0..n).map(|i| Node::Number(i as f64)).collect())
}

fn sample_object() -> Node {
    Node::Object(vec![
        ("alpha".into(), Node::Number(1.0)),
        ("Beta".into(), Node::Str("b".into())),
        ("GAMMA".into(), Node::True),
        ("delta".into(), Node::Array(vec![Node::Null])),
        ("dup".into(), Node::Number(1.0)),
        ("dup".into(), Node::Number(2.0)),
        ("DUP".into(), Node::Number(3.0)),
        ("".into(), Node::False),
    ])
}

/// Row 29
#[test]
fn row29_get_array_size() {
    unsafe {
        let (c, r) = both();
        let mut nodes: Vec<Node> = vec![
            sized_array(0),
            sized_array(1),
            sized_array(3),
            sized_array(50),
            sample_object(),
            Node::Number(1.0),
            Node::Str("x".into()),
            Node::Null,
            Node::Raw("[1,2]".into()),
        ];
        let mut rng = Rng::new(29);
        for _ in 0..200 {
            nodes.push(random_node(&mut rng, 3));
        }
        for n in &nodes {
            let ci = build(&c, n);
            let ri = build(&r, n);
            assert_eq!(
                (c.cJSON_GetArraySize)(ci),
                (r.cJSON_GetArraySize)(ri),
                "row29 GetArraySize"
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 30
#[test]
fn row30_get_array_item() {
    unsafe {
        let (c, r) = both();
        for size in [0usize, 1, 5, 20] {
            let n = sized_array(size);
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            let mut idxs: Vec<c_int> = vec![-1, 0, 1, c_int::MIN, c_int::MAX];
            idxs.push(size as c_int);
            idxs.push(size as c_int + 1);
            if size > 0 {
                idxs.push(size as c_int - 1);
            }
            for idx in idxs {
                let a = (c.cJSON_GetArrayItem)(ci, idx);
                let b = (r.cJSON_GetArrayItem)(ri, idx);
                assert_eq!(
                    a.is_null(),
                    b.is_null(),
                    "row30 size={} idx={} NULL-ness",
                    size,
                    idx
                );
                if !a.is_null() {
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, a)),
                        show(&print_unformatted_and_free(&r, b)),
                        "row30 size={} idx={} value",
                        size,
                        idx
                    );
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 31, 32, 33
#[test]
fn row31_32_33_get_object_item() {
    unsafe {
        let (c, r) = both();
        let mut nodes = vec![sample_object(), Node::Object(vec![]), sized_array(3)];
        let mut rng = Rng::new(31);
        for _ in 0..80 {
            nodes.push(random_node(&mut rng, 3));
        }
        let keys: Vec<String> = vec![
            "alpha".into(),
            "ALPHA".into(),
            "Alpha".into(),
            "beta".into(),
            "Beta".into(),
            "gamma".into(),
            "GAMMA".into(),
            "delta".into(),
            "dup".into(),
            "DUP".into(),
            "missing".into(),
            "".into(),
            "a".into(),
        ];
        for n in &nodes {
            let ci = build(&c, n);
            let ri = build(&r, n);
            for k in &keys {
                let kb = cbytes(k.as_bytes());
                let kp = kb.as_ptr() as *const c_char;

                let a = (c.cJSON_GetObjectItem)(ci, kp);
                let b = (r.cJSON_GetObjectItem)(ri, kp);
                assert_eq!(a.is_null(), b.is_null(), "row31 key={:?} NULL-ness", k);
                if !a.is_null() {
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, a)),
                        show(&print_unformatted_and_free(&r, b)),
                        "row31 key={:?} value",
                        k
                    );
                    assert_eq!(
                        read_cstr((*a).string),
                        read_cstr((*b).string),
                        "row31 key={:?} matched key",
                        k
                    );
                }

                let a = (c.cJSON_GetObjectItemCaseSensitive)(ci, kp);
                let b = (r.cJSON_GetObjectItemCaseSensitive)(ri, kp);
                assert_eq!(a.is_null(), b.is_null(), "row32 key={:?} NULL-ness", k);
                if !a.is_null() {
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, a)),
                        show(&print_unformatted_and_free(&r, b)),
                        "row32 key={:?} value",
                        k
                    );
                }

                assert_eq!(
                    (c.cJSON_HasObjectItem)(ci, kp),
                    (r.cJSON_HasObjectItem)(ri, kp),
                    "row33 key={:?}",
                    k
                );
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

fn one_of_each_type() -> Vec<Node> {
    let mut v = vec![
        Node::Null,
        Node::True,
        Node::False,
        Node::Number(1.25),
        Node::Str("s".into()),
        Node::Raw("42".into()),
        Node::Array(vec![Node::Null]),
        Node::Object(vec![("k".into(), Node::Null)]),
        Node::StrRef("ref".into()),
        Node::Retyped(Box::new(Node::Null), 0),
        Node::Retyped(Box::new(Node::Str("x".into())), cJSON_String | cJSON_IsReference),
        Node::Retyped(Box::new(Node::Str("x".into())), cJSON_String | cJSON_StringIsConst),
        Node::Retyped(Box::new(Node::Number(1.0)), cJSON_Number | 0x10000),
        Node::Retyped(Box::new(Node::Number(1.0)), -1),
        Node::Retyped(Box::new(Node::Number(1.0)), 0x1FF),
        Node::Retyped(Box::new(Node::Number(1.0)), cJSON_False | cJSON_True),
    ];
    for t in [0i32, 1, 2, 3, 4, 8, 9, 16, 32, 64, 128, 255, 256, 511, 512] {
        v.push(Node::Retyped(Box::new(Node::Number(7.0)), t));
    }
    v
}

/// Row 34
#[test]
fn row34_get_string_number_value() {
    unsafe {
        let (c, r) = both();
        for (i, n) in one_of_each_type().iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            let sc = (c.cJSON_GetStringValue)(ci);
            let sr = (r.cJSON_GetStringValue)(ri);
            assert_eq!(sc.is_null(), sr.is_null(), "row34 #{} GetStringValue NULL", i);
            assert_eq!(read_cstr(sc), read_cstr(sr), "row34 #{} GetStringValue", i);
            let nc = (c.cJSON_GetNumberValue)(ci);
            let nr = (r.cJSON_GetNumberValue)(ri);
            assert_eq!(
                nc.is_nan(),
                nr.is_nan(),
                "row34 #{} GetNumberValue NaN-ness",
                i
            );
            if !nc.is_nan() {
                assert_eq!(
                    nc.to_bits(),
                    nr.to_bits(),
                    "row34 #{} GetNumberValue value",
                    i
                );
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 35
#[test]
fn row35_predicates() {
    unsafe {
        let (c, r) = both();
        for (i, n) in one_of_each_type().iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            macro_rules! p {
                ($f:ident) => {
                    assert_eq!(
                        (c.$f)(ci),
                        (r.$f)(ri),
                        "row35 #{} {}",
                        i,
                        stringify!($f)
                    );
                };
            }
            p!(cJSON_IsInvalid);
            p!(cJSON_IsFalse);
            p!(cJSON_IsTrue);
            p!(cJSON_IsBool);
            p!(cJSON_IsNull);
            p!(cJSON_IsNumber);
            p!(cJSON_IsString);
            p!(cJSON_IsArray);
            p!(cJSON_IsObject);
            p!(cJSON_IsRaw);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}
