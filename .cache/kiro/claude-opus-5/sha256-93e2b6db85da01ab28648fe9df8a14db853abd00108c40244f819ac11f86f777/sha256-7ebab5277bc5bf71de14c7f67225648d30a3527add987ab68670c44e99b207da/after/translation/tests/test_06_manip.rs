//! Tree manipulation: getters, add/detach/delete/insert/replace, duplicate,
//! compare, minify, and the value setters.
mod common;

use common::*;
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_void};

pub struct Trees {
    pub c: *mut cJSON,
    pub r: *mut cJSON,
}

impl Trees {
    pub unsafe fn parse(doc: &str) -> Trees {
        let a = apis();
        let s = CString::new(doc).unwrap();
        unsafe {
            let c = a.c.cJSON_Parse(s.as_ptr());
            let r = a.rust.cJSON_Parse(s.as_ptr());
            assert_eq!(c.is_null(), r.is_null(), "parse({doc:?})");
            assert!(!c.is_null(), "corpus document failed to parse: {doc:?}");
            Trees { c, r }
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

pub const DOCS: &[&str] = &[
    "[]",
    "[1]",
    "[1,2,3]",
    "[null,true,false,\"s\",[1],{\"a\":2}]",
    "{}",
    "{\"a\":1}",
    "{\"a\":1,\"b\":2,\"c\":3}",
    "{\"A\":1,\"a\":2,\"B\":3}",
    "{\"nested\":{\"deep\":{\"deeper\":[1,2,3]}}}",
    "\"scalar\"",
    "42",
    "null",
];

fn names() -> Vec<&'static str> {
    vec!["a", "A", "b", "B", "c", "missing", "", "nested", "NESTED", "deep"]
}

#[test]
fn array_getters() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            let t = Trees::parse(doc);
            assert_eq!(
                a.c.cJSON_GetArraySize(t.c),
                a.rust.cJSON_GetArraySize(t.r),
                "GetArraySize({doc})"
            );
            for i in -3..8 {
                let ci = a.c.cJSON_GetArrayItem(t.c, i);
                let ri = a.rust.cJSON_GetArrayItem(t.r, i);
                assert_eq!(ci.is_null(), ri.is_null(), "GetArrayItem({doc},{i})");
                if !ci.is_null() {
                    assert_eq!(dump(ci), dump(ri), "GetArrayItem({doc},{i}) contents");
                }
            }
            assert_eq!(
                a.c.cJSON_GetArrayItem(t.c, c_int::MAX).is_null(),
                a.rust.cJSON_GetArrayItem(t.r, c_int::MAX).is_null()
            );
            assert_eq!(
                a.c.cJSON_GetArrayItem(t.c, c_int::MIN).is_null(),
                a.rust.cJSON_GetArrayItem(t.r, c_int::MIN).is_null()
            );
        }
        assert_eq!(
            a.c.cJSON_GetArraySize(std::ptr::null()),
            a.rust.cJSON_GetArraySize(std::ptr::null())
        );
        assert_eq!(
            a.c.cJSON_GetArrayItem(std::ptr::null(), 0).is_null(),
            a.rust.cJSON_GetArrayItem(std::ptr::null(), 0).is_null()
        );
    }
}

#[test]
fn object_getters() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            let t = Trees::parse(doc);
            for n in names() {
                let name = CString::new(n).unwrap();
                let ci = a.c.cJSON_GetObjectItem(t.c, name.as_ptr());
                let ri = a.rust.cJSON_GetObjectItem(t.r, name.as_ptr());
                assert_eq!(ci.is_null(), ri.is_null(), "GetObjectItem({doc},{n})");
                if !ci.is_null() {
                    assert_eq!(dump(ci), dump(ri), "GetObjectItem({doc},{n})");
                }

                let ci = a.c.cJSON_GetObjectItemCaseSensitive(t.c, name.as_ptr());
                let ri = a.rust.cJSON_GetObjectItemCaseSensitive(t.r, name.as_ptr());
                assert_eq!(ci.is_null(), ri.is_null(), "GetObjectItemCS({doc},{n})");
                if !ci.is_null() {
                    assert_eq!(dump(ci), dump(ri), "GetObjectItemCS({doc},{n})");
                }

                assert_eq!(
                    a.c.cJSON_HasObjectItem(t.c, name.as_ptr()),
                    a.rust.cJSON_HasObjectItem(t.r, name.as_ptr()),
                    "HasObjectItem({doc},{n})"
                );
            }
            // NULL name
            assert_eq!(
                a.c.cJSON_GetObjectItem(t.c, std::ptr::null()).is_null(),
                a.rust.cJSON_GetObjectItem(t.r, std::ptr::null()).is_null()
            );
            assert_eq!(
                a.c.cJSON_HasObjectItem(t.c, std::ptr::null()),
                a.rust.cJSON_HasObjectItem(t.r, std::ptr::null())
            );
        }
        let name = CString::new("a").unwrap();
        assert_eq!(
            a.c.cJSON_GetObjectItem(std::ptr::null(), name.as_ptr())
                .is_null(),
            a.rust
                .cJSON_GetObjectItem(std::ptr::null(), name.as_ptr())
                .is_null()
        );
        assert_eq!(
            a.c.cJSON_HasObjectItem(std::ptr::null(), name.as_ptr()),
            a.rust.cJSON_HasObjectItem(std::ptr::null(), name.as_ptr())
        );
    }
}

#[test]
fn add_items() {
    let _guard = serial();
    let a = apis();
    unsafe {
        // AddItemToArray on arrays, objects and scalars
        for doc in DOCS {
            let t = Trees::parse(doc);
            let cr = a.c.cJSON_AddItemToArray(t.c, a.c.cJSON_CreateNumber(9.0));
            let rr = a
                .rust
                .cJSON_AddItemToArray(t.r, a.rust.cJSON_CreateNumber(9.0));
            assert_eq!(cr, rr, "AddItemToArray({doc}) result");
            assert_tree_eq(&format!("AddItemToArray({doc})"), t.c, t.r);

            let key = CString::new("newkey").unwrap();
            let cr = a
                .c
                .cJSON_AddItemToObject(t.c, key.as_ptr(), a.c.cJSON_CreateTrue());
            let rr = a
                .rust
                .cJSON_AddItemToObject(t.r, key.as_ptr(), a.rust.cJSON_CreateTrue());
            assert_eq!(cr, rr, "AddItemToObject({doc}) result");
            assert_tree_eq(&format!("AddItemToObject({doc})"), t.c, t.r);

            let cr = a
                .c
                .cJSON_AddItemToObjectCS(t.c, key.as_ptr(), a.c.cJSON_CreateFalse());
            let rr = a
                .rust
                .cJSON_AddItemToObjectCS(t.r, key.as_ptr(), a.rust.cJSON_CreateFalse());
            assert_eq!(cr, rr, "AddItemToObjectCS({doc}) result");
            assert_tree_eq(&format!("AddItemToObjectCS({doc})"), t.c, t.r);
        }

        // NULL arguments
        let t = Trees::parse("[]");
        assert_eq!(
            a.c.cJSON_AddItemToArray(t.c, std::ptr::null_mut()),
            a.rust.cJSON_AddItemToArray(t.r, std::ptr::null_mut())
        );
        assert_eq!(
            a.c.cJSON_AddItemToArray(std::ptr::null_mut(), std::ptr::null_mut()),
            a.rust
                .cJSON_AddItemToArray(std::ptr::null_mut(), std::ptr::null_mut())
        );
        let key = CString::new("k").unwrap();
        assert_eq!(
            a.c.cJSON_AddItemToObject(t.c, key.as_ptr(), std::ptr::null_mut()),
            a.rust
                .cJSON_AddItemToObject(t.r, key.as_ptr(), std::ptr::null_mut())
        );
        assert_eq!(
            a.c.cJSON_AddItemToObject(t.c, std::ptr::null(), a.c.cJSON_CreateNull()),
            a.rust
                .cJSON_AddItemToObject(t.r, std::ptr::null(), a.rust.cJSON_CreateNull())
        );
        assert_tree_eq("add with NULL args", t.c, t.r);

        // self-add must be rejected the same way
        let t = Trees::parse("[1]");
        assert_eq!(
            a.c.cJSON_AddItemToArray(t.c, t.c),
            a.rust.cJSON_AddItemToArray(t.r, t.r),
            "AddItemToArray(self)"
        );
        assert_tree_eq("AddItemToArray(self)", t.c, t.r);
    }
}

#[test]
fn add_reference_items() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let t = Trees::parse("{\"arr\":[1,2]}");
        let arr = CString::new("arr").unwrap();
        let cchild = a.c.cJSON_GetObjectItem(t.c, arr.as_ptr());
        let rchild = a.rust.cJSON_GetObjectItem(t.r, arr.as_ptr());

        let cholder = a.c.cJSON_CreateArray();
        let rholder = a.rust.cJSON_CreateArray();
        assert_eq!(
            a.c.cJSON_AddItemReferenceToArray(cholder, cchild),
            a.rust.cJSON_AddItemReferenceToArray(rholder, rchild)
        );
        assert_tree_eq("AddItemReferenceToArray", cholder, rholder);

        let cobj = a.c.cJSON_CreateObject();
        let robj = a.rust.cJSON_CreateObject();
        let key = CString::new("ref").unwrap();
        assert_eq!(
            a.c.cJSON_AddItemReferenceToObject(cobj, key.as_ptr(), cchild),
            a.rust
                .cJSON_AddItemReferenceToObject(robj, key.as_ptr(), rchild)
        );
        assert_tree_eq("AddItemReferenceToObject", cobj, robj);

        // NULL handling
        assert_eq!(
            a.c.cJSON_AddItemReferenceToArray(cholder, std::ptr::null_mut()),
            a.rust
                .cJSON_AddItemReferenceToArray(rholder, std::ptr::null_mut())
        );
        assert_eq!(
            a.c.cJSON_AddItemReferenceToObject(cobj, key.as_ptr(), std::ptr::null_mut()),
            a.rust
                .cJSON_AddItemReferenceToObject(robj, key.as_ptr(), std::ptr::null_mut())
        );

        a.c.cJSON_Delete(cholder);
        a.rust.cJSON_Delete(rholder);
        a.c.cJSON_Delete(cobj);
        a.rust.cJSON_Delete(robj);
    }
}

#[test]
fn add_helpers() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let t = Trees::parse("{}");
        let n = |s: &str| CString::new(s).unwrap();

        macro_rules! pair {
            ($ctx:expr, $c:expr, $r:expr) => {{
                let cp = $c;
                let rp = $r;
                assert_eq!(cp.is_null(), rp.is_null(), concat!($ctx, ": nullness"));
                if !cp.is_null() {
                    assert_eq!(dump(cp), dump(rp), concat!($ctx, ": item"));
                }
                assert_tree_eq($ctx, t.c, t.r);
            }};
        }

        let k = n("null");
        pair!(
            "AddNullToObject",
            a.c.cJSON_AddNullToObject(t.c, k.as_ptr()),
            a.rust.cJSON_AddNullToObject(t.r, k.as_ptr())
        );
        let k = n("true");
        pair!(
            "AddTrueToObject",
            a.c.cJSON_AddTrueToObject(t.c, k.as_ptr()),
            a.rust.cJSON_AddTrueToObject(t.r, k.as_ptr())
        );
        let k = n("false");
        pair!(
            "AddFalseToObject",
            a.c.cJSON_AddFalseToObject(t.c, k.as_ptr()),
            a.rust.cJSON_AddFalseToObject(t.r, k.as_ptr())
        );
        let k = n("bool0");
        pair!(
            "AddBoolToObject(0)",
            a.c.cJSON_AddBoolToObject(t.c, k.as_ptr(), 0),
            a.rust.cJSON_AddBoolToObject(t.r, k.as_ptr(), 0)
        );
        let k = n("bool1");
        pair!(
            "AddBoolToObject(1)",
            a.c.cJSON_AddBoolToObject(t.c, k.as_ptr(), 7),
            a.rust.cJSON_AddBoolToObject(t.r, k.as_ptr(), 7)
        );
        let k = n("num");
        pair!(
            "AddNumberToObject",
            a.c.cJSON_AddNumberToObject(t.c, k.as_ptr(), 1.5),
            a.rust.cJSON_AddNumberToObject(t.r, k.as_ptr(), 1.5)
        );
        let k = n("nan");
        pair!(
            "AddNumberToObject(NaN)",
            a.c.cJSON_AddNumberToObject(t.c, k.as_ptr(), f64::NAN),
            a.rust.cJSON_AddNumberToObject(t.r, k.as_ptr(), f64::NAN)
        );
        let k = n("inf");
        pair!(
            "AddNumberToObject(inf)",
            a.c.cJSON_AddNumberToObject(t.c, k.as_ptr(), f64::INFINITY),
            a.rust.cJSON_AddNumberToObject(t.r, k.as_ptr(), f64::INFINITY)
        );
        let k = n("str");
        let v = n("value \"quoted\"");
        pair!(
            "AddStringToObject",
            a.c.cJSON_AddStringToObject(t.c, k.as_ptr(), v.as_ptr()),
            a.rust.cJSON_AddStringToObject(t.r, k.as_ptr(), v.as_ptr())
        );
        let k = n("raw");
        let v = n("[1,2]");
        pair!(
            "AddRawToObject",
            a.c.cJSON_AddRawToObject(t.c, k.as_ptr(), v.as_ptr()),
            a.rust.cJSON_AddRawToObject(t.r, k.as_ptr(), v.as_ptr())
        );
        let k = n("obj");
        pair!(
            "AddObjectToObject",
            a.c.cJSON_AddObjectToObject(t.c, k.as_ptr()),
            a.rust.cJSON_AddObjectToObject(t.r, k.as_ptr())
        );
        let k = n("arr");
        pair!(
            "AddArrayToObject",
            a.c.cJSON_AddArrayToObject(t.c, k.as_ptr()),
            a.rust.cJSON_AddArrayToObject(t.r, k.as_ptr())
        );

        // NULL object / NULL name / NULL string value
        let k = n("x");
        assert_eq!(
            a.c.cJSON_AddNullToObject(std::ptr::null_mut(), k.as_ptr())
                .is_null(),
            a.rust
                .cJSON_AddNullToObject(std::ptr::null_mut(), k.as_ptr())
                .is_null()
        );
        assert_eq!(
            a.c.cJSON_AddStringToObject(t.c, k.as_ptr(), std::ptr::null())
                .is_null(),
            a.rust
                .cJSON_AddStringToObject(t.r, k.as_ptr(), std::ptr::null())
                .is_null()
        );
        assert_eq!(
            a.c.cJSON_AddRawToObject(t.c, k.as_ptr(), std::ptr::null())
                .is_null(),
            a.rust
                .cJSON_AddRawToObject(t.r, k.as_ptr(), std::ptr::null())
                .is_null()
        );
        assert_eq!(
            a.c.cJSON_AddNumberToObject(t.c, std::ptr::null(), 1.0)
                .is_null(),
            a.rust
                .cJSON_AddNumberToObject(t.r, std::ptr::null(), 1.0)
                .is_null()
        );
        assert_tree_eq("add helpers with NULL args", t.c, t.r);
    }
}

#[test]
fn detach_and_delete() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            for which in -2..6 {
                let t = Trees::parse(doc);
                let cd = a.c.cJSON_DetachItemFromArray(t.c, which);
                let rd = a.rust.cJSON_DetachItemFromArray(t.r, which);
                let ctx = format!("DetachItemFromArray({doc},{which})");
                assert_eq!(cd.is_null(), rd.is_null(), "{ctx}: nullness");
                if !cd.is_null() {
                    assert_eq!(dump(cd), dump(rd), "{ctx}: detached");
                }
                assert_tree_eq(&ctx, t.c, t.r);
                a.c.cJSON_Delete(cd);
                a.rust.cJSON_Delete(rd);
            }
            for which in -1..5 {
                let t = Trees::parse(doc);
                a.c.cJSON_DeleteItemFromArray(t.c, which);
                a.rust.cJSON_DeleteItemFromArray(t.r, which);
                assert_tree_eq(&format!("DeleteItemFromArray({doc},{which})"), t.c, t.r);
            }
            for name in names() {
                let key = CString::new(name).unwrap();
                let t = Trees::parse(doc);
                let cd = a.c.cJSON_DetachItemFromObject(t.c, key.as_ptr());
                let rd = a.rust.cJSON_DetachItemFromObject(t.r, key.as_ptr());
                let ctx = format!("DetachItemFromObject({doc},{name})");
                assert_eq!(cd.is_null(), rd.is_null(), "{ctx}: nullness");
                if !cd.is_null() {
                    assert_eq!(dump(cd), dump(rd), "{ctx}: detached");
                }
                assert_tree_eq(&ctx, t.c, t.r);
                a.c.cJSON_Delete(cd);
                a.rust.cJSON_Delete(rd);

                let t = Trees::parse(doc);
                let cd = a
                    .c
                    .cJSON_DetachItemFromObjectCaseSensitive(t.c, key.as_ptr());
                let rd = a
                    .rust
                    .cJSON_DetachItemFromObjectCaseSensitive(t.r, key.as_ptr());
                let ctx = format!("DetachItemFromObjectCS({doc},{name})");
                assert_eq!(cd.is_null(), rd.is_null(), "{ctx}: nullness");
                if !cd.is_null() {
                    assert_eq!(dump(cd), dump(rd), "{ctx}: detached");
                }
                assert_tree_eq(&ctx, t.c, t.r);
                a.c.cJSON_Delete(cd);
                a.rust.cJSON_Delete(rd);

                let t = Trees::parse(doc);
                a.c.cJSON_DeleteItemFromObject(t.c, key.as_ptr());
                a.rust.cJSON_DeleteItemFromObject(t.r, key.as_ptr());
                assert_tree_eq(&format!("DeleteItemFromObject({doc},{name})"), t.c, t.r);

                let t = Trees::parse(doc);
                a.c.cJSON_DeleteItemFromObjectCaseSensitive(t.c, key.as_ptr());
                a.rust
                    .cJSON_DeleteItemFromObjectCaseSensitive(t.r, key.as_ptr());
                assert_tree_eq(&format!("DeleteItemFromObjectCS({doc},{name})"), t.c, t.r);
            }
        }

        // DetachItemViaPointer edge cases
        let t = Trees::parse("[1,2,3]");
        let ci = a.c.cJSON_GetArrayItem(t.c, 1);
        let ri = a.rust.cJSON_GetArrayItem(t.r, 1);
        let cd = a.c.cJSON_DetachItemViaPointer(t.c, ci);
        let rd = a.rust.cJSON_DetachItemViaPointer(t.r, ri);
        assert_eq!(cd.is_null(), rd.is_null());
        assert_eq!(dump(cd), dump(rd));
        assert_tree_eq("DetachItemViaPointer(middle)", t.c, t.r);
        a.c.cJSON_Delete(cd);
        a.rust.cJSON_Delete(rd);

        // detach an item that is not a child of the parent
        let t = Trees::parse("[1,2,3]");
        let other = Trees::parse("[9]");
        let ci = a.c.cJSON_GetArrayItem(other.c, 0);
        let ri = a.rust.cJSON_GetArrayItem(other.r, 0);
        let cd = a.c.cJSON_DetachItemViaPointer(t.c, ci);
        let rd = a.rust.cJSON_DetachItemViaPointer(t.r, ri);
        assert_eq!(cd.is_null(), rd.is_null(), "detach foreign item");
        assert_tree_eq("DetachItemViaPointer(foreign)", t.c, t.r);

        // NULL arguments
        assert_eq!(
            a.c.cJSON_DetachItemViaPointer(std::ptr::null_mut(), std::ptr::null_mut())
                .is_null(),
            a.rust
                .cJSON_DetachItemViaPointer(std::ptr::null_mut(), std::ptr::null_mut())
                .is_null()
        );
        assert_eq!(
            a.c.cJSON_DetachItemFromArray(std::ptr::null_mut(), 0)
                .is_null(),
            a.rust
                .cJSON_DetachItemFromArray(std::ptr::null_mut(), 0)
                .is_null()
        );
    }
}

#[test]
fn insert_and_replace() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for doc in DOCS {
            for which in -2..6 {
                let t = Trees::parse(doc);
                let cr = a
                    .c
                    .cJSON_InsertItemInArray(t.c, which, a.c.cJSON_CreateNumber(99.0));
                let rr = a.rust.cJSON_InsertItemInArray(
                    t.r,
                    which,
                    a.rust.cJSON_CreateNumber(99.0),
                );
                let ctx = format!("InsertItemInArray({doc},{which})");
                assert_eq!(cr, rr, "{ctx}: result");
                assert_tree_eq(&ctx, t.c, t.r);

                let t = Trees::parse(doc);
                let cr = a
                    .c
                    .cJSON_ReplaceItemInArray(t.c, which, a.c.cJSON_CreateNumber(98.0));
                let rr = a.rust.cJSON_ReplaceItemInArray(
                    t.r,
                    which,
                    a.rust.cJSON_CreateNumber(98.0),
                );
                let ctx = format!("ReplaceItemInArray({doc},{which})");
                assert_eq!(cr, rr, "{ctx}: result");
                assert_tree_eq(&ctx, t.c, t.r);
            }
            for name in names() {
                let key = CString::new(name).unwrap();
                let t = Trees::parse(doc);
                let cr = a.c.cJSON_ReplaceItemInObject(
                    t.c,
                    key.as_ptr(),
                    a.c.cJSON_CreateString(key.as_ptr()),
                );
                let rr = a.rust.cJSON_ReplaceItemInObject(
                    t.r,
                    key.as_ptr(),
                    a.rust.cJSON_CreateString(key.as_ptr()),
                );
                let ctx = format!("ReplaceItemInObject({doc},{name})");
                assert_eq!(cr, rr, "{ctx}: result");
                assert_tree_eq(&ctx, t.c, t.r);

                let t = Trees::parse(doc);
                let cr = a.c.cJSON_ReplaceItemInObjectCaseSensitive(
                    t.c,
                    key.as_ptr(),
                    a.c.cJSON_CreateString(key.as_ptr()),
                );
                let rr = a.rust.cJSON_ReplaceItemInObjectCaseSensitive(
                    t.r,
                    key.as_ptr(),
                    a.rust.cJSON_CreateString(key.as_ptr()),
                );
                let ctx = format!("ReplaceItemInObjectCS({doc},{name})");
                assert_eq!(cr, rr, "{ctx}: result");
                assert_tree_eq(&ctx, t.c, t.r);
            }
        }

        // ReplaceItemViaPointer: first, middle, last, self, foreign, NULLs
        for idx in 0..3 {
            let t = Trees::parse("[1,2,3]");
            let ci = a.c.cJSON_GetArrayItem(t.c, idx);
            let ri = a.rust.cJSON_GetArrayItem(t.r, idx);
            let cr = a
                .c
                .cJSON_ReplaceItemViaPointer(t.c, ci, a.c.cJSON_CreateString(c_str("x")));
            let rr = a.rust.cJSON_ReplaceItemViaPointer(
                t.r,
                ri,
                a.rust.cJSON_CreateString(c_str("x")),
            );
            assert_eq!(cr, rr, "ReplaceItemViaPointer(idx={idx})");
            assert_tree_eq(&format!("ReplaceItemViaPointer(idx={idx})"), t.c, t.r);
        }
        {
            let t = Trees::parse("[1,2,3]");
            let ci = a.c.cJSON_GetArrayItem(t.c, 1);
            let ri = a.rust.cJSON_GetArrayItem(t.r, 1);
            // replacement == item
            assert_eq!(
                a.c.cJSON_ReplaceItemViaPointer(t.c, ci, ci),
                a.rust.cJSON_ReplaceItemViaPointer(t.r, ri, ri),
                "ReplaceItemViaPointer(item==replacement)"
            );
            assert_tree_eq("ReplaceItemViaPointer(item==replacement)", t.c, t.r);
            // NULL replacement
            assert_eq!(
                a.c.cJSON_ReplaceItemViaPointer(t.c, ci, std::ptr::null_mut()),
                a.rust
                    .cJSON_ReplaceItemViaPointer(t.r, ri, std::ptr::null_mut())
            );
            assert_eq!(
                a.c.cJSON_ReplaceItemViaPointer(std::ptr::null_mut(), ci, ci),
                a.rust
                    .cJSON_ReplaceItemViaPointer(std::ptr::null_mut(), ri, ri)
            );
        }
    }
}

fn c_str(s: &str) -> *const c_char {
    // leaked, tests are short lived
    Box::leak(CString::new(s).unwrap().into_boxed_c_str()).as_ptr()
}

#[test]
fn duplicate_items() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let docs: Vec<String> = DOCS
            .iter()
            .map(|s| s.to_string())
            .chain(
                [
                    "[[1,[2,[3]]]]".to_string(),
                    "{\"a\":{\"b\":{\"c\":[1,2,{\"d\":null}]}}}".to_string(),
                    format!("[{}]", (0..100).map(|i| i.to_string()).collect::<Vec<_>>().join(",")),
                ]
                .into_iter(),
            )
            .collect();
        for doc in docs {
            let t = Trees::parse(&doc);
            for recurse in [0, 1] {
                let cd = a.c.cJSON_Duplicate(t.c, recurse);
                let rd = a.rust.cJSON_Duplicate(t.r, recurse);
                let ctx = format!("Duplicate({doc:.40},{recurse})");
                assert_eq!(cd.is_null(), rd.is_null(), "{ctx}: nullness");
                if !cd.is_null() {
                    assert_tree_eq(&ctx, cd, rd);
                }
                a.c.cJSON_Delete(cd);
                a.rust.cJSON_Delete(rd);
            }
        }
        assert_eq!(
            a.c.cJSON_Duplicate(std::ptr::null(), 1).is_null(),
            a.rust.cJSON_Duplicate(std::ptr::null(), 1).is_null()
        );

        // duplicating a tree that contains references
        let t = Trees::parse("[1,2]");
        let cholder = a.c.cJSON_CreateArray();
        let rholder = a.rust.cJSON_CreateArray();
        a.c.cJSON_AddItemReferenceToArray(cholder, t.c);
        a.rust.cJSON_AddItemReferenceToArray(rholder, t.r);
        for recurse in [0, 1] {
            let cd = a.c.cJSON_Duplicate(cholder, recurse);
            let rd = a.rust.cJSON_Duplicate(rholder, recurse);
            assert_eq!(cd.is_null(), rd.is_null());
            if !cd.is_null() {
                assert_tree_eq(&format!("Duplicate(ref holder,{recurse})"), cd, rd);
            }
            a.c.cJSON_Delete(cd);
            a.rust.cJSON_Delete(rd);
        }
        a.c.cJSON_Delete(cholder);
        a.rust.cJSON_Delete(rholder);
    }
}

/// `cJSON_Duplicate` refuses trees whose sibling chain exceeds
/// `CJSON_CIRCULAR_LIMIT`.
#[test]
fn duplicate_circular_limit() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for count in [9999usize, 10000, 10001, 10050] {
            let carr = a.c.cJSON_CreateArray();
            let rarr = a.rust.cJSON_CreateArray();
            for i in 0..count {
                a.c.cJSON_AddItemToArray(carr, a.c.cJSON_CreateNumber(i as f64));
                a.rust
                    .cJSON_AddItemToArray(rarr, a.rust.cJSON_CreateNumber(i as f64));
            }
            let cd = a.c.cJSON_Duplicate(carr, 1);
            let rd = a.rust.cJSON_Duplicate(rarr, 1);
            assert_eq!(cd.is_null(), rd.is_null(), "Duplicate(array of {count})");
            if !cd.is_null() {
                assert_eq!(
                    a.c.cJSON_GetArraySize(cd),
                    a.rust.cJSON_GetArraySize(rd),
                    "duplicate size for {count}"
                );
            }
            a.c.cJSON_Delete(cd);
            a.rust.cJSON_Delete(rd);
            a.c.cJSON_Delete(carr);
            a.rust.cJSON_Delete(rarr);
        }
    }
}

#[test]
fn compare_items() {
    let _guard = serial();
    let a = apis();
    let docs = [
        "null", "true", "false", "0", "1", "1.0", "1.5", "\"\"", "\"a\"", "\"A\"",
        "[]", "[1]", "[1,2]", "[2,1]", "{}", "{\"a\":1}", "{\"A\":1}",
        "{\"a\":1,\"b\":2}", "{\"b\":2,\"a\":1}", "{\"a\":2}", "[[1]]",
        "{\"a\":{\"b\":1}}", "{\"a\":[1,2]}",
    ];
    unsafe {
        let ct: Vec<*mut cJSON> = docs
            .iter()
            .map(|d| a.c.cJSON_Parse(CString::new(*d).unwrap().as_ptr()))
            .collect();
        let rt: Vec<*mut cJSON> = docs
            .iter()
            .map(|d| a.rust.cJSON_Parse(CString::new(*d).unwrap().as_ptr()))
            .collect();
        for i in 0..docs.len() {
            for j in 0..docs.len() {
                for cs in [0, 1] {
                    assert_eq!(
                        a.c.cJSON_Compare(ct[i], ct[j], cs),
                        a.rust.cJSON_Compare(rt[i], rt[j], cs),
                        "Compare({}, {}, cs={cs})",
                        docs[i],
                        docs[j]
                    );
                }
            }
            for cs in [0, 1] {
                assert_eq!(
                    a.c.cJSON_Compare(ct[i], std::ptr::null(), cs),
                    a.rust.cJSON_Compare(rt[i], std::ptr::null(), cs)
                );
                assert_eq!(
                    a.c.cJSON_Compare(std::ptr::null(), ct[i], cs),
                    a.rust.cJSON_Compare(std::ptr::null(), rt[i], cs)
                );
            }
        }
        for cs in [0, 1] {
            assert_eq!(
                a.c.cJSON_Compare(std::ptr::null(), std::ptr::null(), cs),
                a.rust.cJSON_Compare(std::ptr::null(), std::ptr::null(), cs)
            );
        }
        for p in ct {
            a.c.cJSON_Delete(p);
        }
        for p in rt {
            a.rust.cJSON_Delete(p);
        }

        // synthetic: invalid type, raw items, self comparison
        for type_ in [
            cJSON_Invalid,
            cJSON_Raw,
            cJSON_Number | cJSON_String,
            1 << 9,
            -1,
        ] {
            let mut x = cJSON {
                next: std::ptr::null_mut(),
                prev: std::ptr::null_mut(),
                child: std::ptr::null_mut(),
                type_,
                valuestring: std::ptr::null_mut(),
                valueint: 0,
                valuedouble: 0.0,
                string: std::ptr::null_mut(),
            };
            let p: *const cJSON = &mut x;
            for cs in [0, 1] {
                assert_eq!(
                    a.c.cJSON_Compare(p, p, cs),
                    a.rust.cJSON_Compare(p, p, cs),
                    "Compare(type={type_} with itself, cs={cs})"
                );
            }
        }
    }
}

#[test]
fn minify() {
    let _guard = serial();
    let a = apis();
    let inputs = [
        "",
        " ",
        "\t\r\n",
        "{ }",
        "{ \"a\" : 1 }",
        "[ 1 , 2 , 3 ]",
        "{\n\t\"a\": [1, 2],\n\t\"b\": \"c d\"\n}",
        "// line comment\n{}",
        "/* block */{}",
        "{} // trailing",
        "{} /* trailing",
        "/",
        "//",
        "/*",
        "/**/",
        "/*/",
        "\"string with // and /* inside\"",
        "\"unterminated",
        "\"escaped \\\" quote\" ",
        "[\"a b\", \"c\td\"]",
        "{\"a\":\"\\\\\"}",
        "1 2 3",
        "\n\n\n",
        "{\"a\":1}// c\n/*d*/[2]",
        "\"\\",
    ];
    unsafe {
        for input in inputs {
            let mut cbuf: Vec<u8> = input.bytes().chain(std::iter::once(0)).collect();
            let mut rbuf = cbuf.clone();
            a.c.cJSON_Minify(cbuf.as_mut_ptr() as *mut c_char);
            a.rust.cJSON_Minify(rbuf.as_mut_ptr() as *mut c_char);
            assert_eq!(
                cbuf,
                rbuf,
                "Minify({input:?})\nC:    {:?}\nRust: {:?}",
                String::from_utf8_lossy(&cbuf),
                String::from_utf8_lossy(&rbuf)
            );
        }
        // NULL input must be tolerated identically
        a.c.cJSON_Minify(std::ptr::null_mut());
        a.rust.cJSON_Minify(std::ptr::null_mut());
    }
}

#[test]
fn set_number_and_valuestring() {
    let _guard = serial();
    let a = apis();
    unsafe {
        for v in [
            0.0f64,
            1.0,
            -1.0,
            1.5,
            2147483647.0,
            2147483648.0,
            -2147483648.0,
            -2147483649.0,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
            1e300,
        ] {
            let ci = a.c.cJSON_CreateNumber(0.0);
            let ri = a.rust.cJSON_CreateNumber(0.0);
            let cv = a.c.cJSON_SetNumberHelper(ci, v);
            let rv = a.rust.cJSON_SetNumberHelper(ri, v);
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "SetNumberHelper({v:?}) return value"
            );
            assert_tree_eq(&format!("SetNumberHelper({v:?})"), ci, ri);
            a.c.cJSON_Delete(ci);
            a.rust.cJSON_Delete(ri);
        }

        // SetValuestring: shorter, equal, longer, non-string types, NULL
        for (initial, new) in [
            ("hello", "hi"),
            ("hello", "hello"),
            ("hi", "hello world"),
            ("", ""),
            ("", "x"),
            ("abc", ""),
        ] {
            let init = CString::new(initial).unwrap();
            let nv = CString::new(new).unwrap();
            let ci = a.c.cJSON_CreateString(init.as_ptr());
            let ri = a.rust.cJSON_CreateString(init.as_ptr());
            let cr = a.c.cJSON_SetValuestring(ci, nv.as_ptr());
            let rr = a.rust.cJSON_SetValuestring(ri, nv.as_ptr());
            let ctx = format!("SetValuestring({initial:?} -> {new:?})");
            assert_eq!(cr.is_null(), rr.is_null(), "{ctx}: nullness");
            assert_eq!(cstr_bytes(cr), cstr_bytes(rr), "{ctx}: returned string");
            assert_tree_eq(&ctx, ci, ri);
            a.c.cJSON_Delete(ci);
            a.rust.cJSON_Delete(ri);
        }

        // wrong types
        let nv = CString::new("new").unwrap();
        let ci = a.c.cJSON_CreateNumber(1.0);
        let ri = a.rust.cJSON_CreateNumber(1.0);
        assert_eq!(
            a.c.cJSON_SetValuestring(ci, nv.as_ptr()).is_null(),
            a.rust.cJSON_SetValuestring(ri, nv.as_ptr()).is_null()
        );
        a.c.cJSON_Delete(ci);
        a.rust.cJSON_Delete(ri);

        // string reference (cJSON_IsReference set)
        let sv = CString::new("static").unwrap();
        let ci = a.c.cJSON_CreateStringReference(sv.as_ptr());
        let ri = a.rust.cJSON_CreateStringReference(sv.as_ptr());
        assert_eq!(
            a.c.cJSON_SetValuestring(ci, nv.as_ptr()).is_null(),
            a.rust.cJSON_SetValuestring(ri, nv.as_ptr()).is_null()
        );
        a.c.cJSON_Delete(ci);
        a.rust.cJSON_Delete(ri);

        // NULL arguments
        let ci = a.c.cJSON_CreateString(sv.as_ptr());
        let ri = a.rust.cJSON_CreateString(sv.as_ptr());
        assert_eq!(
            a.c.cJSON_SetValuestring(ci, std::ptr::null()).is_null(),
            a.rust.cJSON_SetValuestring(ri, std::ptr::null()).is_null()
        );
        assert_eq!(
            a.c.cJSON_SetValuestring(std::ptr::null_mut(), nv.as_ptr())
                .is_null(),
            a.rust
                .cJSON_SetValuestring(std::ptr::null_mut(), nv.as_ptr())
                .is_null()
        );
        a.c.cJSON_Delete(ci);
        a.rust.cJSON_Delete(ri);

        // overlapping source and destination: valuestring points into itself
        let ci = a.c.cJSON_CreateString(c_str("abcdef"));
        let ri = a.rust.cJSON_CreateString(c_str("abcdef"));
        let cinner = (*ci).valuestring.add(2);
        let rinner = (*ri).valuestring.add(2);
        let cr = a.c.cJSON_SetValuestring(ci, cinner);
        let rr = a.rust.cJSON_SetValuestring(ri, rinner);
        assert_eq!(cr.is_null(), rr.is_null(), "SetValuestring(overlapping)");
        assert_eq!(cstr_bytes(cr), cstr_bytes(rr));
        assert_tree_eq("SetValuestring(overlapping)", ci, ri);
        a.c.cJSON_Delete(ci);
        a.rust.cJSON_Delete(ri);
    }
}

/// `cJSON_free` / `cJSON_malloc` on printed buffers, plus deleting sub-trees.
#[test]
fn free_printed_buffers() {
    let _guard = serial();
    let a = apis();
    unsafe {
        let t = Trees::parse("{\"a\":[1,2,3]}");
        let cp = a.c.cJSON_Print(t.c);
        let rp = a.rust.cJSON_Print(t.r);
        assert_eq!(cstr_bytes(cp), cstr_bytes(rp));
        a.c.cJSON_free(cp as *mut c_void);
        a.rust.cJSON_free(rp as *mut c_void);
    }
}
