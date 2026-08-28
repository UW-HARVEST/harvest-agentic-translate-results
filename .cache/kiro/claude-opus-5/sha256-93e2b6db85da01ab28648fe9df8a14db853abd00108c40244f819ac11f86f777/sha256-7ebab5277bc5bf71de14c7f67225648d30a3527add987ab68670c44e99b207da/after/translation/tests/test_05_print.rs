//! Printing: `cJSON_Print`, `cJSON_PrintUnformatted`, `cJSON_PrintBuffered`,
//! `cJSON_PrintPreallocated`, including buffer-growth and out-of-space paths.
mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

/// Documents parsed with each library to obtain two identical trees.
pub fn documents() -> Vec<String> {
    let mut v: Vec<String> = vec![
        "null".into(),
        "true".into(),
        "false".into(),
        "0".into(),
        "-1.5".into(),
        "1e300".into(),
        "\"\"".into(),
        "\"simple\"".into(),
        "\"esc \\\" \\\\ \\b \\f \\n \\r \\t\"".into(),
        "\"\\u0001\\u001f\\u007f\"".into(),
        "\"\\ud83d\\ude00\"".into(),
        "[]".into(),
        "{}".into(),
        "[1,2,3]".into(),
        "[[],[],{}]".into(),
        "{\"a\":1}".into(),
        "{\"a\":[1,{\"b\":null}],\"c\":\"d\"}".into(),
        "[null,true,false,0,\"\",[],{}]".into(),
        r#"{"name":"Jack (\"Bee\") Nimble","format":{"type":"rect","width":1920,"height":1080,"interlace":false,"frame rate":24}}"#.into(),
        r#"[{"precision":"zip","Latitude":37.7668,"Longitude":-122.3959,"Address":"","City":"SAN FRANCISCO","State":"CA","Zip":"94107","Country":"US"}]"#.into(),
    ];
    // a wide array and a wide object to force many reallocations
    let wide: Vec<String> = (0..200).map(|i| i.to_string()).collect();
    v.push(format!("[{}]", wide.join(",")));
    let obj: Vec<String> = (0..200).map(|i| format!("\"k{i}\":{i}")).collect();
    v.push(format!("{{{}}}", obj.join(",")));
    // deep nesting (still below CJSON_NESTING_LIMIT)
    let mut deep = String::new();
    for _ in 0..300 {
        deep.push('[');
    }
    deep.push('1');
    for _ in 0..300 {
        deep.push(']');
    }
    v.push(deep);
    // long strings
    v.push(format!("\"{}\"", "x".repeat(1000)));
    v.push(format!("\"{}\"", "\\n".repeat(300)));
    v
}

struct Trees {
    c: *mut cJSON,
    r: *mut cJSON,
}

impl Trees {
    unsafe fn parse(doc: &str) -> Option<Trees> {
        let a = apis();
        let s = CString::new(doc).unwrap();
        unsafe {
            let c = a.c.cJSON_Parse(s.as_ptr());
            let r = a.rust.cJSON_Parse(s.as_ptr());
            assert_eq!(c.is_null(), r.is_null(), "parse({doc:?})");
            if c.is_null() {
                return None;
            }
            Some(Trees { c, r })
        }
    }
}

impl Drop for Trees {
    fn drop(&mut self) {
        let a = apis();
        unsafe {
            a.c.cJSON_Delete(self.c);
            a.rust.cJSON_Delete(self.r);
        }
    }
}

#[test]
fn print_documents() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in documents() {
            let Some(t) = Trees::parse(&doc) else { continue };
            let (cf, cu) = print_both(&a.c, t.c);
            let (rf, ru) = print_both(&a.rust, t.r);
            assert_eq!(cf, rf, "Print({doc:.60?})");
            assert_eq!(cu, ru, "PrintUnformatted({doc:.60?})");
            assert!(cf.is_some(), "C failed to print {doc:.60?}");
        }
    }
}

#[test]
fn print_null_item() {
    let _guard = serial();
    let a = apis();
    unsafe {
        assert_eq!(
            a.c.cJSON_Print(std::ptr::null()).is_null(),
            a.rust.cJSON_Print(std::ptr::null()).is_null()
        );
        assert_eq!(
            a.c.cJSON_PrintUnformatted(std::ptr::null()).is_null(),
            a.rust.cJSON_PrintUnformatted(std::ptr::null()).is_null()
        );
        for pre in [0, 1, 16, 256] {
            let cp = a.c.cJSON_PrintBuffered(std::ptr::null(), pre, 1);
            let rp = a.rust.cJSON_PrintBuffered(std::ptr::null(), pre, 1);
            assert_eq!(cp.is_null(), rp.is_null(), "PrintBuffered(NULL,{pre})");
            a.c.cJSON_free(cp as *mut c_void);
            a.rust.cJSON_free(rp as *mut c_void);
        }
        let mut cbuf = [0u8; 64];
        let mut rbuf = [0u8; 64];
        assert_eq!(
            a.c.cJSON_PrintPreallocated(
                std::ptr::null_mut(),
                cbuf.as_mut_ptr() as *mut c_char,
                64,
                1
            ),
            a.rust.cJSON_PrintPreallocated(
                std::ptr::null_mut(),
                rbuf.as_mut_ptr() as *mut c_char,
                64,
                1
            )
        );
        assert_eq!(cbuf, rbuf);
        // NULL buffer / negative length
        let mut item_c = a.c.cJSON_CreateNumber(1.0);
        let mut item_r = a.rust.cJSON_CreateNumber(1.0);
        assert_eq!(
            a.c.cJSON_PrintPreallocated(item_c, std::ptr::null_mut(), 10, 1),
            a.rust
                .cJSON_PrintPreallocated(item_r, std::ptr::null_mut(), 10, 1)
        );
        assert_eq!(
            a.c.cJSON_PrintPreallocated(item_c, cbuf.as_mut_ptr() as *mut c_char, -1, 1),
            a.rust
                .cJSON_PrintPreallocated(item_r, rbuf.as_mut_ptr() as *mut c_char, -1, 1)
        );
        a.c.cJSON_Delete(item_c);
        a.rust.cJSON_Delete(item_r);
        item_c = std::ptr::null_mut();
        item_r = std::ptr::null_mut();
        let _ = (item_c, item_r);
    }
}

#[test]
fn print_buffered_prebuffer_sizes() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in documents() {
            let Some(t) = Trees::parse(&doc) else { continue };
            for fmt in [0, 1] {
                for pre in [0, 1, 2, 3, 5, 8, 16, 31, 32, 64, 255, 256, 257, 4096] {
                    let cp = a.c.cJSON_PrintBuffered(t.c, pre, fmt);
                    let rp = a.rust.cJSON_PrintBuffered(t.r, pre, fmt);
                    let ctx = format!("PrintBuffered({doc:.40?}, pre={pre}, fmt={fmt})");
                    assert_eq!(cp.is_null(), rp.is_null(), "{ctx}: nullness");
                    if !cp.is_null() {
                        assert_eq!(cstr_bytes(cp), cstr_bytes(rp), "{ctx}: content");
                    }
                    a.c.cJSON_free(cp as *mut c_void);
                    a.rust.cJSON_free(rp as *mut c_void);
                }
                // negative prebuffer
                let cp = a.c.cJSON_PrintBuffered(t.c, -1, fmt);
                let rp = a.rust.cJSON_PrintBuffered(t.r, -1, fmt);
                assert_eq!(cp.is_null(), rp.is_null(), "PrintBuffered(-1)");
                assert!(cp.is_null());
            }
        }
    }
}

/// `cJSON_PrintPreallocated` with every buffer length from 0 up to a little
/// past the exact required size.  Both the boolean result and the full buffer
/// contents must match byte-for-byte.
#[test]
fn print_preallocated_all_lengths() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in documents() {
            let Some(t) = Trees::parse(&doc) else { continue };
            for fmt in [0, 1] {
                let printed = if fmt == 1 {
                    a.c.cJSON_Print(t.c)
                } else {
                    a.c.cJSON_PrintUnformatted(t.c)
                };
                let needed = cstr_bytes(printed).unwrap().len();
                a.c.cJSON_free(printed as *mut c_void);
                if needed > 4096 {
                    // keep the test fast: only probe interesting lengths
                    let cap = needed + 8;
                    for len in [0, 1, needed - 1, needed, needed + 1, needed + 5] {
                        check_prealloc(&doc, &t, fmt, len, cap);
                    }
                    continue;
                }
                let cap = needed + 8;
                for len in 0..=(needed + 6) {
                    check_prealloc(&doc, &t, fmt, len, cap);
                }
            }
        }
    }
}

unsafe fn check_prealloc(doc: &str, t: &Trees, fmt: c_int, len: usize, cap: usize) {
    let a = apis();
    unsafe {
        let mut cbuf = vec![0x5Au8; cap];
        let mut rbuf = vec![0x5Au8; cap];
        let cr = a.c.cJSON_PrintPreallocated(
            t.c,
            cbuf.as_mut_ptr() as *mut c_char,
            len as c_int,
            fmt,
        );
        let rr = a.rust.cJSON_PrintPreallocated(
            t.r,
            rbuf.as_mut_ptr() as *mut c_char,
            len as c_int,
            fmt,
        );
        let ctx = format!("PrintPreallocated({doc:.40?}, len={len}, fmt={fmt})");
        assert_eq!(cr, rr, "{ctx}: return value");
        assert_eq!(
            cbuf,
            rbuf,
            "{ctx}: buffer contents\nC:    {:?}\nRust: {:?}",
            String::from_utf8_lossy(&cbuf),
            String::from_utf8_lossy(&rbuf)
        );
    }
}

/// Items that cannot be produced by the parser: raw values, references,
/// invalid types, and strings with a NULL `valuestring`.
#[test]
fn print_synthetic_items() {
    let _guard = serial();
    let a = apis();
    unsafe {
        // raw
        for raw in ["1234", "{\"already\":\"json\"}", "", "not json"] {
            let s = CString::new(raw).unwrap();
            let cp = a.c.cJSON_CreateRaw(s.as_ptr());
            let rp = a.rust.cJSON_CreateRaw(s.as_ptr());
            assert_tree_eq(&format!("raw {raw:?}"), cp, rp);
            a.c.cJSON_Delete(cp);
            a.rust.cJSON_Delete(rp);
        }

        // array containing a raw item and references
        let cbase = a.c.cJSON_CreateArray();
        let rbase = a.rust.cJSON_CreateArray();
        let one = CString::new("1").unwrap();
        a.c.cJSON_AddItemToArray(cbase, a.c.cJSON_CreateRaw(one.as_ptr()));
        a.rust
            .cJSON_AddItemToArray(rbase, a.rust.cJSON_CreateRaw(one.as_ptr()));
        let inner_c = a.c.cJSON_CreateNumber(5.0);
        let inner_r = a.rust.cJSON_CreateNumber(5.0);
        a.c.cJSON_AddItemToArray(cbase, inner_c);
        a.rust.cJSON_AddItemToArray(rbase, inner_r);
        a.c.cJSON_AddItemReferenceToArray(cbase, inner_c);
        a.rust.cJSON_AddItemReferenceToArray(rbase, inner_r);
        assert_tree_eq("array with raw + reference", cbase, rbase);
        a.c.cJSON_Delete(cbase);
        a.rust.cJSON_Delete(rbase);

        // string item whose valuestring is NULL, and raw item with NULL
        for type_ in [cJSON_String, cJSON_Raw] {
            let mut ci = cJSON {
                next: std::ptr::null_mut(),
                prev: std::ptr::null_mut(),
                child: std::ptr::null_mut(),
                type_,
                valuestring: std::ptr::null_mut(),
                valueint: 0,
                valuedouble: 0.0,
                string: std::ptr::null_mut(),
            };
            let p: *mut cJSON = &mut ci;
            let cp = a.c.cJSON_Print(p);
            let rp = a.rust.cJSON_Print(p);
            assert_eq!(
                cstr_bytes(cp),
                cstr_bytes(rp),
                "Print(type={type_}, valuestring=NULL)"
            );
            a.c.cJSON_free(cp as *mut c_void);
            a.rust.cJSON_free(rp as *mut c_void);
        }

        // invalid / unknown types
        for type_ in [cJSON_Invalid, 0x80 | 0x40, 1 << 9, 0x7FFF_FFFF, -1] {
            let mut ci = cJSON {
                next: std::ptr::null_mut(),
                prev: std::ptr::null_mut(),
                child: std::ptr::null_mut(),
                type_,
                valuestring: std::ptr::null_mut(),
                valueint: 0,
                valuedouble: 0.0,
                string: std::ptr::null_mut(),
            };
            let p: *mut cJSON = &mut ci;
            let cp = a.c.cJSON_Print(p);
            let rp = a.rust.cJSON_Print(p);
            assert_eq!(cp.is_null(), rp.is_null(), "Print(type={type_}) nullness");
            assert_eq!(cstr_bytes(cp), cstr_bytes(rp), "Print(type={type_})");
            a.c.cJSON_free(cp as *mut c_void);
            a.rust.cJSON_free(rp as *mut c_void);
        }
    }
}

/// Every single byte value inside a string, to exercise the escaping table.
#[test]
fn print_all_byte_values() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for b in 1u8..=255 {
            let buf = [b, 0u8];
            let cs_item = a.c.cJSON_CreateString(buf.as_ptr() as *const c_char);
            let rs_item = a.rust.cJSON_CreateString(buf.as_ptr() as *const c_char);
            assert_tree_eq(&format!("string byte {b:#04x}"), cs_item, rs_item);
            a.c.cJSON_Delete(cs_item);
            a.rust.cJSON_Delete(rs_item);
        }
        // all bytes at once
        let mut all: Vec<u8> = (1u8..=255).collect();
        all.push(0);
        let cs_item = a.c.cJSON_CreateString(all.as_ptr() as *const c_char);
        let rs_item = a.rust.cJSON_CreateString(all.as_ptr() as *const c_char);
        assert_tree_eq("string with all byte values", cs_item, rs_item);
        a.c.cJSON_Delete(cs_item);
        a.rust.cJSON_Delete(rs_item);
    }
}
