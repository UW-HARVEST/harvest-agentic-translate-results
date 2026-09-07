//! Phase B — CONFIGS.md rows 53..56 and 59: Duplicate, Compare, Minify.

mod common;
use common::*;
use std::ffi::{c_char, c_int};

fn typed_samples() -> Vec<Node> {
    vec![
        Node::Null,
        Node::True,
        Node::False,
        Node::Number(1.5),
        Node::Str("dup\"me".into()),
        Node::Raw("[1]".into()),
        Node::Array(vec![Node::Number(1.0), Node::Array(vec![Node::Null])]),
        Node::Object(vec![
            ("a".into(), Node::Number(1.0)),
            ("b".into(), Node::Object(vec![("c".into(), Node::True)])),
        ]),
        Node::StrRef("reference".into()),
        Node::ObjectCS(vec![("cs".into(), Node::Number(2.0))]),
        Node::Retyped(
            Box::new(Node::Str("x".into())),
            cJSON_String | cJSON_IsReference,
        ),
        Node::Retyped(
            Box::new(Node::Str("y".into())),
            cJSON_String | cJSON_StringIsConst,
        ),
        Node::Array((0..100).map(|i| Node::Number(i as f64)).collect()),
        Node::Array((0..5000).map(|i| Node::Number(i as f64)).collect()),
    ]
}

/// Rows 53 & 54
#[test]
fn row53_54_duplicate() {
    unsafe {
        let (c, r) = both();
        let mut nodes = typed_samples();
        let mut rng = Rng::new(53);
        for _ in 0..200 {
            nodes.push(random_node(&mut rng, 4));
        }
        for (i, n) in nodes.iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            for recurse in [0i32, 1, 2, -1] {
                let dc = (c.cJSON_Duplicate)(ci, recurse);
                let dr = (r.cJSON_Duplicate)(ri, recurse);
                assert_eq!(
                    dc.is_null(),
                    dr.is_null(),
                    "row53 #{} recurse={} NULL-ness",
                    i,
                    recurse
                );
                if !dc.is_null() {
                    assert_eq!((*dc).type_, (*dr).type_, "row53 #{} type", i);
                    assert_eq!(
                        read_cstr((*dc).valuestring),
                        read_cstr((*dr).valuestring),
                        "row53 #{} valuestring",
                        i
                    );
                    assert_eq!(
                        read_cstr((*dc).string),
                        read_cstr((*dr).string),
                        "row53 #{} string",
                        i
                    );
                    assert_eq!(
                        (*dc).next.is_null(),
                        (*dr).next.is_null(),
                        "row53 #{} next cleared",
                        i
                    );
                    assert_eq!(
                        (*dc).prev.is_null(),
                        (*dr).prev.is_null(),
                        "row53 #{} prev cleared",
                        i
                    );
                    assert_eq!(
                        fingerprint(&c, dc),
                        fingerprint(&r, dr),
                        "row53 #{} recurse={} structure",
                        i,
                        recurse
                    );
                    assert_eq!(link_shape(dc), link_shape(dr), "row53 #{} links", i);
                    assert_eq!(
                        show(&print_and_free(&c, dc)),
                        show(&print_and_free(&r, dr)),
                        "row53 #{} recurse={} print",
                        i,
                        recurse
                    );
                    for cs_flag in [0i32, 1] {
                        assert_eq!(
                            (c.cJSON_Compare)(ci, dc, cs_flag),
                            (r.cJSON_Compare)(ri, dr, cs_flag),
                            "row53 #{} Compare original/duplicate cs={}",
                            i,
                            cs_flag
                        );
                    }
                    (c.cJSON_Delete)(dc);
                    (r.cJSON_Delete)(dr);
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

// NOTE: `cJSON_Duplicate_rec` is compiled with hidden visibility in the C .so
// (CMake passes -fvisibility=hidden and the function has no CJSON_PUBLIC), so it
// cannot be dlsym'd from the C side and no differential test is possible for it.
// It is reached indirectly through `cJSON_Duplicate` above.

/// Rows 55 & 56
#[test]
fn row55_56_compare() {
    unsafe {
        let (c, r) = both();
        // hand-built pairs that stress the interesting branches
        let pairs: Vec<(Node, Node)> = vec![
            (Node::Null, Node::Null),
            (Node::True, Node::True),
            (Node::True, Node::False),
            (Node::Number(1.0), Node::Number(1.0)),
            (Node::Number(1.0), Node::Number(1.0 + f64::EPSILON)),
            (Node::Number(1.0), Node::Number(1.0 + 4.0 * f64::EPSILON)),
            (Node::Number(1e300), Node::Number(1e300 * (1.0 + 1e-16))),
            (Node::Number(f64::INFINITY), Node::Number(f64::INFINITY)),
            (
                Node::Number(f64::INFINITY),
                Node::Number(f64::NEG_INFINITY),
            ),
            (Node::Number(f64::NAN), Node::Number(f64::NAN)),
            (Node::Number(0.0), Node::Number(-0.0)),
            (Node::Str("a".into()), Node::Str("a".into())),
            (Node::Str("a".into()), Node::Str("A".into())),
            (Node::Raw("1".into()), Node::Raw("1".into())),
            (Node::Raw("1".into()), Node::Raw("2".into())),
            (
                Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]),
                Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]),
            ),
            (
                Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]),
                Node::Array(vec![Node::Number(2.0), Node::Number(1.0)]),
            ),
            (
                Node::Array(vec![Node::Number(1.0)]),
                Node::Array(vec![Node::Number(1.0), Node::Number(2.0)]),
            ),
            (
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
            ),
            (
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
                Node::Object(vec![("A".into(), Node::Number(1.0))]),
            ),
            (
                Node::Object(vec![
                    ("a".into(), Node::Number(1.0)),
                    ("b".into(), Node::Number(2.0)),
                ]),
                Node::Object(vec![
                    ("b".into(), Node::Number(2.0)),
                    ("a".into(), Node::Number(1.0)),
                ]),
            ),
            (
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
                Node::Object(vec![
                    ("a".into(), Node::Number(1.0)),
                    ("b".into(), Node::Number(2.0)),
                ]),
            ),
            (
                Node::Object(vec![
                    ("a".into(), Node::Number(1.0)),
                    ("b".into(), Node::Number(2.0)),
                ]),
                Node::Object(vec![("a".into(), Node::Number(1.0))]),
            ),
            (Node::Retyped(Box::new(Node::Null), 0), Node::Retyped(Box::new(Node::Null), 0)),
            (
                Node::Retyped(Box::new(Node::Number(1.0)), 9),
                Node::Retyped(Box::new(Node::Number(1.0)), 9),
            ),
            (
                Node::Retyped(Box::new(Node::Number(1.0)), 0xFF),
                Node::Retyped(Box::new(Node::Number(1.0)), 0xFF),
            ),
            (
                Node::Retyped(Box::new(Node::Str("s".into())), cJSON_String | 0x10000),
                Node::Str("s".into()),
            ),
            (Node::StrRef("q".into()), Node::Str("q".into())),
        ];
        for (i, (na, nb)) in pairs.iter().enumerate() {
            let ca = build(&c, na);
            let cb = build(&c, nb);
            let ra = build(&r, na);
            let rb = build(&r, nb);
            for cs_flag in [0i32, 1, 2, -1] {
                assert_eq!(
                    (c.cJSON_Compare)(ca, cb, cs_flag),
                    (r.cJSON_Compare)(ra, rb, cs_flag),
                    "row55 pair #{} cs={}",
                    i,
                    cs_flag
                );
                assert_eq!(
                    (c.cJSON_Compare)(cb, ca, cs_flag),
                    (r.cJSON_Compare)(rb, ra, cs_flag),
                    "row55 pair #{} reversed cs={}",
                    i,
                    cs_flag
                );
                // identity
                assert_eq!(
                    (c.cJSON_Compare)(ca, ca, cs_flag),
                    (r.cJSON_Compare)(ra, ra, cs_flag),
                    "row55 pair #{} identity cs={}",
                    i,
                    cs_flag
                );
            }
            (c.cJSON_Delete)(ca);
            (c.cJSON_Delete)(cb);
            (r.cJSON_Delete)(ra);
            (r.cJSON_Delete)(rb);
        }

        // randomized cross product
        let mut rng = Rng::new(55);
        let nodes: Vec<Node> = (0..40).map(|_| random_node(&mut rng, 3)).collect();
        for (i, na) in nodes.iter().enumerate() {
            for (j, nb) in nodes.iter().enumerate() {
                let ca = build(&c, na);
                let cb = build(&c, nb);
                let ra = build(&r, na);
                let rb = build(&r, nb);
                for cs_flag in [0i32, 1] {
                    assert_eq!(
                        (c.cJSON_Compare)(ca, cb, cs_flag),
                        (r.cJSON_Compare)(ra, rb, cs_flag),
                        "row55 random {}x{} cs={}",
                        i,
                        j,
                        cs_flag
                    );
                }
                (c.cJSON_Delete)(ca);
                (c.cJSON_Delete)(cb);
                (r.cJSON_Delete)(ra);
                (r.cJSON_Delete)(rb);
            }
        }
    }
}

/// Row 56b — numbers just inside/outside the `compare_double` tolerance.
#[test]
fn row56b_compare_double_tolerance() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(56);
        let mut vals: Vec<f64> = vec![0.0, 1.0, -1.0, 1e-300, 1e300, 12345.6789];
        for _ in 0..300 {
            vals.push(rng.f64());
        }
        for a in vals {
            let variants = [
                a,
                f64::from_bits(a.to_bits().wrapping_add(1)),
                f64::from_bits(a.to_bits().wrapping_add(2)),
                a * (1.0 + f64::EPSILON),
                a * (1.0 + 2.0 * f64::EPSILON),
                a * (1.0 + 8.0 * f64::EPSILON),
                a + f64::EPSILON,
                -a,
            ];
            for b in variants {
                let ca = (c.cJSON_CreateNumber)(a);
                let cb = (c.cJSON_CreateNumber)(b);
                let ra = (r.cJSON_CreateNumber)(a);
                let rb = (r.cJSON_CreateNumber)(b);
                assert_eq!(
                    (c.cJSON_Compare)(ca, cb, 1),
                    (r.cJSON_Compare)(ra, rb, 1),
                    "row56 Compare({:?}, {:?})",
                    a,
                    b
                );
                (c.cJSON_Delete)(ca);
                (c.cJSON_Delete)(cb);
                (r.cJSON_Delete)(ra);
                (r.cJSON_Delete)(rb);
            }
        }
    }
}

/// Row 59
#[test]
fn row59_minify() {
    unsafe {
        let (c, r) = both();
        let mut inputs: Vec<Vec<u8>> = vec![
            b"".to_vec(),
            b" ".to_vec(),
            b"\t\r\n ".to_vec(),
            b"{ }".to_vec(),
            b"{ \"a\" : 1 }".to_vec(),
            b"[ 1 , 2 , 3 ]".to_vec(),
            b"// comment\n{}".to_vec(),
            b"{} // trailing".to_vec(),
            b"/* block */{}".to_vec(),
            b"{/* inner */}".to_vec(),
            b"{\"a\":\"// not a comment\"}".to_vec(),
            b"{\"a\":\"/* not a comment */\"}".to_vec(),
            b"{\"a\":\"has \\\" quote\"}".to_vec(),
            b"{\"a\":\"has \\\\ backslash\"}".to_vec(),
            b"/* unterminated".to_vec(),
            b"\"unterminated".to_vec(),
            b"//".to_vec(),
            b"/".to_vec(),
            b"/x".to_vec(),
            b"\"a\\".to_vec(),
            b"{\"a\":1}/*".to_vec(),
            b"  \n\t{\n\t\"k\"\t:\t[\n1,\n2\n]\n}\n".to_vec(),
            b"{\"s\":\"a\\nb\"}".to_vec(),
        ];
        let mut rng = Rng::new(59);
        for _ in 0..200 {
            let n = random_node(&mut rng, 3);
            let ci = build(&c, &n);
            if let Some(t) = print_and_free(&c, ci) {
                inputs.push(t);
            }
            (c.cJSON_Delete)(ci);
        }
        for text in &inputs {
            let mut a = cbytes(text);
            let mut b = cbytes(text);
            (c.cJSON_Minify)(a.as_mut_ptr() as *mut c_char);
            (r.cJSON_Minify)(b.as_mut_ptr() as *mut c_char);
            assert_eq!(
                a,
                b,
                "row59 Minify {:?}\n C={:?}\n R={:?}",
                String::from_utf8_lossy(text),
                String::from_utf8_lossy(&a),
                String::from_utf8_lossy(&b)
            );
        }
        let _: c_int = 0;
    }
}
