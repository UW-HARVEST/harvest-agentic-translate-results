//! Phase B — CONFIGS.md rows 26–34, 73, 74: the four print entry points across
//! both `format` values and every buffer-size boundary, `print_number`'s four
//! formatting branches, `print_string_ptr`'s escape arithmetic, nested
//! indentation, and low-level caller-fabricated `cJSON` nodes.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_char, c_int, c_void};

/// Every print entry point × format, plus a full buffer-size sweep, compared
/// byte-for-byte.  This is what `observe()` does; here it is applied to
/// deliberately chosen and to randomly generated trees.
fn cmp(c: &Api, r: &Api, spec: &Spec, ctx: &str) {
    assert_spec_matches(c, r, spec, ctx);
}

// ---------------------------------------------------------------------------
// rows 26–31 — the four print entry points over randomized nested trees
// ---------------------------------------------------------------------------
#[test]
fn cfg26_31_print_entry_points_random_trees() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2631_2631);
    for depth in [0usize, 1, 2, 3, 4] {
        for i in 0..250 {
            let spec = rand_spec(&mut rng, depth);
            cmp(&c, &r, &spec, &format!("random tree depth={depth} #{i}"));
        }
    }
}

#[test]
fn cfg26_31_print_entry_points_explicit_shapes() {
    let (c, r) = both();
    let specs = vec![
        Spec::Null,
        Spec::True,
        Spec::False,
        Spec::Num(0.0),
        Spec::Str(b"".to_vec()),
        Spec::Raw(b"".to_vec()),
        Spec::Arr(vec![]),
        Spec::Obj(vec![]),
        Spec::Arr(vec![Spec::Arr(vec![Spec::Arr(vec![Spec::Arr(vec![])])])]),
        Spec::Obj(vec![(
            b"a".to_vec(),
            Spec::Obj(vec![(
                b"b".to_vec(),
                Spec::Obj(vec![(b"c".to_vec(), Spec::Arr(vec![Spec::Num(1.0)]))]),
            )]),
        )]),
        // long payloads that force several `ensure` growth rounds
        Spec::Arr((0..200).map(|i| Spec::Num(i as f64)).collect()),
        Spec::Obj(
            (0..120)
                .map(|i| (format!("key{i:04}").into_bytes(), Spec::Num(i as f64 / 7.0)))
                .collect(),
        ),
        Spec::Str(vec![b'x'; 1000]),
        Spec::Str(vec![b'\n'; 300]),
        Spec::Str(vec![1u8; 300]),
        Spec::Raw(vec![b'z'; 1000]),
    ];
    for (i, spec) in specs.iter().enumerate() {
        cmp(&c, &r, spec, &format!("explicit shape #{i}"));
    }
}

// ---------------------------------------------------------------------------
// rows 28, 29 — cJSON_PrintBuffered prebuffer sweep (explicit, beyond observe)
// ---------------------------------------------------------------------------
#[test]
fn cfg28_29_print_buffered_prebuffer_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0x2829_2829);
    let specs: Vec<Spec> = (0..40).map(|_| rand_spec(&mut rng, 3)).collect();
    for (si, spec) in specs.iter().enumerate() {
        unsafe {
            let bc = build(&c, spec);
            let br = build(&r, spec);
            for pb in [
                0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 31, 32, 63, 64, 127, 128, 200, 254, 255, 256, 257,
                258, 511, 512, 1023, 1024, 4095, 65536,
            ] {
                for fmt in [0, 1] {
                    let a = print_buffered_and_take(&c, bc.root, pb, fmt);
                    let b = print_buffered_and_take(&r, br.root, pb, fmt);
                    assert_eq!(
                        a,
                        b,
                        "PrintBuffered(prebuffer={pb}, fmt={fmt}) spec #{si} = {spec:?}\nC={} Rust={}",
                        show(&a),
                        show(&b)
                    );
                }
            }
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 30, 31 — cJSON_PrintPreallocated length sweep over the whole range
// ---------------------------------------------------------------------------
#[test]
fn cfg30_31_print_preallocated_length_sweep() {
    let (c, r) = both();
    let mut rng = Rng::new(0x3031_3031);
    let specs: Vec<Spec> = (0..30).map(|_| rand_spec(&mut rng, 3)).collect();
    for (si, spec) in specs.iter().enumerate() {
        unsafe {
            let bc = build(&c, spec);
            let br = build(&r, spec);
            for fmt in [0, 1] {
                let want = if fmt == 1 {
                    print_and_take(&c, bc.root)
                } else {
                    print_unformatted_and_take(&c, bc.root)
                };
                let exact = want.map(|v| v.len()).unwrap_or(0);
                // every length from 0 to exact+8 — this walks the `ensure`
                // noalloc rejection across every single output position.
                for len in 0..=(exact + 8) {
                    let cap = len + 64;
                    let mut buf_c = vec![0xAAu8; cap];
                    let mut buf_r = vec![0xAAu8; cap];
                    let rc_c = (c.cJSON_PrintPreallocated)(
                        bc.root,
                        buf_c.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    let rc_r = (r.cJSON_PrintPreallocated)(
                        br.root,
                        buf_r.as_mut_ptr() as *mut c_char,
                        len as c_int,
                        fmt,
                    );
                    assert_eq!(
                        rc_c, rc_r,
                        "PrintPreallocated rc differs: spec #{si} fmt={fmt} len={len} (exact={exact})\nspec = {spec:?}"
                    );
                    // Compare the whole buffer, including the guard bytes past
                    // `len`: a translation that overruns the caller's buffer by
                    // even one byte must be caught.
                    assert_eq!(
                        &buf_c[..],
                        &buf_r[..],
                        "PrintPreallocated buffer differs: spec #{si} fmt={fmt} len={len} (exact={exact})\n\
                         spec = {spec:?}\nC    = {:?}\nRust = {:?}",
                        String::from_utf8_lossy(&buf_c),
                        String::from_utf8_lossy(&buf_r)
                    );
                }
            }
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// row 32 — print_number's four formatting branches
// ---------------------------------------------------------------------------
#[test]
fn cfg32_print_number_branches() {
    let (c, r) = both();

    // (a) values reached through cJSON_CreateNumber
    for d in number_pool() {
        cmp(&c, &r, &Spec::Num(d), &format!("print_number({d:?})"));
    }
    let mut rng = Rng::new(0x3232_3232);
    for i in 0..6000 {
        let d = match i % 4 {
            0 => rng.any_f64(),
            1 => rng.json_f64(),
            2 => (rng.range_i32(i32::MIN, i32::MAX) as f64) + 0.5,
            _ => f64::from_bits(rng.next_u64() & 0x7FEF_FFFF_FFFF_FFFF),
        };
        cmp(
            &c,
            &r,
            &Spec::Num(d),
            &format!("print_number random #{i} bits={:#018x}", d.to_bits()),
        );
    }

    // (b) `valueint` deliberately desynchronised from `valuedouble` so that the
    // `d == (double)item->valueint` branch (cJSON.c:578) is taken/not taken for
    // combinations no constructor can produce.
    unsafe {
        let pairs: Vec<(f64, c_int)> = vec![
            (5.0, 5),
            (5.0, 6),
            (5.5, 5),
            (0.0, 0),
            (-0.0, 0),
            (3.0, 7),
            (2147483647.0, i32::MAX),
            (2147483647.0, 0),
            (-2147483648.0, i32::MIN),
            (-2147483648.0, 1),
            (1e300, i32::MAX),
            (f64::NAN, 0),
            (f64::INFINITY, i32::MAX),
            (f64::NEG_INFINITY, i32::MIN),
            (1.0 / 3.0, 0),
            (1e15, 0),
            (-1.0, -1),
            (1e-300, 0),
        ];
        for (d, vi) in pairs {
            let nc = (c.cJSON_CreateNumber)(0.0);
            let nr = (r.cJSON_CreateNumber)(0.0);
            (*nc).valuedouble = d;
            (*nr).valuedouble = d;
            (*nc).valueint = vi;
            (*nr).valueint = vi;
            let oc = observe(&c, nc);
            let or = observe(&r, nr);
            assert_obs_eq(&oc, &or, &format!("desync number ({d:?}, {vi})"), &Spec::Num(d));
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
        // randomized desynchronisation
        let mut rng = Rng::new(0x3232_DE59);
        for i in 0..3000 {
            let d = if rng.bool() { rng.json_f64() } else { rng.any_f64() };
            let vi = rng.range_i32(i32::MIN, i32::MAX);
            let nc = (c.cJSON_CreateNumber)(0.0);
            let nr = (r.cJSON_CreateNumber)(0.0);
            (*nc).valuedouble = d;
            (*nr).valuedouble = d;
            (*nc).valueint = vi;
            (*nr).valueint = vi;
            let a = print_and_take(&c, nc);
            let b = print_and_take(&r, nr);
            assert_eq!(
                a,
                b,
                "desync number random #{i}: valuedouble bits {:#018x}, valueint {vi}\nC={} Rust={}",
                d.to_bits(),
                show(&a),
                show(&b)
            );
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
    }
}

// ---------------------------------------------------------------------------
// row 33 — print_string_ptr: escape counting and the NULL input path
// ---------------------------------------------------------------------------
#[test]
fn cfg33_print_string_branches() {
    let (c, r) = both();

    // every single byte, and every byte followed by a plain character
    for b in 1u16..=255 {
        let b = b as u8;
        for pattern in [
            vec![b],
            vec![b, b'x'],
            vec![b'x', b],
            vec![b, b, b],
            vec![b'a', b, b'b', b, b'c'],
        ] {
            cmp(
                &c,
                &r,
                &Spec::Str(pattern.clone()),
                &format!("print_string byte {b:#04x} pattern {pattern:?}"),
            );
        }
    }

    // strings whose escape count changes output_length arithmetic
    let mut s = Vec::new();
    for b in 1u16..32 {
        s.push(b as u8);
    }
    cmp(&c, &r, &Spec::Str(s.clone()), "all control bytes");
    cmp(&c, &r, &Spec::Raw(s), "all control bytes as Raw");

    // A String item whose `valuestring` is NULL takes the `input == NULL` path
    // in print_string_ptr (cJSON.c:931) and prints as `""`.
    unsafe {
        for &type_ in &[cJSON_String, cJSON_String | cJSON_StringIsConst] {
            let sc = (c.cJSON_CreateString)(cs("x").as_ptr());
            let sr = (r.cJSON_CreateString)(cs("x").as_ptr());
            (c.cJSON_free)((*sc).valuestring as *mut c_void);
            (r.cJSON_free)((*sr).valuestring as *mut c_void);
            (*sc).valuestring = std::ptr::null_mut();
            (*sr).valuestring = std::ptr::null_mut();
            (*sc).type_ = type_;
            (*sr).type_ = type_;
            let oc = observe(&c, sc);
            let or = observe(&r, sr);
            assert_obs_eq(
                &oc,
                &or,
                &format!("String with NULL valuestring, type={type_:#x}"),
                &Spec::Str(vec![]),
            );
            (*sc).type_ = cJSON_String;
            (*sr).type_ = cJSON_String;
            (c.cJSON_Delete)(sc);
            (r.cJSON_Delete)(sr);
        }
    }

    // An object whose children have `string == NULL` (i.e. an array retyped as
    // an object) exercises print_object's key path with a NULL key.
    unsafe {
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Str(b"two".to_vec()), Spec::Null]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        (*bc.root).type_ = cJSON_Object;
        (*br.root).type_ = cJSON_Object;
        let oc = observe(&c, bc.root);
        let or = observe(&r, br.root);
        assert_obs_eq(&oc, &or, "array retyped as object (NULL keys)", &spec);
        (*bc.root).type_ = cJSON_Array;
        (*br.root).type_ = cJSON_Array;
        bc.delete();
        br.delete();
    }
}

// ---------------------------------------------------------------------------
// row 34 — formatted nesting: print_object's depth indentation loops
// ---------------------------------------------------------------------------
#[test]
fn cfg34_formatted_nesting_depth() {
    let (c, r) = both();
    for depth in 1..=12usize {
        // pure object nesting
        let mut spec = Spec::Num(1.0);
        for i in 0..depth {
            spec = Spec::Obj(vec![(format!("lvl{i}").into_bytes(), spec)]);
        }
        cmp(&c, &r, &spec, &format!("object nesting depth {depth}"));

        // alternating object/array nesting
        let mut spec = Spec::Str(b"leaf".to_vec());
        for i in 0..depth {
            spec = if i % 2 == 0 {
                Spec::Arr(vec![spec, Spec::Num(i as f64)])
            } else {
                Spec::Obj(vec![
                    (format!("k{i}").into_bytes(), spec),
                    (b"extra".to_vec(), Spec::True),
                ])
            };
        }
        cmp(&c, &r, &spec, &format!("alternating nesting depth {depth}"));
    }
}

// ---------------------------------------------------------------------------
// row 73 — caller-fabricated nodes (the lowest level a consumer can reach)
// ---------------------------------------------------------------------------
#[test]
fn cfg73_fabricated_nodes() {
    let (c, r) = both();
    unsafe {
        // (a) every `type` value fed through every print entry point
        for t in [
            0i32,
            1,
            2,
            3,
            4,
            5,
            6,
            7,
            8,
            9,
            16,
            17,
            32,
            33,
            64,
            65,
            128,
            129,
            255,
            256,
            257,
            512,
            513,
            768,
            0x0A,
            0x18,
            0x30,
            0x88,
            0xFF,
            0x1FF,
            0x2FF,
            0x3FF,
            -1,
            i32::MIN,
            i32::MAX,
        ] {
            // fabricate on top of a String so `valuestring` is a valid pointer
            let nc = (c.cJSON_CreateString)(cs("payload").as_ptr());
            let nr = (r.cJSON_CreateString)(cs("payload").as_ptr());
            (*nc).valueint = 12345;
            (*nr).valueint = 12345;
            (*nc).valuedouble = 12345.5;
            (*nr).valuedouble = 12345.5;
            (*nc).type_ = t;
            (*nr).type_ = t;
            let oc = observe(&c, nc);
            let or = observe(&r, nr);
            assert_obs_eq(
                &oc,
                &or,
                &format!("fabricated type {t:#x}"),
                &Spec::Str(b"payload".to_vec()),
            );
            (*nc).type_ = cJSON_String;
            (*nr).type_ = cJSON_String;
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }

        // (b) a Raw item with a NULL payload → print_value returns false
        let nc = (c.cJSON_CreateRaw)(cs("1").as_ptr());
        let nr = (r.cJSON_CreateRaw)(cs("1").as_ptr());
        (c.cJSON_free)((*nc).valuestring as *mut c_void);
        (r.cJSON_free)((*nr).valuestring as *mut c_void);
        (*nc).valuestring = std::ptr::null_mut();
        (*nr).valuestring = std::ptr::null_mut();
        let oc = observe(&c, nc);
        let or = observe(&r, nr);
        assert_obs_eq(&oc, &or, "Raw with NULL valuestring", &Spec::Raw(vec![]));
        (c.cJSON_Delete)(nc);
        (r.cJSON_Delete)(nr);

        // (c) containers whose child chain contains an unprintable element:
        // print_array / print_object must abort mid-way, identically.
        for bad_type in [0i32, 3, 0xFF, 0x0A] {
            for as_object in [false, true] {
                let spec = Spec::Arr(vec![
                    Spec::Num(1.0),
                    Spec::Str(b"ok".to_vec()),
                    Spec::Num(2.0),
                    Spec::True,
                ]);
                let bc = build(&c, &spec);
                let br = build(&r, &spec);
                // poison the third element
                let pc = (c.cJSON_GetArrayItem)(bc.root, 2);
                let pr = (r.cJSON_GetArrayItem)(br.root, 2);
                (*pc).type_ = bad_type;
                (*pr).type_ = bad_type;
                if as_object {
                    (*bc.root).type_ = cJSON_Object;
                    (*br.root).type_ = cJSON_Object;
                }
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("container with unprintable child type={bad_type:#x} as_object={as_object}"),
                    &spec,
                );
                (*pc).type_ = cJSON_Number;
                (*pr).type_ = cJSON_Number;
                (*bc.root).type_ = cJSON_Array;
                (*br.root).type_ = cJSON_Array;
                bc.delete();
                br.delete();
            }
        }

        // (d) sibling chains printed from a non-first node — `print_value` only
        // ever looks at `->child`, but `cJSON_Delete` walks `->next`, and
        // `cJSON_GetArraySize` walks `->child->next`.  Build a detached chain by
        // hand and observe it.
        let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0), Spec::Num(3.0)]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        let midc = (c.cJSON_GetArrayItem)(bc.root, 1);
        let midr = (r.cJSON_GetArrayItem)(br.root, 1);
        // print starting at the middle element (a Number: no children involved)
        assert_eq!(
            print_and_take(&c, midc),
            print_and_take(&r, midr),
            "print from mid-chain element"
        );
        assert_eq!(
            (c.cJSON_GetArraySize)(midc),
            (r.cJSON_GetArraySize)(midr),
            "GetArraySize on a leaf"
        );
        bc.delete();
        br.delete();
    }
}

// ---------------------------------------------------------------------------
// row 74 — locale decimal point handling
// ---------------------------------------------------------------------------
#[test]
fn cfg74_locale_decimal_point() {
    let (c, r) = both();
    // `ENABLE_LOCALES` is ON, so both sides call `localeconv()`.  The process
    // locale is "C" (neither library calls setlocale), so the decimal point is
    // '.'; the substitution loops in parse_number/print_number must therefore be
    // no-ops on both sides.  Numbers containing '.' in every position exercise
    // both loops.
    let inputs: Vec<&str> = vec![
        "0.0", "-0.0", ".5", "0.5", "1.", "1.5", "1.5e3", "1.5E-3", "123.456789012345",
        "0.000001", "1e-7", "-1.7976931348623157e308", "3.141592653589793",
    ];
    unsafe {
        for s in inputs {
            let b = Bytes::new(s.as_bytes());
            let ic = (c.cJSON_Parse)(b.as_ptr());
            let ir = (r.cJSON_Parse)(b.as_ptr());
            assert_eq!(ic.is_null(), ir.is_null(), "parse {s:?} nullness");
            assert_eq!(snap(ic), snap(ir), "parse {s:?} snapshot");
            if !ic.is_null() {
                assert_eq!(
                    print_and_take(&c, ic),
                    print_and_take(&r, ir),
                    "print of parsed {s:?}"
                );
            }
            (c.cJSON_Delete)(ic);
            (r.cJSON_Delete)(ir);
        }
    }
}
