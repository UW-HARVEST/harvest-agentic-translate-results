//! Phase B — CONFIGS.md rows 49–65: `cJSON_Minify`, `cJSON_Duplicate`,
//! `cJSON_Compare`, the detach / delete / insert / replace family,
//! `cJSON_SetNumberHelper`, `cJSON_SetValuestring` and `cJSON_Delete`.
#![allow(non_snake_case)]

mod harness;

use harness::*;
use std::ffi::{c_int, c_void};

// ---------------------------------------------------------------------------
// row 49 — cJSON_Minify
// ---------------------------------------------------------------------------

/// Minifies `input` in place with both libraries and compares the ENTIRE
/// buffer afterwards (not just up to the new NUL): `cJSON_Minify` leaves the
/// tail of the original buffer untouched, and that trailing garbage is part of
/// the observable behaviour.
fn minify_case(c: &Api, r: &Api, input: &[u8]) {
    unsafe {
        let mut bc = Bytes::new(input);
        let mut br = Bytes::new(input);
        (c.cJSON_Minify)(bc.as_mut_ptr());
        (r.cJSON_Minify)(br.as_mut_ptr());
        if bc.0 != br.0 {
            panic!(
                "cJSON_Minify differs\ninput = {:?}\nC     = {:?}\nRust  = {:?}",
                String::from_utf8_lossy(input),
                String::from_utf8_lossy(&bc.0),
                String::from_utf8_lossy(&br.0)
            );
        }
        // and the resulting C strings must be equal too
        assert_eq!(cstr(bc.as_ptr()), cstr(br.as_ptr()));
    }
}

#[test]
fn cfg49_minify_explicit() {
    let (c, r) = both();
    let cases: Vec<&[u8]> = vec![
        b"",
        b" ",
        b"\t\r\n ",
        b"{}",
        b"{ }",
        b"{ \"a\" : 1 }",
        b"[ 1 , 2 , 3 ]",
        b"//comment\n{}",
        b"//comment",
        b"//comment\n",
        b"{}//trailing",
        b"/*block*/{}",
        b"/*block",
        b"/*",
        b"/",
        b"//",
        b"/x",
        b"{/*a*/\"k\"/*b*/:/*c*/1/*d*/}",
        br#""a b c""#,
        br#" "a b c" "#,
        br#""with \" quote""#,
        br#""with \\ backslash""#,
        br#""with // slashes""#,
        br#""with /* comment */""#,
        br#""unterminated"#,
        br#""ends with backslash \"#,
        br#"["a", "b"]"#,
        br#"{"a": "x y", "b": [1, 2]}"#,
        b"\"\\\\\"",
        b"\"\\\\\\\"\"",
        b"\"\\\"",
        b"\"\\\"\"",
        b"\t\"a\"\t",
        b"[\"\\t\", \"\\n\"]",
        b"\x01\x02",
        b"\x80\xff",
        b"a/b//c\nd",
        b"a/*b*/c",
        b"*/",
        b"1 2 3",
        b"[1,2,3]//x\n/*y*/[4]",
    ];
    for case in cases {
        minify_case(&c, &r, case);
    }
}

#[test]
fn cfg49_minify_randomized() {
    let (c, r) = both();
    let mut rng = Rng::new(0x4949_4949);
    let alphabet: &[u8] = b"{}[],:\"\\/*\n\r\t 01abz";
    for _ in 0..20000 {
        let n = rng.below(28);
        let mut t: Vec<u8> = (0..n).map(|_| alphabet[rng.below(alphabet.len())]).collect();
        // occasionally sprinkle high bytes
        if rng.below(6) == 0 && !t.is_empty() {
            let i = rng.below(t.len());
            t[i] = (0x80 + rng.below(0x80)) as u8;
        }
        minify_case(&c, &r, &t);
    }
    // and every valid document from a JSON-shaped corpus with random whitespace
    for _ in 0..4000 {
        let mut t: Vec<u8> = Vec::new();
        let toks: &[&[u8]] = &[
            b"{", b"}", b"[", b"]", b",", b":", b"\"a\"", b"1", b"true", b"null", b" ", b"\t",
            b"\n", b"//c\n", b"/*c*/", b"\"a\\\"b\"", b"\"a//b\"", b"\"a/*b*/\"",
        ];
        for _ in 0..rng.below(14) {
            t.extend_from_slice(toks[rng.below(toks.len())]);
        }
        minify_case(&c, &r, &t);
    }
}

// ---------------------------------------------------------------------------
// rows 50, 51 — cJSON_Duplicate
// ---------------------------------------------------------------------------
#[test]
fn cfg50_51_duplicate() {
    let (c, r) = both();
    let mut rng = Rng::new(0x5051_5051);

    for depth in [0usize, 1, 2, 3, 4] {
        for i in 0..200 {
            let spec = rand_spec(&mut rng, depth);
            for recurse in [0, 1, 2, -1] {
                unsafe {
                    let bc = build(&c, &spec);
                    let br = build(&r, &spec);
                    let dc = (c.cJSON_Duplicate)(bc.root, recurse);
                    let dr = (r.cJSON_Duplicate)(br.root, recurse);
                    assert_eq!(
                        dc.is_null(),
                        dr.is_null(),
                        "Duplicate nullness (depth={depth} #{i} recurse={recurse}) spec={spec:?}"
                    );
                    let oc = observe(&c, dc);
                    let or = observe(&r, dr);
                    assert_obs_eq(
                        &oc,
                        &or,
                        &format!("Duplicate(recurse={recurse}) depth={depth} #{i}"),
                        &spec,
                    );
                    // the original must be untouched and still identical
                    let oc = observe(&c, bc.root);
                    let or = observe(&r, br.root);
                    assert_obs_eq(&oc, &or, "original after Duplicate", &spec);
                    // Compare(original, duplicate) must agree on both sides
                    for cs_flag in [0, 1] {
                        assert_eq!(
                            (c.cJSON_Compare)(bc.root, dc, cs_flag),
                            (r.cJSON_Compare)(br.root, dr, cs_flag),
                            "Compare(original, duplicate, {cs_flag}) spec={spec:?}"
                        );
                    }
                    (c.cJSON_Delete)(dc);
                    (r.cJSON_Delete)(dr);
                    bc.delete();
                    br.delete();
                }
            }
        }
    }

    // deep `child` chains: exercise cJSON_Duplicate_rec's depth accounting
    for depth in [1usize, 8, 64, 512] {
        let mut spec = Spec::Num(1.0);
        for _ in 0..depth {
            spec = Spec::Arr(vec![spec]);
        }
        unsafe {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            for recurse in [0, 1] {
                let dc = (c.cJSON_Duplicate)(bc.root, recurse);
                let dr = (r.cJSON_Duplicate)(br.root, recurse);
                assert_eq!(dc.is_null(), dr.is_null(), "deep Duplicate depth={depth}");
                assert_eq!(
                    print_and_take(&c, dc),
                    print_and_take(&r, dr),
                    "deep Duplicate print depth={depth} recurse={recurse}"
                );
                (c.cJSON_Delete)(dc);
                (r.cJSON_Delete)(dr);
            }
            bc.delete();
            br.delete();
        }
    }

    // StringIsConst keys must be shared (not strdup'ed) by Duplicate
    unsafe {
        let spec = Spec::ObjCS(vec![
            (b"const_a".to_vec(), Spec::Num(1.0)),
            (b"const_b".to_vec(), Spec::Str(b"v".to_vec())),
        ]);
        let bc = build(&c, &spec);
        let br = build(&r, &spec);
        let dc = (c.cJSON_Duplicate)(bc.root, 1);
        let dr = (r.cJSON_Duplicate)(br.root, 1);
        // the duplicated child's key must alias the ORIGINAL key buffer
        let kc = (*(*dc).child).string;
        let kr = (*(*dr).child).string;
        let oc = (*(*bc.root).child).string;
        let or = (*(*br.root).child).string;
        assert_eq!(
            kc == oc,
            kr == or,
            "Duplicate must share StringIsConst keys identically"
        );
        assert!(kc == oc, "C shares the const key (sanity)");
        assert_eq!(print_and_take(&c, dc), print_and_take(&r, dr));
        (c.cJSON_Delete)(dc);
        (r.cJSON_Delete)(dr);
        bc.delete();
        br.delete();
    }
}

// ---------------------------------------------------------------------------
// rows 52, 53 — cJSON_Compare
// ---------------------------------------------------------------------------
#[test]
fn cfg52_53_compare() {
    let (c, r) = both();
    let mut rng = Rng::new(0x5253_5253);

    // hand-picked pairs covering every switch arm and every rejection
    let pairs: Vec<(Spec, Spec)> = vec![
        (Spec::Null, Spec::Null),
        (Spec::True, Spec::True),
        (Spec::False, Spec::False),
        (Spec::True, Spec::False),
        (Spec::Null, Spec::True),
        (Spec::Num(1.0), Spec::Num(1.0)),
        (Spec::Num(1.0), Spec::Num(1.0000000000000002)),
        (Spec::Num(1.0), Spec::Num(2.0)),
        (Spec::Num(0.0), Spec::Num(-0.0)),
        (Spec::Num(f64::NAN), Spec::Num(f64::NAN)),
        (Spec::Num(f64::INFINITY), Spec::Num(f64::INFINITY)),
        (Spec::Num(f64::INFINITY), Spec::Num(f64::NEG_INFINITY)),
        (Spec::Num(1e308), Spec::Num(1e308)),
        (Spec::Str(b"a".to_vec()), Spec::Str(b"a".to_vec())),
        (Spec::Str(b"a".to_vec()), Spec::Str(b"A".to_vec())),
        (Spec::Str(b"".to_vec()), Spec::Str(b"".to_vec())),
        (Spec::Raw(b"1".to_vec()), Spec::Raw(b"1".to_vec())),
        (Spec::Raw(b"1".to_vec()), Spec::Raw(b"2".to_vec())),
        (Spec::Str(b"a".to_vec()), Spec::Raw(b"a".to_vec())),
        (Spec::Arr(vec![]), Spec::Arr(vec![])),
        (Spec::Arr(vec![Spec::Num(1.0)]), Spec::Arr(vec![Spec::Num(1.0)])),
        (Spec::Arr(vec![Spec::Num(1.0)]), Spec::Arr(vec![])),
        (
            Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]),
            Spec::Arr(vec![Spec::Num(2.0), Spec::Num(1.0)]),
        ),
        (Spec::Obj(vec![]), Spec::Obj(vec![])),
        (
            Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]),
            Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]),
        ),
        (
            Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]),
            Spec::Obj(vec![(b"A".to_vec(), Spec::Num(1.0))]),
        ),
        (
            Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]),
            Spec::Obj(vec![
                (b"a".to_vec(), Spec::Num(1.0)),
                (b"b".to_vec(), Spec::Num(2.0)),
            ]),
        ),
        (
            Spec::Obj(vec![
                (b"a".to_vec(), Spec::Num(1.0)),
                (b"b".to_vec(), Spec::Num(2.0)),
            ]),
            Spec::Obj(vec![
                (b"b".to_vec(), Spec::Num(2.0)),
                (b"a".to_vec(), Spec::Num(1.0)),
            ]),
        ),
        // arrays whose children have NULL keys, compared as objects
        (
            Spec::Arr(vec![Spec::Num(1.0)]),
            Spec::Obj(vec![(b"a".to_vec(), Spec::Num(1.0))]),
        ),
        (
            Spec::ObjCS(vec![(b"k".to_vec(), Spec::Num(1.0))]),
            Spec::Obj(vec![(b"k".to_vec(), Spec::Num(1.0))]),
        ),
        (
            Spec::StrRef(b"same".to_vec()),
            Spec::Str(b"same".to_vec()),
        ),
    ];

    for (i, (sa, sb)) in pairs.iter().enumerate() {
        unsafe {
            let ac = build(&c, sa);
            let bc = build(&c, sb);
            let ar = build(&r, sa);
            let br = build(&r, sb);
            for cs_flag in [0, 1, 2, -1] {
                assert_eq!(
                    (c.cJSON_Compare)(ac.root, bc.root, cs_flag),
                    (r.cJSON_Compare)(ar.root, br.root, cs_flag),
                    "Compare pair #{i} cs={cs_flag}\na = {sa:?}\nb = {sb:?}"
                );
                assert_eq!(
                    (c.cJSON_Compare)(bc.root, ac.root, cs_flag),
                    (r.cJSON_Compare)(br.root, ar.root, cs_flag),
                    "Compare (reversed) pair #{i} cs={cs_flag}"
                );
                // self comparison
                assert_eq!(
                    (c.cJSON_Compare)(ac.root, ac.root, cs_flag),
                    (r.cJSON_Compare)(ar.root, ar.root, cs_flag),
                    "Compare self pair #{i} cs={cs_flag}"
                );
            }
            ac.delete();
            bc.delete();
            ar.delete();
            br.delete();
        }
    }

    // randomized pairs
    for i in 0..3000 {
        let sa = rand_spec(&mut rng, 2);
        let sb = if rng.below(3) == 0 {
            sa.clone()
        } else {
            rand_spec(&mut rng, 2)
        };
        unsafe {
            let ac = build(&c, &sa);
            let bc = build(&c, &sb);
            let ar = build(&r, &sa);
            let br = build(&r, &sb);
            for cs_flag in [0, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ac.root, bc.root, cs_flag),
                    (r.cJSON_Compare)(ar.root, br.root, cs_flag),
                    "random Compare #{i} cs={cs_flag}\na = {sa:?}\nb = {sb:?}"
                );
            }
            ac.delete();
            bc.delete();
            ar.delete();
            br.delete();
        }
    }

    // Compare with fabricated `type` values on both sides
    unsafe {
        for t in [0i32, 3, 0x0A, 0xFF, 256, 512, 0x110, 0x210, -1] {
            let ac = (c.cJSON_CreateNumber)(1.0);
            let bcx = (c.cJSON_CreateNumber)(1.0);
            let ar = (r.cJSON_CreateNumber)(1.0);
            let brx = (r.cJSON_CreateNumber)(1.0);
            for (x, y) in [(ac, bcx), (ar, brx)] {
                (*x).type_ = t;
                (*y).type_ = t;
            }
            for cs_flag in [0, 1] {
                assert_eq!(
                    (c.cJSON_Compare)(ac, bcx, cs_flag),
                    (r.cJSON_Compare)(ar, brx, cs_flag),
                    "Compare with fabricated type {t:#x} cs={cs_flag}"
                );
            }
            for x in [ac, bcx] {
                (*x).type_ = cJSON_Number;
            }
            for x in [ar, brx] {
                (*x).type_ = cJSON_Number;
            }
            (c.cJSON_Delete)(ac);
            (c.cJSON_Delete)(bcx);
            (r.cJSON_Delete)(ar);
            (r.cJSON_Delete)(brx);
        }

        // String items with NULL valuestring
        let ac = (c.cJSON_CreateString)(cs("x").as_ptr());
        let bcx = (c.cJSON_CreateString)(cs("x").as_ptr());
        let ar = (r.cJSON_CreateString)(cs("x").as_ptr());
        let brx = (r.cJSON_CreateString)(cs("x").as_ptr());
        (c.cJSON_free)((*ac).valuestring as *mut c_void);
        (*ac).valuestring = std::ptr::null_mut();
        (r.cJSON_free)((*ar).valuestring as *mut c_void);
        (*ar).valuestring = std::ptr::null_mut();
        for cs_flag in [0, 1] {
            assert_eq!(
                (c.cJSON_Compare)(ac, bcx, cs_flag),
                (r.cJSON_Compare)(ar, brx, cs_flag),
                "Compare String with NULL valuestring cs={cs_flag}"
            );
            assert_eq!(
                (c.cJSON_Compare)(bcx, ac, cs_flag),
                (r.cJSON_Compare)(brx, ar, cs_flag),
                "Compare (reversed) String with NULL valuestring cs={cs_flag}"
            );
        }
        (c.cJSON_Delete)(ac);
        (c.cJSON_Delete)(bcx);
        (r.cJSON_Delete)(ar);
        (r.cJSON_Delete)(brx);
    }
}

// ---------------------------------------------------------------------------
// rows 54–62 — detach / delete / insert / replace across every position
// ---------------------------------------------------------------------------

fn container_specs() -> Vec<Spec> {
    let mut v = Vec::new();
    for n in 0..7usize {
        v.push(Spec::Arr(
            (0..n).map(|i| Spec::Num(i as f64 * 10.0)).collect(),
        ));
        v.push(Spec::Obj(
            (0..n)
                .map(|i| (format!("k{i}").into_bytes(), Spec::Num(i as f64 * 10.0)))
                .collect(),
        ));
        v.push(Spec::ObjCS(
            (0..n)
                .map(|i| (format!("c{i}").into_bytes(), Spec::Str(format!("v{i}").into_bytes())))
                .collect(),
        ));
    }
    v.push(Spec::Obj(vec![
        (b"dup".to_vec(), Spec::Num(1.0)),
        (b"DUP".to_vec(), Spec::Num(2.0)),
        (b"dup".to_vec(), Spec::Num(3.0)),
    ]));
    v.push(Spec::Arr(vec![
        Spec::Arr(vec![Spec::Num(1.0)]),
        Spec::Obj(vec![(b"x".to_vec(), Spec::Num(2.0))]),
        Spec::Str(b"s".to_vec()),
    ]));
    v
}

#[test]
fn cfg54_55_57_59_detach_delete_insert_by_index() {
    let (c, r) = both();
    for (si, spec) in container_specs().iter().enumerate() {
        let size = match spec {
            Spec::Arr(v) => v.len() as c_int,
            Spec::Obj(v) | Spec::ObjCS(v) => v.len() as c_int,
            _ => 0,
        };
        let mut indices: Vec<c_int> = vec![-1, 0, size / 2, size - 1, size, size + 1, i32::MIN];
        indices.sort_unstable();
        indices.dedup();

        for which in indices {
            // cJSON_DetachItemFromArray
            unsafe {
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let dc = (c.cJSON_DetachItemFromArray)(bc.root, which);
                let dr = (r.cJSON_DetachItemFromArray)(br.root, which);
                assert_eq!(
                    dc.is_null(),
                    dr.is_null(),
                    "DetachItemFromArray nullness spec#{si} which={which}"
                );
                assert_eq!(
                    snap(dc),
                    snap(dr),
                    "detached node spec#{si} which={which}"
                );
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("parent after DetachItemFromArray({which}) spec#{si}"),
                    spec,
                );
                (c.cJSON_Delete)(dc);
                (r.cJSON_Delete)(dr);
                bc.delete();
                br.delete();
            }

            // cJSON_DeleteItemFromArray
            unsafe {
                let bc = build(&c, spec);
                let br = build(&r, spec);
                (c.cJSON_DeleteItemFromArray)(bc.root, which);
                (r.cJSON_DeleteItemFromArray)(br.root, which);
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("parent after DeleteItemFromArray({which}) spec#{si}"),
                    spec,
                );
                bc.delete();
                br.delete();
            }

            // cJSON_InsertItemInArray
            unsafe {
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let nc = (c.cJSON_CreateString)(cs("INSERTED").as_ptr());
                let nr = (r.cJSON_CreateString)(cs("INSERTED").as_ptr());
                let rc = (c.cJSON_InsertItemInArray)(bc.root, which, nc);
                let rr = (r.cJSON_InsertItemInArray)(br.root, which, nr);
                assert_eq!(
                    rc, rr,
                    "InsertItemInArray rc spec#{si} which={which}"
                );
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("parent after InsertItemInArray({which}) spec#{si}"),
                    spec,
                );
                if rc == 0 {
                    (c.cJSON_Delete)(nc);
                    (r.cJSON_Delete)(nr);
                }
                bc.delete();
                br.delete();
            }

            // cJSON_ReplaceItemInArray
            unsafe {
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let nc = (c.cJSON_CreateString)(cs("REPLACED").as_ptr());
                let nr = (r.cJSON_CreateString)(cs("REPLACED").as_ptr());
                let rc = (c.cJSON_ReplaceItemInArray)(bc.root, which, nc);
                let rr = (r.cJSON_ReplaceItemInArray)(br.root, which, nr);
                assert_eq!(rc, rr, "ReplaceItemInArray rc spec#{si} which={which}");
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("parent after ReplaceItemInArray({which}) spec#{si}"),
                    spec,
                );
                if rc == 0 {
                    (c.cJSON_Delete)(nc);
                    (r.cJSON_Delete)(nr);
                }
                bc.delete();
                br.delete();
            }
        }
    }
}

#[test]
fn cfg54_60_detach_replace_via_pointer() {
    let (c, r) = both();
    for (si, spec) in container_specs().iter().enumerate() {
        let size = (c.cJSON_GetArraySize) as usize; // silence unused warning path
        let _ = size;
        unsafe {
            let n = {
                let bc = build(&c, spec);
                let s = (c.cJSON_GetArraySize)(bc.root);
                bc.delete();
                s
            };
            for idx in 0..n.max(1) {
                // DetachItemViaPointer
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let ic = (c.cJSON_GetArrayItem)(bc.root, idx);
                let ir = (r.cJSON_GetArrayItem)(br.root, idx);
                let dc = (c.cJSON_DetachItemViaPointer)(bc.root, ic);
                let dr = (r.cJSON_DetachItemViaPointer)(br.root, ir);
                assert_eq!(
                    dc.is_null(),
                    dr.is_null(),
                    "DetachItemViaPointer nullness spec#{si} idx={idx}"
                );
                assert_eq!(snap(dc), snap(dr), "detached node spec#{si} idx={idx}");
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("parent after DetachItemViaPointer idx={idx} spec#{si}"),
                    spec,
                );
                (c.cJSON_Delete)(dc);
                (r.cJSON_Delete)(dr);
                bc.delete();
                br.delete();

                // ReplaceItemViaPointer
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let ic = (c.cJSON_GetArrayItem)(bc.root, idx);
                let ir = (r.cJSON_GetArrayItem)(br.root, idx);
                let nc = (c.cJSON_CreateNumber)(-999.5);
                let nr = (r.cJSON_CreateNumber)(-999.5);
                let rc = (c.cJSON_ReplaceItemViaPointer)(bc.root, ic, nc);
                let rr = (r.cJSON_ReplaceItemViaPointer)(br.root, ir, nr);
                assert_eq!(rc, rr, "ReplaceItemViaPointer rc spec#{si} idx={idx}");
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(
                    &oc,
                    &or,
                    &format!("parent after ReplaceItemViaPointer idx={idx} spec#{si}"),
                    spec,
                );
                if rc == 0 {
                    (c.cJSON_Delete)(nc);
                    (r.cJSON_Delete)(nr);
                }
                bc.delete();
                br.delete();

                // ReplaceItemViaPointer with replacement == item (early true)
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let ic = (c.cJSON_GetArrayItem)(bc.root, idx);
                let ir = (r.cJSON_GetArrayItem)(br.root, idx);
                assert_eq!(
                    (c.cJSON_ReplaceItemViaPointer)(bc.root, ic, ic),
                    (r.cJSON_ReplaceItemViaPointer)(br.root, ir, ir),
                    "ReplaceItemViaPointer(item, item) spec#{si} idx={idx}"
                );
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(&oc, &or, "parent after self-replace", spec);
                bc.delete();
                br.delete();
            }
        }
    }
}

#[test]
fn cfg56_58_62_object_key_operations() {
    let (c, r) = both();
    let keys: Vec<Vec<u8>> = vec![
        b"k0".to_vec(),
        b"K0".to_vec(),
        b"k1".to_vec(),
        b"k2".to_vec(),
        b"c0".to_vec(),
        b"C0".to_vec(),
        b"dup".to_vec(),
        b"DUP".to_vec(),
        b"missing".to_vec(),
        b"".to_vec(),
    ];
    for (si, spec) in container_specs().iter().enumerate() {
        for key in &keys {
            let kb = Bytes::new(key);
            let label = format!("spec#{si} key={:?}", String::from_utf8_lossy(key));

            // DetachItemFromObject
            unsafe {
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let dc = (c.cJSON_DetachItemFromObject)(bc.root, kb.as_ptr());
                let dr = (r.cJSON_DetachItemFromObject)(br.root, kb.as_ptr());
                assert_eq!(dc.is_null(), dr.is_null(), "DetachItemFromObject {label}");
                assert_eq!(snap(dc), snap(dr), "detached {label}");
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(&oc, &or, &format!("after DetachItemFromObject {label}"), spec);
                (c.cJSON_Delete)(dc);
                (r.cJSON_Delete)(dr);
                bc.delete();
                br.delete();
            }

            // DetachItemFromObjectCaseSensitive
            unsafe {
                let bc = build(&c, spec);
                let br = build(&r, spec);
                let dc = (c.cJSON_DetachItemFromObjectCaseSensitive)(bc.root, kb.as_ptr());
                let dr = (r.cJSON_DetachItemFromObjectCaseSensitive)(br.root, kb.as_ptr());
                assert_eq!(dc.is_null(), dr.is_null(), "Detach…CaseSensitive {label}");
                assert_eq!(snap(dc), snap(dr), "detached CS {label}");
                let oc = observe(&c, bc.root);
                let or = observe(&r, br.root);
                assert_obs_eq(&oc, &or, &format!("after Detach…CaseSensitive {label}"), spec);
                (c.cJSON_Delete)(dc);
                (r.cJSON_Delete)(dr);
                bc.delete();
                br.delete();
            }

            // DeleteItemFromObject / …CaseSensitive
            for case_sensitive in [false, true] {
                unsafe {
                    let bc = build(&c, spec);
                    let br = build(&r, spec);
                    if case_sensitive {
                        (c.cJSON_DeleteItemFromObjectCaseSensitive)(bc.root, kb.as_ptr());
                        (r.cJSON_DeleteItemFromObjectCaseSensitive)(br.root, kb.as_ptr());
                    } else {
                        (c.cJSON_DeleteItemFromObject)(bc.root, kb.as_ptr());
                        (r.cJSON_DeleteItemFromObject)(br.root, kb.as_ptr());
                    }
                    let oc = observe(&c, bc.root);
                    let or = observe(&r, br.root);
                    assert_obs_eq(
                        &oc,
                        &or,
                        &format!("after DeleteItemFromObject(cs={case_sensitive}) {label}"),
                        spec,
                    );
                    bc.delete();
                    br.delete();
                }
            }

            // ReplaceItemInObject / …CaseSensitive
            for case_sensitive in [false, true] {
                unsafe {
                    let bc = build(&c, spec);
                    let br = build(&r, spec);
                    let nc = (c.cJSON_CreateNumber)(4242.0);
                    let nr = (r.cJSON_CreateNumber)(4242.0);
                    let (rc, rr) = if case_sensitive {
                        (
                            (c.cJSON_ReplaceItemInObjectCaseSensitive)(bc.root, kb.as_ptr(), nc),
                            (r.cJSON_ReplaceItemInObjectCaseSensitive)(br.root, kb.as_ptr(), nr),
                        )
                    } else {
                        (
                            (c.cJSON_ReplaceItemInObject)(bc.root, kb.as_ptr(), nc),
                            (r.cJSON_ReplaceItemInObject)(br.root, kb.as_ptr(), nr),
                        )
                    };
                    assert_eq!(
                        rc, rr,
                        "ReplaceItemInObject(cs={case_sensitive}) rc {label}"
                    );
                    // the replacement's key is rewritten even on failure
                    assert_eq!(snap(nc), snap(nr), "replacement node state {label}");
                    let oc = observe(&c, bc.root);
                    let or = observe(&r, br.root);
                    assert_obs_eq(
                        &oc,
                        &or,
                        &format!("after ReplaceItemInObject(cs={case_sensitive}) {label}"),
                        spec,
                    );
                    if rc == 0 {
                        (c.cJSON_Delete)(nc);
                        (r.cJSON_Delete)(nr);
                    }
                    bc.delete();
                    br.delete();
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 63 — cJSON_SetNumberHelper
// ---------------------------------------------------------------------------
#[test]
fn cfg63_set_number_helper() {
    let (c, r) = both();
    let mut rng = Rng::new(0x6363_6363);
    let mut values = number_pool();
    for _ in 0..4000 {
        values.push(if rng.bool() { rng.json_f64() } else { rng.any_f64() });
    }
    unsafe {
        for d in values {
            let nc = (c.cJSON_CreateNumber)(0.0);
            let nr = (r.cJSON_CreateNumber)(0.0);
            let rc = (c.cJSON_SetNumberHelper)(nc, d);
            let rr = (r.cJSON_SetNumberHelper)(nr, d);
            assert_eq!(
                rc.to_bits(),
                rr.to_bits(),
                "SetNumberHelper return value for {:#018x}",
                d.to_bits()
            );
            assert_eq!(
                snap(nc),
                snap(nr),
                "SetNumberHelper node state for {:#018x}",
                d.to_bits()
            );
            assert_eq!(print_and_take(&c, nc), print_and_take(&r, nr));
            (c.cJSON_Delete)(nc);
            (r.cJSON_Delete)(nr);
        }
        // on non-Number items too (the C function does not check the type)
        for spec in [
            Spec::Null,
            Spec::True,
            Spec::Str(b"s".to_vec()),
            Spec::Arr(vec![]),
            Spec::Obj(vec![]),
            Spec::Raw(b"1".to_vec()),
        ] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            for d in [0.0, 1.5, -1.5, f64::NAN, f64::INFINITY, 1e300] {
                assert_eq!(
                    (c.cJSON_SetNumberHelper)(bc.root, d).to_bits(),
                    (r.cJSON_SetNumberHelper)(br.root, d).to_bits(),
                    "SetNumberHelper on {spec:?} with {d:?}"
                );
                assert_eq!(snap(bc.root), snap(br.root), "state after SetNumberHelper");
            }
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// row 64 — cJSON_SetValuestring
// ---------------------------------------------------------------------------
#[test]
fn cfg64_set_valuestring() {
    let (c, r) = both();
    let olds: Vec<&[u8]> = vec![b"", b"a", b"abc", b"abcdefghij", b"\t\"\\"];
    let news: Vec<&[u8]> = vec![
        b"",
        b"a",
        b"ab",
        b"abc",
        b"abcd",
        b"abcdefghij",
        b"abcdefghijk",
        b"much much longer replacement value",
        b"\n\r\t",
        b"\x80\xff",
    ];
    unsafe {
        for old in &olds {
            for new in &news {
                let ob = Bytes::new(old);
                let nb = Bytes::new(new);
                let sc = (c.cJSON_CreateString)(ob.as_ptr());
                let sr = (r.cJSON_CreateString)(ob.as_ptr());
                let rc = (c.cJSON_SetValuestring)(sc, nb.as_ptr());
                let rr = (r.cJSON_SetValuestring)(sr, nb.as_ptr());
                assert_eq!(
                    rc.is_null(),
                    rr.is_null(),
                    "SetValuestring nullness old={old:?} new={new:?}"
                );
                assert_eq!(
                    cstr(rc),
                    cstr(rr),
                    "SetValuestring return contents old={old:?} new={new:?}"
                );
                assert_eq!(
                    snap(sc),
                    snap(sr),
                    "SetValuestring node state old={old:?} new={new:?}"
                );
                assert_eq!(print_and_take(&c, sc), print_and_take(&r, sr));
                (c.cJSON_Delete)(sc);
                (r.cJSON_Delete)(sr);
            }
        }

        // on every other item type (must be rejected identically)
        for spec in [
            Spec::Null,
            Spec::True,
            Spec::False,
            Spec::Num(1.0),
            Spec::Raw(b"1".to_vec()),
            Spec::Arr(vec![]),
            Spec::Obj(vec![]),
            Spec::StrRef(b"reference".to_vec()),
        ] {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let nb = Bytes::new(b"replacement");
            let rc = (c.cJSON_SetValuestring)(bc.root, nb.as_ptr());
            let rr = (r.cJSON_SetValuestring)(br.root, nb.as_ptr());
            assert_eq!(
                rc.is_null(),
                rr.is_null(),
                "SetValuestring on {spec:?} nullness"
            );
            assert_eq!(cstr(rc), cstr(rr), "SetValuestring on {spec:?} contents");
            assert_eq!(snap(bc.root), snap(br.root), "state after SetValuestring on {spec:?}");
            bc.delete();
            br.delete();
        }
    }
}

// ---------------------------------------------------------------------------
// row 65 — cJSON_Delete over sibling chains and reference items
// ---------------------------------------------------------------------------
#[test]
fn cfg65_delete_sibling_chains() {
    let (c, r) = both();
    // Delete on a detached sibling chain walks `->next`: build an array, detach
    // its child chain wholesale and delete the chain head.
    for n in 1..8usize {
        let spec = Spec::Arr(
            (0..n)
                .map(|i| {
                    if i % 3 == 0 {
                        Spec::Str(format!("s{i}").into_bytes())
                    } else if i % 3 == 1 {
                        Spec::Arr(vec![Spec::Num(i as f64)])
                    } else {
                        Spec::Obj(vec![(format!("k{i}").into_bytes(), Spec::Num(i as f64))])
                    }
                })
                .collect(),
        );
        unsafe {
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let hc = (*bc.root).child;
            let hr = (*br.root).child;
            (*bc.root).child = std::ptr::null_mut();
            (*br.root).child = std::ptr::null_mut();
            // the two chains must have identical shape before deletion
            assert_eq!(snap(hc), snap(hr), "detached chain shape n={n}");
            (c.cJSON_Delete)(hc);
            (r.cJSON_Delete)(hr);
            bc.delete();
            br.delete();
        }
    }

    // Reference items: cJSON_Delete must NOT free the referenced payload.
    unsafe {
        // string reference
        let buf = Bytes::new(b"externally owned");
        let sc = (c.cJSON_CreateStringReference)(buf.as_ptr());
        let sr = (r.cJSON_CreateStringReference)(buf.as_ptr());
        assert_eq!(snap(sc), snap(sr));
        (c.cJSON_Delete)(sc);
        (r.cJSON_Delete)(sr);
        // the caller's buffer is still intact
        assert_eq!(cstr(buf.as_ptr()), Some(b"externally owned".to_vec()));

        // array/object reference over a live subtree
        for as_object in [false, true] {
            let spec = Spec::Arr(vec![Spec::Num(1.0), Spec::Num(2.0)]);
            let bc = build(&c, &spec);
            let br = build(&r, &spec);
            let refc = if as_object {
                (c.cJSON_CreateObjectReference)(bc.root)
            } else {
                (c.cJSON_CreateArrayReference)(bc.root)
            };
            let refr = if as_object {
                (r.cJSON_CreateObjectReference)(br.root)
            } else {
                (r.cJSON_CreateArrayReference)(br.root)
            };
            assert_eq!(snap(refc), snap(refr));
            (c.cJSON_Delete)(refc);
            (r.cJSON_Delete)(refr);
            // the referenced tree survives and still prints the same
            assert_eq!(
                print_and_take(&c, bc.root),
                print_and_take(&r, br.root),
                "referenced tree survives Delete (as_object={as_object})"
            );
            bc.delete();
            br.delete();
        }
    }
}
