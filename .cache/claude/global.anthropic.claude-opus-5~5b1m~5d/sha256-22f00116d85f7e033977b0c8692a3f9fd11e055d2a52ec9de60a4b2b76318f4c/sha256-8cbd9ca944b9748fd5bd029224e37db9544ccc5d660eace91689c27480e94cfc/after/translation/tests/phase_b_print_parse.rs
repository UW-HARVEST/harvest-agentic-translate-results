//! Phase B — CONFIGS.md rows 13..28: printing and parsing.

mod common;
use common::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

fn all_type_roots() -> Vec<Node> {
    vec![
        Node::Null,
        Node::True,
        Node::False,
        Node::Number(3.5),
        Node::Str("hi\n\"there\"".into()),
        Node::Raw("{\"raw\":1}".into()),
        Node::Array(vec![Node::Number(1.0), Node::Str("x".into())]),
        Node::Object(vec![
            ("a".into(), Node::Number(1.0)),
            ("b".into(), Node::Array(vec![])),
        ]),
        Node::Retyped(Box::new(Node::Null), 0),          // cJSON_Invalid
        Node::Retyped(Box::new(Node::Null), 0x4000),     // unknown high bits
        Node::Retyped(Box::new(Node::Str("x".into())), cJSON_String | cJSON_IsReference),
        Node::Array(vec![]),
        Node::Object(vec![]),
    ]
}

fn nested(depth: u32, empty_leaf: bool) -> Node {
    if depth == 0 {
        return if empty_leaf {
            Node::Array(vec![])
        } else {
            Node::Number(depth as f64)
        };
    }
    if depth % 2 == 0 {
        Node::Array(vec![nested(depth - 1, empty_leaf), Node::Array(vec![])])
    } else {
        Node::Object(vec![
            ("k".into(), nested(depth - 1, empty_leaf)),
            ("e".into(), Node::Object(vec![])),
        ])
    }
}

/// Rows 13 & 14
#[test]
fn row13_14_print_all_types() {
    unsafe {
        let (c, r) = both();
        for (i, n) in all_type_roots().iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "row13 Print root #{}",
                i
            );
            assert_eq!(
                show(&print_unformatted_and_free(&c, ci)),
                show(&print_unformatted_and_free(&r, ri)),
                "row14 PrintUnformatted root #{}",
                i
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Rows 15 & 16
#[test]
fn row15_16_print_nested() {
    unsafe {
        let (c, r) = both();
        for depth in [1u32, 2, 5, 50] {
            for empty_leaf in [false, true] {
                let n = nested(depth, empty_leaf);
                let ci = build(&c, &n);
                let ri = build(&r, &n);
                assert_eq!(
                    show(&print_and_free(&c, ci)),
                    show(&print_and_free(&r, ri)),
                    "row15 depth={} empty={}",
                    depth,
                    empty_leaf
                );
                assert_eq!(
                    show(&print_unformatted_and_free(&c, ci)),
                    show(&print_unformatted_and_free(&r, ri)),
                    "row16 depth={} empty={}",
                    depth,
                    empty_leaf
                );
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);
            }
        }
        // randomized trees
        let mut rng = Rng::new(1516);
        for i in 0..400 {
            let n = random_node(&mut rng, 4);
            let ci = build(&c, &n);
            let ri = build(&r, &n);
            assert_eq!(
                show(&print_and_free(&c, ci)),
                show(&print_and_free(&r, ri)),
                "row15 random #{}",
                i
            );
            assert_eq!(
                show(&print_unformatted_and_free(&c, ci)),
                show(&print_unformatted_and_free(&r, ri)),
                "row16 random #{}",
                i
            );
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 17 — cJSON_PrintBuffered across prebuffer sizes and fmt.
#[test]
fn row17_print_buffered() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(17);
        let mut trees: Vec<Node> = all_type_roots();
        trees.push(nested(30, false));
        trees.push(Node::Array(
            (0..200).map(|i| Node::Number(i as f64 * 1.5)).collect(),
        ));
        for _ in 0..80 {
            trees.push(random_node(&mut rng, 4));
        }
        for (i, n) in trees.iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            for prebuffer in [0i32, 1, 2, 15, 256, 65536] {
                for fmt in [0i32, 1] {
                    assert_eq!(
                        show(&print_buffered_and_free(&c, ci, prebuffer, fmt)),
                        show(&print_buffered_and_free(&r, ri, prebuffer, fmt)),
                        "row17 tree #{} prebuffer={} fmt={}",
                        i,
                        prebuffer,
                        fmt
                    );
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 18 — cJSON_PrintPreallocated with several buffer sizes (noalloc path).
#[test]
fn row18_print_preallocated() {
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(18);
        let mut trees: Vec<Node> = all_type_roots();
        for _ in 0..120 {
            trees.push(random_node(&mut rng, 4));
        }
        for (i, n) in trees.iter().enumerate() {
            let ci = build(&c, n);
            let ri = build(&r, n);
            for format in [0i32, 1] {
                let reference = if format == 1 {
                    print_and_free(&c, ci)
                } else {
                    print_unformatted_and_free(&c, ci)
                };
                let exact = reference.as_ref().map(|b| b.len() + 1).unwrap_or(1);
                for len in [exact as i32, exact as i32 + 5, exact as i32 + 1000] {
                    let mut cbuf = vec![0u8; (len.max(1)) as usize + 16];
                    let mut rbuf = vec![0u8; (len.max(1)) as usize + 16];
                    let okc = (c.cJSON_PrintPreallocated)(
                        ci,
                        cbuf.as_mut_ptr() as *mut c_char,
                        len,
                        format,
                    );
                    let okr = (r.cJSON_PrintPreallocated)(
                        ri,
                        rbuf.as_mut_ptr() as *mut c_char,
                        len,
                        format,
                    );
                    assert_eq!(
                        okc, okr,
                        "row18 tree #{} format={} len={} return",
                        i, format, len
                    );
                    assert_eq!(
                        cbuf, rbuf,
                        "row18 tree #{} format={} len={} buffer bytes",
                        i, format, len
                    );
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

unsafe fn cmp_parse(label: &str, c: &Api, r: &Api, text: &[u8]) {
    let buf = cbytes(text);
    let ci = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
    let ri = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
    assert_eq!(
        ci.is_null(),
        ri.is_null(),
        "{}: parse NULL-ness ({:?})",
        label,
        String::from_utf8_lossy(text)
    );
    if !ci.is_null() {
        assert_eq!(
            fingerprint(c, ci),
            fingerprint(r, ri),
            "{}: parsed structure ({:?})",
            label,
            String::from_utf8_lossy(text)
        );
        assert_eq!(link_shape(ci), link_shape(ri), "{}: parsed links", label);
        assert_eq!(
            show(&print_and_free(c, ci)),
            show(&print_and_free(r, ri)),
            "{}: reprint ({:?})",
            label,
            String::from_utf8_lossy(text)
        );
        assert_eq!(
            show(&print_unformatted_and_free(c, ci)),
            show(&print_unformatted_and_free(r, ri)),
            "{}: reprint unformatted",
            label
        );
    }
    // error pointer offset relative to the shared input buffer
    let ec = (c.cJSON_GetErrorPtr)();
    let er = (r.cJSON_GetErrorPtr)();
    let offc = ec as isize - buf.as_ptr() as isize;
    let offr = er as isize - buf.as_ptr() as isize;
    assert_eq!(offc, offr, "{}: GetErrorPtr offset ({:?})", label, String::from_utf8_lossy(text));
    (c.cJSON_Delete)(ci);
    (r.cJSON_Delete)(ri);
}

/// Rows 19, 20, 24, 25, 26, 27, 28
#[test]
fn row19_parse_all_kinds() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        for t in [
            "null", "true", "false", "0", "-1.5", "\"str\"", "[]", "{}", "[1,2,3]",
            "{\"a\":1,\"b\":[true,null]}",
        ] {
            cmp_parse("row19", &c, &r, t.as_bytes());
        }
    }
}

/// Row 20 — random trees → print → reparse → print.
#[test]
fn row20_parse_random_roundtrip() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(20);
        for i in 0..400 {
            let n = random_json_node(&mut rng, 4);
            let ci = build(&c, &n);
            let printed = print_and_free(&c, ci);
            (c.cJSON_Delete)(ci);
            if let Some(text) = printed {
                cmp_parse(&format!("row20 #{}", i), &c, &r, &text);
            }
        }
    }
}

/// Row 21 — cJSON_ParseWithOpts option cross-product.
#[test]
fn row21_parse_with_opts() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let inputs: Vec<&str> = vec![
            "{}",
            "  {}  ",
            "{}x",
            "1 2",
            "[1,2]",
            "[1,2] ",
            "[1,2]\n\t",
            "null",
            "nullx",
            "\"a\"trailing",
            "",
            " ",
        ];
        for input in inputs {
            for rnt in [0i32, 1, 2, -1] {
                for use_end in [false, true] {
                    let buf = cbytes(input.as_bytes());
                    let mut endc: *const c_char = ptr::null();
                    let mut endr: *const c_char = ptr::null();
                    let pc = if use_end {
                        &mut endc as *mut *const c_char
                    } else {
                        ptr::null_mut()
                    };
                    let pr = if use_end {
                        &mut endr as *mut *const c_char
                    } else {
                        ptr::null_mut()
                    };
                    let ci = (c.cJSON_ParseWithOpts)(buf.as_ptr() as *const c_char, pc, rnt);
                    let ri = (r.cJSON_ParseWithOpts)(buf.as_ptr() as *const c_char, pr, rnt);
                    assert_eq!(
                        ci.is_null(),
                        ri.is_null(),
                        "row21 {:?} rnt={} end={}: NULL-ness",
                        input,
                        rnt,
                        use_end
                    );
                    if use_end {
                        let oc = endc as isize - buf.as_ptr() as isize;
                        let or = endr as isize - buf.as_ptr() as isize;
                        assert_eq!(
                            oc, or,
                            "row21 {:?} rnt={}: return_parse_end offset",
                            input, rnt
                        );
                    }
                    if !ci.is_null() {
                        assert_eq!(
                            show(&print_unformatted_and_free(&c, ci)),
                            show(&print_unformatted_and_free(&r, ri)),
                            "row21 {:?} rnt={}: printed",
                            input,
                            rnt
                        );
                    }
                    let oc = (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize;
                    let or = (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize;
                    assert_eq!(oc, or, "row21 {:?} rnt={}: error offset", input, rnt);
                    (c.cJSON_Delete)(ci);
                    (r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

/// Rows 22 & 23 — length based parsing.
#[test]
fn row22_23_parse_with_length() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let mut cases: Vec<Vec<u8>> = vec![
            b"{}".to_vec(),
            b"[1,2,3]".to_vec(),
            b"  null  ".to_vec(),
            b"{\"a\":\"b\"}".to_vec(),
            b"12345".to_vec(),
            b"\"abc\"".to_vec(),
        ];
        // input with an embedded NUL: only length based parsing can see past it
        cases.push(b"[1,\0 2]".to_vec());
        cases.push(b"{\"a\0b\":1}".to_vec());

        let mut rng = Rng::new(2223);
        for _ in 0..80 {
            let n = random_json_node(&mut rng, 3);
            let ci = build(&c, &n);
            if let Some(t) = print_unformatted_and_free(&c, ci) {
                cases.push(t);
            }
            (c.cJSON_Delete)(ci);
        }

        for text in &cases {
            let buf = cbytes(text);
            let strlen = text.iter().position(|&b| b == 0).unwrap_or(text.len());
            let lens: Vec<usize> = vec![
                0,
                1,
                strlen.saturating_sub(1),
                strlen,
                strlen + 1,
                text.len(),
                text.len() + 1,
            ];
            for &len in &lens {
                // cJSON_ParseWithLength
                let ci = (c.cJSON_ParseWithLength)(buf.as_ptr() as *const c_char, len);
                let ri = (r.cJSON_ParseWithLength)(buf.as_ptr() as *const c_char, len);
                assert_eq!(
                    ci.is_null(),
                    ri.is_null(),
                    "row22 {:?} len={}: NULL-ness",
                    String::from_utf8_lossy(text),
                    len
                );
                if !ci.is_null() {
                    assert_eq!(
                        show(&print_unformatted_and_free(&c, ci)),
                        show(&print_unformatted_and_free(&r, ri)),
                        "row22 {:?} len={}",
                        String::from_utf8_lossy(text),
                        len
                    );
                }
                (c.cJSON_Delete)(ci);
                (r.cJSON_Delete)(ri);

                // cJSON_ParseWithLengthOpts full cross-product
                for rnt in [0i32, 1] {
                    for use_end in [false, true] {
                        let mut endc: *const c_char = ptr::null();
                        let mut endr: *const c_char = ptr::null();
                        let pc = if use_end {
                            &mut endc as *mut *const c_char
                        } else {
                            ptr::null_mut()
                        };
                        let pr = if use_end {
                            &mut endr as *mut *const c_char
                        } else {
                            ptr::null_mut()
                        };
                        let ci = (c.cJSON_ParseWithLengthOpts)(
                            buf.as_ptr() as *const c_char,
                            len,
                            pc,
                            rnt,
                        );
                        let ri = (r.cJSON_ParseWithLengthOpts)(
                            buf.as_ptr() as *const c_char,
                            len,
                            pr,
                            rnt,
                        );
                        assert_eq!(
                            ci.is_null(),
                            ri.is_null(),
                            "row23 {:?} len={} rnt={} end={}",
                            String::from_utf8_lossy(text),
                            len,
                            rnt,
                            use_end
                        );
                        if use_end {
                            assert_eq!(
                                endc as isize - buf.as_ptr() as isize,
                                endr as isize - buf.as_ptr() as isize,
                                "row23 end offset {:?} len={} rnt={}",
                                String::from_utf8_lossy(text),
                                len,
                                rnt
                            );
                        }
                        if !ci.is_null() {
                            assert_eq!(
                                show(&print_unformatted_and_free(&c, ci)),
                                show(&print_unformatted_and_free(&r, ri)),
                                "row23 printed {:?} len={} rnt={}",
                                String::from_utf8_lossy(text),
                                len,
                                rnt
                            );
                        }
                        assert_eq!(
                            (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                            (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize,
                            "row23 error offset {:?} len={} rnt={}",
                            String::from_utf8_lossy(text),
                            len,
                            rnt
                        );
                        (c.cJSON_Delete)(ci);
                        (r.cJSON_Delete)(ri);
                    }
                }
            }
        }
    }
}

/// Row 24 — BOM and whitespace.
#[test]
fn row24_bom_and_whitespace() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let bodies = ["{}", "[1]", "null", "\"x\"", "1"];
        let prefixes: Vec<Vec<u8>> = vec![
            vec![],
            b"\xef\xbb\xbf".to_vec(),
            b" ".to_vec(),
            b"\t".to_vec(),
            b"\r\n".to_vec(),
            b"\xef\xbb\xbf  \t".to_vec(),
            b" \xef\xbb\xbf".to_vec(), // BOM after whitespace (skip_utf8_bom requires offset 0)
            b"\xef\xbb".to_vec(),      // truncated BOM
        ];
        let suffixes: Vec<&[u8]> = vec![b"", b" ", b"\t\r\n", b"\0extra"];
        for body in bodies {
            for p in &prefixes {
                for s in &suffixes {
                    let mut t = p.clone();
                    t.extend_from_slice(body.as_bytes());
                    t.extend_from_slice(s);
                    cmp_parse("row24", &c, &r, &t);
                }
            }
        }
    }
}

/// Row 25 — number literals.
#[test]
fn row25_number_literals() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let mut lits: Vec<String> = vec![
            "0", "-0", "1", "-1", "1e5", "1E+5", "1e-5", "1.5", "-1.5", "0.0", "1e0",
            "123456789012345678901234567890", "0.1", "0.3", "1e308", "1e309", "1e400",
            "-1e400", "1e-400", "3.141592653589793", "2.220446049250313e-16",
            "1.7976931348623157e308", "5e-324", "2147483647", "2147483648",
            "-2147483648", "-2147483649", "9007199254740993", "1234567890123456789",
            "0.30000000000000004", "1e1000000", "1.0000000000000002",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        let mut rng = Rng::new(25);
        for _ in 0..500 {
            let d = rng.f64();
            if d.is_finite() {
                lits.push(format!("{:?}", d));
                lits.push(format!("{:e}", d));
            }
        }
        for l in &lits {
            cmp_parse("row25", &c, &r, l.as_bytes());
            cmp_parse("row25-arr", &c, &r, format!("[{}]", l).as_bytes());
        }
    }
}

/// Row 26 — string escapes and UTF-8.
#[test]
fn row26_string_escapes() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let mut lits: Vec<Vec<u8>> = vec![
            br#""\"""#.to_vec(),
            br#""\\""#.to_vec(),
            br#""\/""#.to_vec(),
            br#""\b\f\n\r\t""#.to_vec(),
            br#"" ""#.to_vec(),
            br#""""#.to_vec(),
            "\"\u{e9}\"".as_bytes().to_vec(),
            "\"\u{20ac}\"".as_bytes().to_vec(),
            "\"\u{1f600}\"".as_bytes().to_vec(),
            "\"\u{1f600} tail\"".as_bytes().to_vec(),
            br#""AB""#.to_vec(),
            "\"é€😀\"".as_bytes().to_vec(),
            b"\"\x7f\"".to_vec(),
            b"\"\xc3\xa9\"".to_vec(),
            b"\"\xe2\x82\xac\"".to_vec(),
            b"\"\xf0\x9f\x98\x80\"".to_vec(),
            b"\"plain\"".to_vec(),
            b"\"\"".to_vec(),
        ];
        let mut rng = Rng::new(26);
        for _ in 0..300 {
            let s = rng.string(24);
            let ci = (c.cJSON_CreateString)(cbytes(s.as_bytes()).as_ptr() as *const c_char);
            if let Some(t) = print_unformatted_and_free(&c, ci) {
                lits.push(t);
            }
            (c.cJSON_Delete)(ci);
        }
        for l in &lits {
            cmp_parse("row26", &c, &r, l);
            let mut wrapped = b"{\"k\":".to_vec();
            wrapped.extend_from_slice(l);
            wrapped.push(b'}');
            cmp_parse("row26-obj", &c, &r, &wrapped);
        }
    }
}

/// Row 27 — valid nesting depth up to the limit.
#[test]
fn row27_nesting_depth() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        for depth in [1usize, 2, 3, 998, 999, 1000] {
            for (open, close) in [('[', ']'), ('{', '}')] {
                let mut t = String::new();
                for _ in 0..depth {
                    if open == '{' {
                        t.push_str("{\"a\":");
                    } else {
                        t.push('[');
                    }
                }
                t.push_str("1");
                for _ in 0..depth {
                    t.push(close);
                }
                cmp_parse(&format!("row27 depth={} {}", depth, open), &c, &r, t.as_bytes());
            }
        }
    }
}

/// Row 28 — GetErrorPtr after success and after failure.
#[test]
fn row28_error_ptr() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        for t in [
            "{}", "{", "[1,", "tru", "@", "", "{\"a\"}", "[1,]", "\"unterminated",
            "{\"a\":}", "1.2.3", "[1 2]", "nul", "fals",
        ] {
            let buf = cbytes(t.as_bytes());
            let ci = (c.cJSON_Parse)(buf.as_ptr() as *const c_char);
            let ri = (r.cJSON_Parse)(buf.as_ptr() as *const c_char);
            assert_eq!(ci.is_null(), ri.is_null(), "row28 {:?}", t);
            let oc = (c.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize;
            let or = (r.cJSON_GetErrorPtr)() as isize - buf.as_ptr() as isize;
            assert_eq!(oc, or, "row28 {:?} error offset", t);
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
    }
}

/// Row 66 — the full low-level pipeline over randomized trees.
#[test]
fn row66_full_pipeline() {
    let _g = global_lock();
    unsafe {
        let (c, r) = both();
        let mut rng = Rng::new(66);
        for i in 0..250 {
            let n = random_json_node(&mut rng, 4);
            let ci = build(&c, &n);
            let ri = build(&r, &n);

            for prebuffer in [1i32, 64] {
                for fmt in [0i32, 1] {
                    let tc = print_buffered_and_free(&c, ci, prebuffer, fmt);
                    let tr = print_buffered_and_free(&r, ri, prebuffer, fmt);
                    assert_eq!(show(&tc), show(&tr), "row66 #{} PrintBuffered", i);

                    if let Some(text) = tc {
                        let bufc = cbytes(&text);
                        let bufr = cbytes(&text);
                        let pc = (c.cJSON_Parse)(bufc.as_ptr() as *const c_char);
                        let pr = (r.cJSON_Parse)(bufr.as_ptr() as *const c_char);
                        assert_eq!(pc.is_null(), pr.is_null(), "row66 #{} reparse", i);

                        for cs_flag in [0i32, 1] {
                            assert_eq!(
                                (c.cJSON_Compare)(ci, pc, cs_flag),
                                (r.cJSON_Compare)(ri, pr, cs_flag),
                                "row66 #{} Compare cs={}",
                                i,
                                cs_flag
                            );
                        }

                        for recurse in [0i32, 1] {
                            let dc = (c.cJSON_Duplicate)(pc, recurse);
                            let dr = (r.cJSON_Duplicate)(pr, recurse);
                            assert_eq!(
                                show(&print_and_free(&c, dc)),
                                show(&print_and_free(&r, dr)),
                                "row66 #{} Duplicate recurse={}",
                                i,
                                recurse
                            );
                            assert_eq!(
                                (c.cJSON_Compare)(pc, dc, 1),
                                (r.cJSON_Compare)(pr, dr, 1),
                                "row66 #{} Compare duplicate",
                                i
                            );
                            (c.cJSON_Delete)(dc);
                            (r.cJSON_Delete)(dr);
                        }

                        // Minify then reparse
                        let mut mc = cbytes(&text);
                        let mut mr = cbytes(&text);
                        (c.cJSON_Minify)(mc.as_mut_ptr() as *mut c_char);
                        (r.cJSON_Minify)(mr.as_mut_ptr() as *mut c_char);
                        assert_eq!(mc, mr, "row66 #{} Minify bytes", i);
                        let m2c = (c.cJSON_Parse)(mc.as_ptr() as *const c_char);
                        let m2r = (r.cJSON_Parse)(mr.as_ptr() as *const c_char);
                        assert_eq!(m2c.is_null(), m2r.is_null(), "row66 #{} minified parse", i);
                        assert_eq!(
                            show(&print_and_free(&c, m2c)),
                            show(&print_and_free(&r, m2r)),
                            "row66 #{} minified reprint",
                            i
                        );
                        (c.cJSON_Delete)(m2c);
                        (r.cJSON_Delete)(m2r);

                        (c.cJSON_Delete)(pc);
                        (r.cJSON_Delete)(pr);
                    }
                }
            }
            (c.cJSON_Delete)(ci);
            (r.cJSON_Delete)(ri);
        }
        let _ = ptr::null::<c_void>();
        let _: c_int = 0;
    }
}
