//! Phase B — rows C85..C88: `cJSON_Minify`, and the composed pipelines
//! (minify→parse, parse→mutate→print, print→parse round trip).
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_int;
use std::fmt::Write as _;

/// Buffer with 8 NUL bytes of readable padding after the content: `minify_string`
/// and `skip_multiline_comment` legitimately peek one byte past the terminator.
fn padded(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.extend_from_slice(&[0u8; 8]);
    v
}

fn minify_cases() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b" ".to_vec(),
        b"\t\r\n ".to_vec(),
        b"{ \"a\" : 1 }".to_vec(),
        b"[ 1 , 2 ]".to_vec(),
        b"// comment".to_vec(),
        b"// comment\n1".to_vec(),
        b"1 // trailing".to_vec(),
        b"/* block */1".to_vec(),
        b"/* unterminated".to_vec(),
        b"/*".to_vec(),
        b"/".to_vec(),
        b"1/2".to_vec(),
        b"/x/".to_vec(),
        b"//".to_vec(),
        b"//\n".to_vec(),
        b"/**/".to_vec(),
        b"/*/".to_vec(),
        b"/*//*/".to_vec(),
        b"\"a string\"".to_vec(),
        b"\"with // comment\"".to_vec(),
        b"\"with /* comment */\"".to_vec(),
        b"\"with \\\" quote\"".to_vec(),
        b"\"with \\\\ backslash\"".to_vec(),
        b"\"unterminated".to_vec(),
        b"\"unterminated\\".to_vec(),
        b"\"\\\"".to_vec(),
        b"\"a\" \"b\"".to_vec(),
        b"{\"a\":\"  spaces  \"}".to_vec(),
        b"{\n\t\"a\" : [ 1 , 2 , { \"b\" : \"c\" } ] // x\n}".to_vec(),
        b"[\"\\\\\",\"\\\"\"]".to_vec(),
        b"\"\\\\\\\"\"".to_vec(),
        b"\t/*a*/ \"s\" /*b*/ \t".to_vec(),
    ];
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    for _ in 0..200 {
        v.push(gen_minify_input(&mut rng));
    }
    v
}

#[test]
fn c85_minify() {
    diff("C85 cJSON_Minify", |api| unsafe {
        let mut log = String::new();
        for case in minify_cases() {
            let mut buf = padded(&case);
            (api.cJSON_Minify)(buf.as_mut_ptr() as *mut i8);
            let _ = writeln!(log, "in={}\n  whole_buffer={}", show(&case), show(&buf));
            let nul = buf.iter().position(|&b| b == 0).unwrap();
            let _ = writeln!(log, "  result={}", show(&buf[..nul]));
        }
        log
    });
}

#[test]
fn c86_minify_then_parse() {
    diff("C86 cJSON_Minify -> cJSON_Parse", |api| unsafe {
        let mut log = String::new();
        for case in minify_cases() {
            let mut buf = padded(&case);
            (api.cJSON_Minify)(buf.as_mut_ptr() as *mut i8);
            let root = (api.cJSON_Parse)(buf.as_ptr() as *const i8);
            let _ = writeln!(
                log,
                "in={} minified={} null={}",
                show(&case),
                show(&buf[..buf.iter().position(|&b| b == 0).unwrap()]),
                root.is_null()
            );
            let _ = write!(log, "{}", dump(root));
            let p = take_print(api, (api.cJSON_PrintUnformatted)(root));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(root);
        }
        log
    });
}

/// Row C87 — parse, then a deterministic pseudo-random sequence of mutations
/// through the low-level API, printing and dumping after every step.
#[test]
fn c87_parse_mutate_print_pipeline() {
    diff("C87 parse -> mutate -> print pipeline", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0xF00D_BA5E_BA11_0000);
        // `cJSON_AddItemToObjectCS` stores the key pointer *without copying it*,
        // so the keys must outlive every item that references them.
        let keys: &'static [std::ffi::CString] = Box::leak(Box::new([
            cs("a"),
            cs("A"),
            cs("b"),
            cs(""),
            cs("missing"),
            cs("added"),
        ])) as &'static [std::ffi::CString; 6];
        for round in 0..120 {
            let text = gen_json(&mut rng);
            let buf = CBuf::new(&text);
            let root = (api.cJSON_Parse)(buf.ptr());
            let _ = writeln!(log, "== round {round} src={}", show(&text));
            if root.is_null() {
                let _ = writeln!(log, "  parse failed");
                continue;
            }
            // a side item that lives for the whole round: safe reference target
            let side = (api.cJSON_CreateString)(cs("side").as_ptr());
            for step in 0..10 {
                let op = rng.below(14);
                let idx = rng.range_i32(-1, 6);
                let key = &keys[rng.below(keys.len())];
                let num = rng.nice_f64();
                let sval = cs("mutated");
                match op {
                    0 => {
                        let rc = (api.cJSON_AddItemToArray)(root, (api.cJSON_CreateNumber)(num));
                        let _ = writeln!(log, "  {step}: AddItemToArray -> {rc}");
                    }
                    1 => {
                        let rc = (api.cJSON_AddItemToObject)(
                            root,
                            key.as_ptr(),
                            (api.cJSON_CreateString)(sval.as_ptr()),
                        );
                        let _ = writeln!(log, "  {step}: AddItemToObject -> {rc}");
                    }
                    2 => {
                        let rc = (api.cJSON_AddItemToObjectCS)(
                            root,
                            key.as_ptr(),
                            (api.cJSON_CreateNumber)(num),
                        );
                        let _ = writeln!(log, "  {step}: AddItemToObjectCS -> {rc}");
                    }
                    3 => {
                        let item = (api.cJSON_CreateNumber)(num);
                        let rc = (api.cJSON_InsertItemInArray)(root, idx, item);
                        if rc == 0 {
                            (api.cJSON_Delete)(item);
                        }
                        let _ = writeln!(log, "  {step}: InsertItemInArray({idx}) -> {rc}");
                    }
                    4 => {
                        let item = (api.cJSON_CreateNumber)(num);
                        let rc = (api.cJSON_ReplaceItemInArray)(root, idx, item);
                        if rc == 0 {
                            (api.cJSON_Delete)(item);
                        }
                        let _ = writeln!(log, "  {step}: ReplaceItemInArray({idx}) -> {rc}");
                    }
                    5 => {
                        let item = (api.cJSON_CreateNumber)(num);
                        let rc = (api.cJSON_ReplaceItemInObject)(root, key.as_ptr(), item);
                        if rc == 0 {
                            (api.cJSON_Delete)(item);
                        }
                        let _ = writeln!(log, "  {step}: ReplaceItemInObject -> {rc}");
                    }
                    6 => {
                        let d = (api.cJSON_DetachItemFromArray)(root, idx);
                        let _ = writeln!(log, "  {step}: DetachItemFromArray({idx}) -> {}", !d.is_null());
                        let _ = write!(log, "     detached: {}", dump(d));
                        (api.cJSON_Delete)(d);
                    }
                    7 => {
                        (api.cJSON_DeleteItemFromArray)(root, idx);
                        let _ = writeln!(log, "  {step}: DeleteItemFromArray({idx})");
                    }
                    8 => {
                        (api.cJSON_DeleteItemFromObjectCaseSensitive)(root, key.as_ptr());
                        let _ = writeln!(log, "  {step}: DeleteItemFromObjectCaseSensitive");
                    }
                    9 => {
                        let target = (api.cJSON_GetArrayItem)(root, idx);
                        let d = (api.cJSON_DetachItemViaPointer)(root, target);
                        let _ = writeln!(log, "  {step}: DetachItemViaPointer({idx}) -> {}", !d.is_null());
                        (api.cJSON_Delete)(d);
                    }
                    10 => {
                        let target = (api.cJSON_GetArrayItem)(root, idx);
                        if !target.is_null() {
                            let ret = (api.cJSON_SetValuestring)(target, sval.as_ptr());
                            let _ = writeln!(
                                log,
                                "  {step}: SetValuestring({idx}) -> null={} now={:?}",
                                ret.is_null(),
                                read_cstr((*target).valuestring).map(|v| show(&v))
                            );
                        } else {
                            let _ = writeln!(log, "  {step}: SetValuestring({idx}) -> no target");
                        }
                    }
                    11 => {
                        let target = (api.cJSON_GetArrayItem)(root, idx);
                        if !target.is_null() {
                            let ret = (api.cJSON_SetNumberHelper)(target, num);
                            let _ = writeln!(
                                log,
                                "  {step}: SetNumberHelper({idx}) -> 0x{:016x} int={}",
                                ret.to_bits(),
                                (*target).valueint
                            );
                        } else {
                            let _ = writeln!(log, "  {step}: SetNumberHelper({idx}) -> no target");
                        }
                    }
                    12 => {
                        let rc = (api.cJSON_AddItemReferenceToArray)(root, side);
                        let _ = writeln!(log, "  {step}: AddItemReferenceToArray(side) -> {rc}");
                    }
                    _ => {
                        let d = (api.cJSON_Duplicate)(root, 1);
                        let _ = writeln!(
                            log,
                            "  {step}: Duplicate -> cmp={} cmp0={}",
                            (api.cJSON_Compare)(root, d, 1),
                            (api.cJSON_Compare)(root, d, 0)
                        );
                        let _ = write!(log, "     dup: {}", dump(d));
                        (api.cJSON_Delete)(d);
                    }
                }
                let _ = write!(log, "     graph: {}", dump(root));
                let pf = take_print(api, (api.cJSON_Print)(root));
                let pu = take_print(api, (api.cJSON_PrintUnformatted)(root));
                let _ = writeln!(
                    log,
                    "     fmt={} unfmt={} size={}",
                    pf.map(|v| show(&v)).unwrap_or("NULL".into()),
                    pu.map(|v| show(&v)).unwrap_or("NULL".into()),
                    (api.cJSON_GetArraySize)(root)
                );
            }
            (api.cJSON_Delete)(root);
            (api.cJSON_Delete)(side);
        }
        log
    });
}

/// Row C88 — print → re-parse → print round trip.
#[test]
fn c88_round_trip() {
    diff("C88 print -> parse round trip", |api| unsafe {
        let mut log = String::new();
        let mut rng = Rng::new(0xBEEF_CAFE_D00D_1234);
        for round in 0..200 {
            let text = gen_json(&mut rng);
            let buf = CBuf::new(&text);
            let root = (api.cJSON_Parse)(buf.ptr());
            if root.is_null() {
                let _ = writeln!(log, "{round}: parse failed src={}", show(&text));
                continue;
            }
            for fmt in [0i32, 1] {
                let printed = if fmt == 1 {
                    take_print(api, (api.cJSON_Print)(root))
                } else {
                    take_print(api, (api.cJSON_PrintUnformatted)(root))
                };
                let bytes = printed.unwrap_or_default();
                let b2 = CBuf::new(&bytes);
                let again = (api.cJSON_Parse)(b2.ptr());
                let printed2 = if fmt == 1 {
                    take_print(api, (api.cJSON_Print)(again))
                } else {
                    take_print(api, (api.cJSON_PrintUnformatted)(again))
                };
                let _ = writeln!(
                    log,
                    "{round} fmt={fmt}: first={} second={} stable={} compare={}",
                    show(&bytes),
                    printed2.as_ref().map(|v| show(v)).unwrap_or("NULL".into()),
                    printed2.as_deref() == Some(&bytes[..]),
                    (api.cJSON_Compare)(root, again, 1)
                );
                let _ = write!(log, "  {}", dump(again));
                (api.cJSON_Delete)(again);
            }
            (api.cJSON_Delete)(root);
        }
        log
    });
}

#[allow(unused)]
fn _keep(_: c_int) {}
