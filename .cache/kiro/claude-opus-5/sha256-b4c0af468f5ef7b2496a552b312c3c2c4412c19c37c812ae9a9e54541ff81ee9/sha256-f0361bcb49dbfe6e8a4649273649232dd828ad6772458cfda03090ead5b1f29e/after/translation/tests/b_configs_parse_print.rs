//! Phase B part 2 — CONFIGS rows 50-96: parsing entry points, printing
//! entry points, round trips, duplicate, compare, list manipulation, minify.
mod harness;
use harness::*;
use std::ffi::{c_char, c_int};

const SEED: u64 = 0x_B10C_F00D;

/* ---------------- rows 57-72: all four parse entry points ---------------- */

/// One differential parse observation: tree snapshot, `*return_parse_end`
/// offset and `cJSON_GetErrorPtr()` offset, all relative to the shared input
/// buffer so the two libraries are directly comparable.
#[derive(Debug, PartialEq)]
struct Obs {
    item: Option<Snap>,
    end_off: isize,
    err_off: isize,
}

unsafe fn observe(
    api: &Api,
    buf: &[u8],
    item: *mut cJSON,
    end: *const c_char,
) -> Obs {
    let base = buf.as_ptr() as isize;
    let within = |q: *const c_char| -> isize {
        if q.is_null() {
            -1
        } else {
            let d = q as isize - base;
            if d >= 0 && d <= buf.len() as isize {
                d
            } else {
                // points at some other (earlier) buffer: not comparable as an
                // offset, but "outside" must agree between the two libraries.
                -2
            }
        }
    };
    let o = Obs {
        item: if item.is_null() { None } else { Some(snap(item)) },
        end_off: within(end),
        err_off: within((api.cJSON_GetErrorPtr)()),
    };
    if !item.is_null() {
        (api.cJSON_Delete)(item);
    }
    o
}

/// Drive every parse entry point / flag / length combination, interleaving the
/// C and Rust calls on the SAME buffer so that the per-library global error
/// state evolves through an identical call sequence.
fn assert_parse_all_eq(bytes: &[u8], what: &str) {
    let p = pair();
    let buf = cbytes(bytes);
    let show = String::from_utf8_lossy(&bytes[..bytes.len().min(80)]).to_string();
    unsafe {
        macro_rules! step {
            ($tag:expr, $call:expr) => {{
                let tag: String = $tag;
                #[allow(unused_mut)]
                let mut ce: *const c_char = std::ptr::null();
                #[allow(unused_mut)]
                let mut re: *const c_char = std::ptr::null();
                let (ci, cend) = $call(&p.c, &mut ce);
                let co = observe(&p.c, &buf, ci, cend);
                let (ri, rend) = $call(&p.r, &mut re);
                let ro = observe(&p.r, &buf, ri, rend);
                assert_eq!(co, ro, "{what}: {tag} differs (input {show:?})");
            }};
        }

        step!("Parse".to_string(), |a: &Api, _e: &mut *const c_char| {
            ((a.cJSON_Parse)(sp(&buf)), std::ptr::null())
        });

        for len in [buf.len(), bytes.len(), bytes.len() / 2, 1, 0] {
            step!(format!("ParseWithLength({len})"), |a: &Api,
                                                      _e: &mut *const c_char| {
                ((a.cJSON_ParseWithLength)(sp(&buf), len), std::ptr::null())
            });
        }

        for req in [0i32, 1] {
            step!(
                format!("ParseWithOpts(end,{req})"),
                |a: &Api, e: &mut *const c_char| {
                    let it = (a.cJSON_ParseWithOpts)(sp(&buf), e, req);
                    (it, *e)
                }
            );
            step!(
                format!("ParseWithOpts(NULL,{req})"),
                |a: &Api, _e: &mut *const c_char| {
                    (
                        (a.cJSON_ParseWithOpts)(sp(&buf), std::ptr::null_mut(), req),
                        std::ptr::null(),
                    )
                }
            );
        }

        for len in [buf.len(), bytes.len(), 0] {
            for req in [0i32, 1] {
                step!(
                    format!("ParseWithLengthOpts({len},end,{req})"),
                    |a: &Api, e: &mut *const c_char| {
                        let it = (a.cJSON_ParseWithLengthOpts)(sp(&buf), len, e, req);
                        (it, *e)
                    }
                );
                step!(
                    format!("ParseWithLengthOpts({len},NULL,{req})"),
                    |a: &Api, _e: &mut *const c_char| {
                        (
                            (a.cJSON_ParseWithLengthOpts)(
                                sp(&buf),
                                len,
                                std::ptr::null_mut(),
                                req,
                            ),
                            std::ptr::null(),
                        )
                    }
                );
            }
        }
    }
}

#[test]
fn rows57_72_parse_entry_points_randomized() {
    let _g = lock();
    let mut rng = Rng::new(SEED);
    for i in 0..500 {
        let doc = gen_json(&mut rng, 0);
        assert_parse_all_eq(doc.as_bytes(), &format!("random doc {i}"));
        let ws = sprinkle_ws(&mut rng, &doc);
        assert_parse_all_eq(ws.as_bytes(), &format!("random doc+ws {i}"));
    }
}

#[test]
fn row58_number_shapes() {
    let _g = lock();
    let cases = [
        "0", "-0", "1", "-1", "1.5", "-1.5", "0.0", "1e5", "1E5", "1e+5", "1E+5", "1e-5",
        "1E-5", "1e400", "-1e400", "1e-400", "-1e-400", "123456789012345678901234567890",
        "1.7976931348623157e308", "5e-324", "2147483647", "2147483648", "-2147483648",
        "-2147483649", "9223372036854775807", "0.1", "1.0000000000000002", "3.141592653589793",
        "1e", "1e+", "0e0", "-0.0", "00", "01", "1.", ".5", "1.2.3", "1e5e5", "+1", "0x10",
        "1e310", "17976931348623157000000000000000000000000000000000000000000000000000000000000\
0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\
00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
    ];
    for c in cases {
        assert_parse_all_eq(c.as_bytes(), &format!("number {c}"));
    }
}

#[test]
fn row59_string_escape_shapes() {
    let _g = lock();
    let cases: Vec<Vec<u8>> = vec![
        br#""""#.to_vec(),
        br#""a""#.to_vec(),
        br#""\"""#.to_vec(),
        br#""\\""#.to_vec(),
        br#""\/""#.to_vec(),
        br#""\b""#.to_vec(),
        br#""\f""#.to_vec(),
        br#""\n""#.to_vec(),
        br#""\r""#.to_vec(),
        br#""\t""#.to_vec(),
        br#""\u0041""#.to_vec(),
        br#""\u0000""#.to_vec(),
        br#""\u00e9""#.to_vec(),
        br#""\u20ac""#.to_vec(),
        br#""\uD83D\uDE00""#.to_vec(),
        br#""\ud83d\ude00""#.to_vec(),
        br#""\uD83D""#.to_vec(),      // lone high surrogate
        br#""\uDE00""#.to_vec(),      // lone low surrogate
        br#""\uD83Dx""#.to_vec(),
        br#""\uD83D\u0041""#.to_vec(), // high + non-surrogate
        br#""\uD83D\uD83D""#.to_vec(), // high + high
        br#""\uZZZZ""#.to_vec(),
        br#""\u12""#.to_vec(),
        br#""\x""#.to_vec(),
        br#""\""#.to_vec(),
        br#""abc"#.to_vec(),
        br#""\ud83d\ude00\u0041\u00e9\u20ac""#.to_vec(),
        b"\"\xc3\xa9\xe2\x82\xac\xf0\x9f\x98\x80\"".to_vec(),
        b"\"\x01\x1f\"".to_vec(),
        {
            let mut v = b"\"".to_vec();
            v.extend_from_slice(&vec![b'x'; 5000]);
            v.push(b'"');
            v
        },
    ];
    for c in &cases {
        assert_parse_all_eq(c, &format!("string {:?}", String::from_utf8_lossy(c)));
    }
}

#[test]
fn rows60_62_bom_whitespace_containers() {
    let _g = lock();
    let mut cases: Vec<Vec<u8>> = vec![
        b"\xEF\xBB\xBF[1,2,3]".to_vec(),
        b"\xEF\xBB\xBF".to_vec(),
        b"[\xEF\xBB\xBF1]".to_vec(), // BOM only skipped at offset 0
        b"\xEF\xBB".to_vec(),
        b"[]".to_vec(),
        b"{}".to_vec(),
        b"  []  ".to_vec(),
        b"[ ]".to_vec(),
        b"{ }".to_vec(),
        b"[1]".to_vec(),
        b"{\"a\":1}".to_vec(),
        b"".to_vec(),
        b" ".to_vec(),
        b"\t\n\r ".to_vec(),
    ];
    // every whitespace byte <= 0x20
    for b in 1u8..=0x20 {
        cases.push(vec![b, b'[', b, b'1', b, b',', b, b'2', b, b']', b]);
    }
    // many elements
    cases.push(
        format!("[{}]", (0..64).map(|i| i.to_string()).collect::<Vec<_>>().join(","))
            .into_bytes(),
    );
    cases.push(
        format!(
            "{{{}}}",
            (0..64)
                .map(|i| format!("\"k{i}\":{i}"))
                .collect::<Vec<_>>()
                .join(",")
        )
        .into_bytes(),
    );
    for c in &cases {
        assert_parse_all_eq(c, &format!("shape {:?}", String::from_utf8_lossy(c)));
    }
}

#[test]
fn row63_nesting_depths() {
    let _g = lock();
    for depth in [1usize, 2, 3, 100, 998, 999, 1000, 1001, 1500] {
        for (open, close) in [("[", "]"), ("{\"a\":", "}")] {
            let mut s = String::new();
            for _ in 0..depth {
                s.push_str(open);
            }
            s.push('1');
            for _ in 0..depth {
                s.push_str(close);
            }
            assert_parse_all_eq(s.as_bytes(), &format!("nesting {depth} {open}"));
        }
    }
}

#[test]
fn rows65_66_length_delimited_parsing() {
    let _g = lock();
    let p = pair();
    let payload = b"[1,2,3]trailing-garbage-not-nul-terminated";
    unsafe {
        for len in 0..=payload.len() {
            let ci = (p.c.cJSON_ParseWithLength)(payload.as_ptr() as *const c_char, len);
            let ri = (p.r.cJSON_ParseWithLength)(payload.as_ptr() as *const c_char, len);
            assert_eq!(ci.is_null(), ri.is_null(), "ParseWithLength len={len}");
            if !ci.is_null() {
                assert_snap_eq(ci, ri, &format!("len={len}"));
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
            for req in [0i32, 1] {
                let mut ce: *const c_char = std::ptr::null();
                let mut re: *const c_char = std::ptr::null();
                let ci = (p.c.cJSON_ParseWithLengthOpts)(
                    payload.as_ptr() as *const c_char,
                    len,
                    &mut ce,
                    req,
                );
                let ri = (p.r.cJSON_ParseWithLengthOpts)(
                    payload.as_ptr() as *const c_char,
                    len,
                    &mut re,
                    req,
                );
                assert_eq!(ci.is_null(), ri.is_null(), "PWLO len={len} req={req}");
                let co = if ce.is_null() {
                    -1
                } else {
                    ce as isize - payload.as_ptr() as isize
                };
                let ro = if re.is_null() {
                    -1
                } else {
                    re as isize - payload.as_ptr() as isize
                };
                assert_eq!(co, ro, "PWLO end offset len={len} req={req}");
                if !ci.is_null() {
                    assert_snap_eq(ci, ri, "PWLO");
                    (p.c.cJSON_Delete)(ci);
                    (p.r.cJSON_Delete)(ri);
                }
            }
        }
    }
}

#[test]
fn rows67_68_require_null_terminated() {
    let _g = lock();
    for doc in [
        "[1,2,3]", "[1,2,3] ", "[1,2,3]x", "[1,2,3]\n\t ", "1 2", "null null", "{}{}",
        "\"a\" trailing",
    ] {
        assert_parse_all_eq(doc.as_bytes(), &format!("req_nul {doc}"));
    }
}

/* ---------------- rows 50-56, 73-76: printing & round trips ------------- */

#[test]
fn rows50_56_and_73_76_print_round_trips() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xF00D);
    unsafe {
        for i in 0..400 {
            let doc = gen_json(&mut rng, 0);
            let cs = cstr(&doc);
            let ci = (p.c.cJSON_Parse)(sp(&cs));
            let ri = (p.r.cJSON_Parse)(sp(&cs));
            assert_eq!(ci.is_null(), ri.is_null(), "doc {i}: {doc}");
            if ci.is_null() {
                continue;
            }
            assert_snap_eq(ci, ri, &format!("doc {i}"));
            let co = print_all(&p.c, ci);
            let ro = print_all(&p.r, ri);
            assert_eq!(co.len(), ro.len());
            for (j, (a, b)) in co.iter().zip(ro.iter()).enumerate() {
                assert_eq!(a, b, "doc {i} print variant {j}: {doc}");
            }
            // reparse the formatted output and compare again
            if let Some(txt) = co[0].clone() {
                let t = cbytes(&txt);
                let c2 = (p.c.cJSON_Parse)(sp(&t));
                let r2 = (p.r.cJSON_Parse)(sp(&t));
                assert_eq!(c2.is_null(), r2.is_null());
                if !c2.is_null() {
                    assert_snap_eq(c2, r2, "reparse");
                    (p.c.cJSON_Delete)(c2);
                    (p.r.cJSON_Delete)(r2);
                }
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

#[test]
fn rows52_53_prebuffer_sweep_large_document() {
    let _g = lock();
    let p = pair();
    unsafe {
        let big = format!(
            "{{\"arr\":[{}],\"nested\":{}}}",
            (0..500).map(|i| format!("{}.5", i)).collect::<Vec<_>>().join(","),
            (0..40).fold("1".to_string(), |acc, _| format!("[{acc}]"))
        );
        let bs = cstr(&big);
        let ci = (p.c.cJSON_Parse)(sp(&bs));
        let ri = (p.r.cJSON_Parse)(sp(&bs));
        assert!(!ci.is_null() && !ri.is_null());
        for pre in [0i32, 1, 2, 3, 7, 16, 64, 1024, 65536] {
            for fmt in [0i32, 1] {
                let a = take_print(&p.c, (p.c.cJSON_PrintBuffered)(ci, pre, fmt));
                let b = take_print(&p.r, (p.r.cJSON_PrintBuffered)(ri, pre, fmt));
                assert_eq!(a, b, "PrintBuffered(pre={pre}, fmt={fmt})");
            }
        }
        (p.c.cJSON_Delete)(ci);
        (p.r.cJSON_Delete)(ri);
    }
}

#[test]
fn rows54_56_print_preallocated_exact_boundaries() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xAAAA);
    unsafe {
        for _ in 0..200 {
            let doc = gen_json(&mut rng, 0);
            let cs = cstr(&doc);
            let ci = (p.c.cJSON_Parse)(sp(&cs));
            let ri = (p.r.cJSON_Parse)(sp(&cs));
            if ci.is_null() {
                continue;
            }
            for fmt in [0i32, 1] {
                let reference = take_print(
                    &p.c,
                    if fmt == 1 {
                        (p.c.cJSON_Print)(ci)
                    } else {
                        (p.c.cJSON_PrintUnformatted)(ci)
                    },
                )
                .unwrap();
                let need = reference.len() + 1;
                for len in [
                    0usize,
                    1,
                    need / 2,
                    need.saturating_sub(2),
                    need - 1,
                    need,
                    need + 1,
                    need + 5,
                ] {
                    let mut cb = vec![0xAAu8; need + 64];
                    let mut rb = vec![0xAAu8; need + 64];
                    let cok = (p.c.cJSON_PrintPreallocated)(
                        ci,
                        cb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    let rok = (p.r.cJSON_PrintPreallocated)(
                        ri,
                        rb.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    assert_eq!(cok, rok, "PrintPreallocated ok len={len} fmt={fmt}: {doc}");
                    assert_eq!(
                        cb, rb,
                        "PrintPreallocated buffer len={len} fmt={fmt}: {doc}"
                    );
                }
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
    }
}

/* ---------------- rows 77-80: Duplicate ---------------- */

#[test]
fn rows77_80_duplicate() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xD0D0);
    unsafe {
        for _ in 0..300 {
            let doc = gen_json(&mut rng, 0);
            let cs = cstr(&doc);
            let ci = (p.c.cJSON_Parse)(sp(&cs));
            let ri = (p.r.cJSON_Parse)(sp(&cs));
            if ci.is_null() {
                continue;
            }
            for recurse in [0i32, 1] {
                let cd = (p.c.cJSON_Duplicate)(ci, recurse);
                let rd = (p.r.cJSON_Duplicate)(ri, recurse);
                assert_eq!(cd.is_null(), rd.is_null(), "Duplicate recurse={recurse}");
                if !cd.is_null() {
                    assert_snap_eq(cd, rd, &format!("Duplicate recurse={recurse}: {doc}"));
                    assert_eq!(print_all(&p.c, cd), print_all(&p.r, rd));
                    assert_eq!(
                        (p.c.cJSON_Compare)(ci, cd, 1),
                        (p.r.cJSON_Compare)(ri, rd, 1),
                        "Compare original vs dup recurse={recurse}"
                    );
                    (p.c.cJSON_Delete)(cd);
                    (p.r.cJSON_Delete)(rd);
                }
            }
            (p.c.cJSON_Delete)(ci);
            (p.r.cJSON_Delete)(ri);
        }
        // row 79: reference & const-key flavours
        let s = cstr("refstr");
        let k = cstr("constkey");
        let co = (p.c.cJSON_CreateObject)();
        let ro = (p.r.cJSON_CreateObject)();
        (p.c.cJSON_AddItemToObjectCS)(co, sp(&k), (p.c.cJSON_CreateStringReference)(sp(&s)));
        (p.r.cJSON_AddItemToObjectCS)(ro, sp(&k), (p.r.cJSON_CreateStringReference)(sp(&s)));
        for recurse in [0i32, 1] {
            let cd = (p.c.cJSON_Duplicate)(co, recurse);
            let rd = (p.r.cJSON_Duplicate)(ro, recurse);
            assert_snap_eq(cd, rd, "Duplicate flags");
            (p.c.cJSON_Delete)(cd);
            (p.r.cJSON_Delete)(rd);
        }
        (p.c.cJSON_Delete)(co);
        (p.r.cJSON_Delete)(ro);
    }
}

#[test]
fn row80_duplicate_long_sibling_chain_near_circular_limit() {
    let _g = lock();
    let p = pair();
    unsafe {
        // CJSON_CIRCULAR_LIMIT bounds the sibling-chain walk depth in
        // cJSON_Duplicate_rec, so build chains around that boundary.
        for n in [1usize, 2, 9_999, 10_000, 10_001] {
            let ca = (p.c.cJSON_CreateArray)();
            let ra = (p.r.cJSON_CreateArray)();
            for i in 0..n {
                (p.c.cJSON_AddItemToArray)(ca, (p.c.cJSON_CreateNumber)(i as f64));
                (p.r.cJSON_AddItemToArray)(ra, (p.r.cJSON_CreateNumber)(i as f64));
            }
            let cd = (p.c.cJSON_Duplicate)(ca, 1);
            let rd = (p.r.cJSON_Duplicate)(ra, 1);
            assert_eq!(cd.is_null(), rd.is_null(), "Duplicate chain n={n}");
            if !cd.is_null() {
                assert_eq!(
                    (p.c.cJSON_GetArraySize)(cd),
                    (p.r.cJSON_GetArraySize)(rd),
                    "dup size n={n}"
                );
                (p.c.cJSON_Delete)(cd);
                (p.r.cJSON_Delete)(rd);
            }
            (p.c.cJSON_Delete)(ca);
            (p.r.cJSON_Delete)(ra);
        }
    }
}

/* ---------------- rows 81-84: Compare ---------------- */

#[test]
fn rows81_84_compare() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xC0C0);
    unsafe {
        let mut docs: Vec<String> = (0..80).map(|_| gen_json(&mut rng, 0)).collect();
        docs.extend([
            "{\"a\":1,\"b\":2}".into(),
            "{\"b\":2,\"a\":1}".into(),
            "{\"A\":1,\"b\":2}".into(),
            "{\"a\":1}".into(),
            "[1,2]".into(),
            "[2,1]".into(),
            "[1,2,3]".into(),
            "1".into(),
            "1.0000000000000002".into(),
            "1.0".into(),
            "null".into(),
            "true".into(),
            "false".into(),
            "\"a\"".into(),
            "\"A\"".into(),
            "[]".into(),
            "{}".into(),
        ]);
        let cn: Vec<*mut cJSON> = docs
            .iter()
            .map(|d| {
                let b = cstr(d);
                (p.c.cJSON_Parse)(sp(&b))
            })
            .collect();
        let rn: Vec<*mut cJSON> = docs
            .iter()
            .map(|d| {
                let b = cstr(d);
                (p.r.cJSON_Parse)(sp(&b))
            })
            .collect();
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                for csens in [0i32, 1, 2, -1] {
                    assert_eq!(
                        (p.c.cJSON_Compare)(cn[i], cn[j], csens),
                        (p.r.cJSON_Compare)(rn[i], rn[j], csens),
                        "Compare({}, {}, {csens})",
                        docs[i],
                        docs[j]
                    );
                }
            }
        }
        for i in 0..docs.len() {
            if !cn[i].is_null() {
                (p.c.cJSON_Delete)(cn[i]);
            }
            if !rn[i].is_null() {
                (p.r.cJSON_Delete)(rn[i]);
            }
        }
        // row 84: numbers around the compare_double epsilon threshold
        let vals = [
            1.0f64,
            1.0 + f64::EPSILON,
            1.0 + 2.0 * f64::EPSILON,
            1.0 - f64::EPSILON / 2.0,
            0.0,
            -0.0,
            f64::MIN_POSITIVE,
            f64::NAN,
            f64::INFINITY,
            1e300,
            1e300 * (1.0 + f64::EPSILON),
        ];
        for a in vals {
            for b in vals {
                let ca = (p.c.cJSON_CreateNumber)(a);
                let cb = (p.c.cJSON_CreateNumber)(b);
                let ra = (p.r.cJSON_CreateNumber)(a);
                let rb = (p.r.cJSON_CreateNumber)(b);
                assert_eq!(
                    (p.c.cJSON_Compare)(ca, cb, 1),
                    (p.r.cJSON_Compare)(ra, rb, 1),
                    "Compare numbers {a:?} {b:?}"
                );
                (p.c.cJSON_Delete)(ca);
                (p.c.cJSON_Delete)(cb);
                (p.r.cJSON_Delete)(ra);
                (p.r.cJSON_Delete)(rb);
            }
        }
    }
}

#[test]
fn row83_identical_pointer_and_type_matrix() {
    let _g = lock();
    let p = pair();
    unsafe {
        let s = cstr("s");
        let makers: Vec<Box<dyn Fn(&Api) -> *mut cJSON>> = vec![
            Box::new(|a: &Api| (a.cJSON_CreateNull)()),
            Box::new(|a: &Api| (a.cJSON_CreateTrue)()),
            Box::new(|a: &Api| (a.cJSON_CreateFalse)()),
            Box::new(|a: &Api| (a.cJSON_CreateNumber)(1.0)),
            Box::new(|a: &Api| (a.cJSON_CreateArray)()),
            Box::new(|a: &Api| (a.cJSON_CreateObject)()),
        ];
        let mut cs_items: Vec<*mut cJSON> = makers.iter().map(|f| f(&p.c)).collect();
        let mut rs_items: Vec<*mut cJSON> = makers.iter().map(|f| f(&p.r)).collect();
        cs_items.push((p.c.cJSON_CreateString)(sp(&s)));
        rs_items.push((p.r.cJSON_CreateString)(sp(&s)));
        cs_items.push((p.c.cJSON_CreateRaw)(sp(&s)));
        rs_items.push((p.r.cJSON_CreateRaw)(sp(&s)));
        for i in 0..cs_items.len() {
            for j in 0..cs_items.len() {
                for csens in [0i32, 1] {
                    assert_eq!(
                        (p.c.cJSON_Compare)(cs_items[i], cs_items[j], csens),
                        (p.r.cJSON_Compare)(rs_items[i], rs_items[j], csens),
                        "type matrix ({i},{j},{csens})"
                    );
                }
            }
        }
        for i in 0..cs_items.len() {
            (p.c.cJSON_Delete)(cs_items[i]);
            (p.r.cJSON_Delete)(rs_items[i]);
        }
    }
}

/* ---------------- rows 85-93: detach / delete / insert / replace -------- */

#[test]
fn rows85_93_list_manipulation() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x1234);
    unsafe {
        for size in [1usize, 2, 3, 5, 9] {
            for which in 0..(size as c_int + 2) {
                // arrays
                for op in 0..5 {
                    let (ca, ra) = build_pair_array(&p.c, &p.r, size);
                    match op {
                        0 => {
                            let cd = (p.c.cJSON_DetachItemFromArray)(ca, which);
                            let rd = (p.r.cJSON_DetachItemFromArray)(ra, which);
                            assert_eq!(cd.is_null(), rd.is_null(), "Detach{which}/{size}");
                            if !cd.is_null() {
                                assert_snap_eq(cd, rd, "detached");
                                (p.c.cJSON_Delete)(cd);
                                (p.r.cJSON_Delete)(rd);
                            }
                        }
                        1 => {
                            (p.c.cJSON_DeleteItemFromArray)(ca, which);
                            (p.r.cJSON_DeleteItemFromArray)(ra, which);
                        }
                        2 => {
                            let cn = (p.c.cJSON_CreateString)(sp(&cstr("ins")));
                            let rn = (p.r.cJSON_CreateString)(sp(&cstr("ins")));
                            assert_eq!(
                                (p.c.cJSON_InsertItemInArray)(ca, which, cn),
                                (p.r.cJSON_InsertItemInArray)(ra, which, rn),
                                "Insert{which}/{size}"
                            );
                        }
                        3 => {
                            let cn = (p.c.cJSON_CreateString)(sp(&cstr("rep")));
                            let rn = (p.r.cJSON_CreateString)(sp(&cstr("rep")));
                            assert_eq!(
                                (p.c.cJSON_ReplaceItemInArray)(ca, which, cn),
                                (p.r.cJSON_ReplaceItemInArray)(ra, which, rn),
                                "Replace{which}/{size}"
                            );
                        }
                        _ => {
                            let ct = (p.c.cJSON_GetArrayItem)(ca, which);
                            let rt = (p.r.cJSON_GetArrayItem)(ra, which);
                            let cd = (p.c.cJSON_DetachItemViaPointer)(ca, ct);
                            let rd = (p.r.cJSON_DetachItemViaPointer)(ra, rt);
                            assert_eq!(cd.is_null(), rd.is_null(), "DetachVia{which}/{size}");
                            if !cd.is_null() {
                                assert_snap_eq(cd, rd, "detached via ptr");
                                (p.c.cJSON_Delete)(cd);
                                (p.r.cJSON_Delete)(rd);
                            }
                        }
                    }
                    assert_snap_eq(ca, ra, &format!("array op={op} which={which} size={size}"));
                    assert_eq!(print_all(&p.c, ca), print_all(&p.r, ra));
                    (p.c.cJSON_Delete)(ca);
                    (p.r.cJSON_Delete)(ra);
                }
                // objects
                for op in 0..4 {
                    let (co, ro) = build_pair_object(&p.c, &p.r, size);
                    let key = cbytes(format!("k{}", which.max(0)).as_bytes());
                    match op {
                        0 => {
                            let cd = (p.c.cJSON_DetachItemFromObject)(co, sp(&key));
                            let rd = (p.r.cJSON_DetachItemFromObject)(ro, sp(&key));
                            assert_eq!(cd.is_null(), rd.is_null());
                            if !cd.is_null() {
                                assert_snap_eq(cd, rd, "obj detached");
                                (p.c.cJSON_Delete)(cd);
                                (p.r.cJSON_Delete)(rd);
                            }
                        }
                        1 => {
                            let cd =
                                (p.c.cJSON_DetachItemFromObjectCaseSensitive)(co, sp(&key));
                            let rd =
                                (p.r.cJSON_DetachItemFromObjectCaseSensitive)(ro, sp(&key));
                            assert_eq!(cd.is_null(), rd.is_null());
                            if !cd.is_null() {
                                (p.c.cJSON_Delete)(cd);
                                (p.r.cJSON_Delete)(rd);
                            }
                        }
                        2 => {
                            (p.c.cJSON_DeleteItemFromObject)(co, sp(&key));
                            (p.r.cJSON_DeleteItemFromObject)(ro, sp(&key));
                        }
                        _ => {
                            let cn = (p.c.cJSON_CreateNumber)(rng.nice_f64());
                            let rn = (p.r.cJSON_CreateNumber)(0.0);
                            (*rn).valuedouble = (*cn).valuedouble;
                            (*rn).valueint = (*cn).valueint;
                            assert_eq!(
                                (p.c.cJSON_ReplaceItemInObject)(co, sp(&key), cn),
                                (p.r.cJSON_ReplaceItemInObject)(ro, sp(&key), rn),
                            );
                        }
                    }
                    assert_snap_eq(co, ro, &format!("object op={op} key={which}"));
                    assert_eq!(print_all(&p.c, co), print_all(&p.r, ro));
                    (p.c.cJSON_Delete)(co);
                    (p.r.cJSON_Delete)(ro);
                }
                // case-insensitive object variants
                let (co, ro) = build_pair_object(&p.c, &p.r, size);
                let ukey = cbytes(format!("K{}", which.max(0)).as_bytes());
                let cn = (p.c.cJSON_CreateNumber)(9.0);
                let rn = (p.r.cJSON_CreateNumber)(9.0);
                assert_eq!(
                    (p.c.cJSON_ReplaceItemInObjectCaseSensitive)(co, sp(&ukey), cn),
                    (p.r.cJSON_ReplaceItemInObjectCaseSensitive)(ro, sp(&ukey), rn),
                );
                assert_snap_eq(co, ro, "ReplaceItemInObjectCaseSensitive");
                (p.c.cJSON_DeleteItemFromObjectCaseSensitive)(co, sp(&ukey));
                (p.r.cJSON_DeleteItemFromObjectCaseSensitive)(ro, sp(&ukey));
                assert_snap_eq(co, ro, "DeleteItemFromObjectCaseSensitive");
                (p.c.cJSON_Delete)(co);
                (p.r.cJSON_Delete)(ro);
            }
        }
    }
}

#[test]
fn row91_replace_via_pointer_positions() {
    let _g = lock();
    let p = pair();
    unsafe {
        for size in [1usize, 2, 3, 4] {
            for pos in 0..size {
                for container in 0..2 {
                    let (cc, rc) = if container == 0 {
                        build_pair_array(&p.c, &p.r, size)
                    } else {
                        build_pair_object(&p.c, &p.r, size)
                    };
                    let ct = (p.c.cJSON_GetArrayItem)(cc, pos as c_int);
                    let rt = (p.r.cJSON_GetArrayItem)(rc, pos as c_int);
                    let cn = (p.c.cJSON_CreateString)(sp(&cstr("REPL")));
                    let rn = (p.r.cJSON_CreateString)(sp(&cstr("REPL")));
                    assert_eq!(
                        (p.c.cJSON_ReplaceItemViaPointer)(cc, ct, cn),
                        (p.r.cJSON_ReplaceItemViaPointer)(rc, rt, rn),
                        "ReplaceViaPointer pos={pos} size={size} container={container}"
                    );
                    assert_snap_eq(cc, rc, "after ReplaceViaPointer");
                    assert_eq!(print_all(&p.c, cc), print_all(&p.r, rc));
                    // self-replacement is a no-op success
                    let ct = (p.c.cJSON_GetArrayItem)(cc, pos as c_int);
                    let rt = (p.r.cJSON_GetArrayItem)(rc, pos as c_int);
                    assert_eq!(
                        (p.c.cJSON_ReplaceItemViaPointer)(cc, ct, ct),
                        (p.r.cJSON_ReplaceItemViaPointer)(rc, rt, rt),
                    );
                    assert_snap_eq(cc, rc, "after self-replace");
                    (p.c.cJSON_Delete)(cc);
                    (p.r.cJSON_Delete)(rc);
                }
            }
        }
    }
}

unsafe fn build_pair_array(c: &Api, r: &Api, n: usize) -> (*mut cJSON, *mut cJSON) {
    let ca = (c.cJSON_CreateArray)();
    let ra = (r.cJSON_CreateArray)();
    for i in 0..n {
        (c.cJSON_AddItemToArray)(ca, (c.cJSON_CreateNumber)(i as f64));
        (r.cJSON_AddItemToArray)(ra, (r.cJSON_CreateNumber)(i as f64));
    }
    (ca, ra)
}

unsafe fn build_pair_object(c: &Api, r: &Api, n: usize) -> (*mut cJSON, *mut cJSON) {
    let co = (c.cJSON_CreateObject)();
    let ro = (r.cJSON_CreateObject)();
    for i in 0..n {
        let k = cbytes(format!("k{i}").as_bytes());
        (c.cJSON_AddItemToObject)(co, sp(&k), (c.cJSON_CreateNumber)(i as f64));
        (r.cJSON_AddItemToObject)(ro, sp(&k), (r.cJSON_CreateNumber)(i as f64));
    }
    (co, ro)
}

/* ---------------- row 94: Minify ---------------- */

#[test]
fn row94_minify() {
    let _g = lock();
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0x9999);
    let mut cases: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"\t\r\n ".to_vec(),
        b"{ \"a\" : 1 }".to_vec(),
        b"[1, 2, 3]".to_vec(),
        b"// line comment\n[1]".to_vec(),
        b"[1] // trailing".to_vec(),
        b"/* block */[1]".to_vec(),
        b"[/*a*/1/*b*/]".to_vec(),
        b"/* unterminated".to_vec(),
        b"/*".to_vec(),
        b"/".to_vec(),
        b"//".to_vec(),
        b"[\"a // b\"]".to_vec(),
        b"[\"a /* b */ c\"]".to_vec(),
        b"[\"esc \\\" quote\"]".to_vec(),
        b"[\"unterminated".to_vec(),
        b"\"\\\\\"".to_vec(),
        b"{\n  \"a\": [1, 2],\n  \"b\": \"x y\"\n}".to_vec(),
        b"/*/".to_vec(),
        b"/**/".to_vec(),
        b"\"".to_vec(),
    ];
    for _ in 0..200 {
        let doc = gen_json(&mut rng, 0);
        cases.push(sprinkle_ws(&mut rng, &doc).into_bytes());
    }
    unsafe {
        for c in &cases {
            let mut cb = cbytes(c);
            let mut rb = cbytes(c);
            (p.c.cJSON_Minify)(cb.as_mut_ptr() as *mut c_char);
            (p.r.cJSON_Minify)(rb.as_mut_ptr() as *mut c_char);
            assert_eq!(
                cb,
                rb,
                "Minify {:?}",
                String::from_utf8_lossy(&c[..c.len().min(60)])
            );
        }
    }
}

/* ---------------- rows 99-100: composed pipeline ---------------- */

#[test]
fn rows99_100_composed_pipeline() {
    let _g = lock();
    let p = pair();
    for hooked in [false, true] {
        let mut rng = Rng::new(SEED ^ 0xEEEE ^ (hooked as u64));
        unsafe {
            if hooked {
                let mut h = cJSON_Hooks {
                    malloc_fn: Some(hook_malloc),
                    free_fn: Some(hook_free),
                };
                (p.c.cJSON_InitHooks)(&mut h);
                (p.r.cJSON_InitHooks)(&mut h);
            }
            for iter in 0..250 {
                let doc = gen_json(&mut rng, 0);
                let cs = cstr(&doc);
                let ci = (p.c.cJSON_Parse)(sp(&cs));
                let ri = (p.r.cJSON_Parse)(sp(&cs));
                assert_eq!(ci.is_null(), ri.is_null());
                if ci.is_null() {
                    continue;
                }
                // a random sequence of mutations applied identically to both
                for _ in 0..6 {
                    let sz = (p.c.cJSON_GetArraySize)(ci);
                    assert_eq!(sz, (p.r.cJSON_GetArraySize)(ri));
                    let which = if sz > 0 {
                        rng.below(sz as usize + 2) as c_int
                    } else {
                        0
                    };
                    let key = cbytes(format!("m{}", rng.below(4)).as_bytes());
                    match rng.below(7) {
                        0 => {
                            let d = rng.nice_f64();
                            let cn = (p.c.cJSON_CreateNumber)(d);
                            let rn = (p.r.cJSON_CreateNumber)(d);
                            assert_eq!(
                                (p.c.cJSON_AddItemToArray)(ci, cn),
                                (p.r.cJSON_AddItemToArray)(ri, rn)
                            );
                        }
                        1 => {
                            let d = rng.nice_f64();
                            let cp = (p.c.cJSON_AddNumberToObject)(ci, sp(&key), d);
                            let rp = (p.r.cJSON_AddNumberToObject)(ri, sp(&key), d);
                            assert_eq!(cp.is_null(), rp.is_null());
                        }
                        2 => {
                            let cd = (p.c.cJSON_DetachItemFromArray)(ci, which);
                            let rd = (p.r.cJSON_DetachItemFromArray)(ri, which);
                            assert_eq!(cd.is_null(), rd.is_null());
                            if !cd.is_null() {
                                assert_snap_eq(cd, rd, "pipeline detach");
                                (p.c.cJSON_Delete)(cd);
                                (p.r.cJSON_Delete)(rd);
                            }
                        }
                        3 => {
                            let s = cbytes(&rng.cstring(8));
                            let cn = (p.c.cJSON_CreateString)(sp(&s));
                            let rn = (p.r.cJSON_CreateString)(sp(&s));
                            assert_eq!(
                                (p.c.cJSON_InsertItemInArray)(ci, which, cn),
                                (p.r.cJSON_InsertItemInArray)(ri, which, rn)
                            );
                        }
                        4 => {
                            let cn = (p.c.cJSON_CreateArray)();
                            let rn = (p.r.cJSON_CreateArray)();
                            assert_eq!(
                                (p.c.cJSON_ReplaceItemInArray)(ci, which, cn),
                                (p.r.cJSON_ReplaceItemInArray)(ri, which, rn)
                            );
                        }
                        5 => {
                            (p.c.cJSON_DeleteItemFromObject)(ci, sp(&key));
                            (p.r.cJSON_DeleteItemFromObject)(ri, sp(&key));
                        }
                        _ => {
                            (p.c.cJSON_DeleteItemFromArray)(ci, which);
                            (p.r.cJSON_DeleteItemFromArray)(ri, which);
                        }
                    }
                    assert_snap_eq(ci, ri, &format!("pipeline iter {iter}: {doc}"));
                }
                for recurse in [0i32, 1] {
                    let cd = (p.c.cJSON_Duplicate)(ci, recurse);
                    let rd = (p.r.cJSON_Duplicate)(ri, recurse);
                    assert_eq!(cd.is_null(), rd.is_null());
                    if !cd.is_null() {
                        assert_snap_eq(cd, rd, "pipeline duplicate");
                        for cs2 in [0i32, 1] {
                            assert_eq!(
                                (p.c.cJSON_Compare)(ci, cd, cs2),
                                (p.r.cJSON_Compare)(ri, rd, cs2)
                            );
                        }
                        (p.c.cJSON_Delete)(cd);
                        (p.r.cJSON_Delete)(rd);
                    }
                }
                assert_eq!(print_all(&p.c, ci), print_all(&p.r, ri), "pipeline print");
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
            if hooked {
                (p.c.cJSON_InitHooks)(std::ptr::null_mut());
                (p.r.cJSON_InitHooks)(std::ptr::null_mut());
            }
        }
    }
}

unsafe extern "C" fn hook_malloc(sz: usize) -> *mut std::ffi::c_void {
    raw_malloc(sz)
}
unsafe extern "C" fn hook_free(ptr: *mut std::ffi::c_void) {
    raw_free(ptr)
}
extern "C" {
    #[link_name = "malloc"]
    fn raw_malloc(sz: usize) -> *mut std::ffi::c_void;
    #[link_name = "free"]
    fn raw_free(p: *mut std::ffi::c_void);
}

/* ---------------- row 63 (print side) + row 95: deep nesting ------------ */

/// `print_array`/`print_object` emit `depth` indentation tabs per level when
/// `format` is set, so deep trees exercise a different amount of the ensure()
/// growth logic than shallow ones.  Also confirms `cJSON_Delete` walks deep
/// trees identically.
#[test]
fn row63_print_side_deep_nesting() {
    let _g = lock();
    let p = pair();
    unsafe {
        for depth in [1usize, 2, 10, 100, 500, 998, 999] {
            for (open, close) in [("[", "]"), ("{\"k\":", "}")] {
                let mut s = String::new();
                for _ in 0..depth {
                    s.push_str(open);
                }
                s.push_str("123.456");
                for _ in 0..depth {
                    s.push_str(close);
                }
                let b = cstr(&s);
                let ci = (p.c.cJSON_Parse)(sp(&b));
                let ri = (p.r.cJSON_Parse)(sp(&b));
                assert_eq!(ci.is_null(), ri.is_null(), "deep {depth} {open}");
                if ci.is_null() {
                    continue;
                }
                assert_snap_eq(ci, ri, &format!("deep {depth} {open}"));
                for fmt in [0i32, 1] {
                    let a = take_print(
                        &p.c,
                        if fmt == 1 {
                            (p.c.cJSON_Print)(ci)
                        } else {
                            (p.c.cJSON_PrintUnformatted)(ci)
                        },
                    );
                    let b2 = take_print(
                        &p.r,
                        if fmt == 1 {
                            (p.r.cJSON_Print)(ri)
                        } else {
                            (p.r.cJSON_PrintUnformatted)(ri)
                        },
                    );
                    assert_eq!(a, b2, "deep {depth} {open} print fmt={fmt}");
                    // PrintPreallocated at the exact needed size and one short
                    let need = a.as_ref().unwrap().len() + 1;
                    for len in [need, need - 1] {
                        let mut cb = vec![0x33u8; need + 16];
                        let mut rb = vec![0x33u8; need + 16];
                        assert_eq!(
                            (p.c.cJSON_PrintPreallocated)(
                                ci,
                                cb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                fmt
                            ),
                            (p.r.cJSON_PrintPreallocated)(
                                ri,
                                rb.as_mut_ptr() as *mut c_char,
                                len as c_int,
                                fmt
                            ),
                            "deep {depth} PrintPreallocated len={len} fmt={fmt}"
                        );
                        assert_eq!(cb, rb, "deep {depth} buffer len={len} fmt={fmt}");
                    }
                    for pre in [0i32, 1, 7, 4096] {
                        assert_eq!(
                            take_print(&p.c, (p.c.cJSON_PrintBuffered)(ci, pre, fmt)),
                            take_print(&p.r, (p.r.cJSON_PrintBuffered)(ri, pre, fmt)),
                            "deep {depth} PrintBuffered pre={pre} fmt={fmt}"
                        );
                    }
                }
                // row 95: Delete of a deep tree
                (p.c.cJSON_Delete)(ci);
                (p.r.cJSON_Delete)(ri);
            }
        }
    }
}

/// Row 95: `cJSON_Delete` must respect `cJSON_IsReference` / `cJSON_StringIsConst`
/// (a wrong branch would free memory it does not own).  Build a container that
/// mixes owned children, references and constant keys, delete it, then keep
/// using the referenced objects.
#[test]
fn row95_delete_respects_reference_flags() {
    let _g = lock();
    let p = pair();
    unsafe {
        let key = cstr("constkey");
        let sref = cstr("static string not owned by cJSON");
        for _ in 0..50 {
            let cowned = (p.c.cJSON_Parse)(sp(&cstr("[1,2,3]")));
            let rowned = (p.r.cJSON_Parse)(sp(&cstr("[1,2,3]")));
            let cbox = (p.c.cJSON_CreateObject)();
            let rbox = (p.r.cJSON_CreateObject)();
            (p.c.cJSON_AddItemToObjectCS)(cbox, sp(&key), (p.c.cJSON_CreateStringReference)(sp(&sref)));
            (p.r.cJSON_AddItemToObjectCS)(rbox, sp(&key), (p.r.cJSON_CreateStringReference)(sp(&sref)));
            (p.c.cJSON_AddItemReferenceToArray)(cbox, cowned);
            (p.r.cJSON_AddItemReferenceToArray)(rbox, rowned);
            (p.c.cJSON_AddItemToObject)(cbox, sp(&cstr("owned")), (p.c.cJSON_CreateNumber)(1.5));
            (p.r.cJSON_AddItemToObject)(rbox, sp(&cstr("owned")), (p.r.cJSON_CreateNumber)(1.5));
            assert_snap_eq(cbox, rbox, "mixed-ownership container");
            assert_eq!(print_all(&p.c, cbox), print_all(&p.r, rbox));
            (p.c.cJSON_Delete)(cbox);
            (p.r.cJSON_Delete)(rbox);
            // the referenced tree must still be intact and usable
            assert_snap_eq(cowned, rowned, "referenced tree survives Delete");
            assert_eq!(print_all(&p.c, cowned), print_all(&p.r, rowned));
            (p.c.cJSON_Delete)(cowned);
            (p.r.cJSON_Delete)(rowned);
        }
    }
}
