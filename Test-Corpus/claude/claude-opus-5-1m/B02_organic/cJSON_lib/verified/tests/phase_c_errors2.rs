//! Phase C — differential error-path tests for `ERRORS.md` rows 100..183
//! (query, mutation, creation, duplicate, compare, minify, and the
//! out-of-range values that C accepts across the FFI boundary).
#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::{c_char, c_int};
use std::fmt::Write as _;
use std::ptr::null_mut;

unsafe fn make_array(api: &Api, n: usize) -> *mut CJson {
    let a = (api.cJSON_CreateArray)();
    for i in 0..n {
        (api.cJSON_AddItemToArray)(a, (api.cJSON_CreateNumber)(i as f64));
    }
    a
}

unsafe fn make_object(api: &Api, keys: &[&str]) -> *mut CJson {
    let o = (api.cJSON_CreateObject)();
    for (i, k) in keys.iter().enumerate() {
        let kb = cs(k);
        (api.cJSON_AddItemToObject)(o, kb.as_ptr(), (api.cJSON_CreateNumber)(i as f64));
    }
    o
}

/* ============ rows 100..109: query rejections ============ */

#[test]
fn rows_100_to_109_query_rejections() {
    diff("ERRORS 100-109", |api| unsafe {
        let mut log = String::new();

        // rows 100/101
        let _ = writeln!(log, "row100 size(NULL)={}", (api.cJSON_GetArraySize)(null_mut()));
        for (n, it) in [
            ("null", (api.cJSON_CreateNull)()),
            ("number", (api.cJSON_CreateNumber)(1.0)),
            ("string", (api.cJSON_CreateString)(cs("s").as_ptr())),
            ("empty_array", (api.cJSON_CreateArray)()),
            ("empty_object", (api.cJSON_CreateObject)()),
        ] {
            let _ = writeln!(log, "row101 size({n})={}", (api.cJSON_GetArraySize)(it));
            (api.cJSON_Delete)(it);
        }

        // rows 102/103/104
        let _ = writeln!(
            log,
            "row102 item(NULL,0) null={}",
            (api.cJSON_GetArrayItem)(null_mut(), 0).is_null()
        );
        for n in [0usize, 1, 3] {
            let a = make_array(api, n);
            for idx in [-1i32, 0, 1, 2, 3, 4, 1000, c_int::MAX, c_int::MIN, -2] {
                let _ = writeln!(
                    log,
                    "row103/104 n={n} idx={idx} null={}",
                    (api.cJSON_GetArrayItem)(a, idx).is_null()
                );
            }
            (api.cJSON_Delete)(a);
        }

        // rows 105..109
        let o = make_object(api, &["a", "B"]);
        let key = cs("a");
        let _ = writeln!(
            log,
            "row105 get(NULL,\"a\") insens_null={} sens_null={} has={}",
            (api.cJSON_GetObjectItem)(null_mut(), key.as_ptr()).is_null(),
            (api.cJSON_GetObjectItemCaseSensitive)(null_mut(), key.as_ptr()).is_null(),
            (api.cJSON_HasObjectItem)(null_mut(), key.as_ptr())
        );
        let _ = writeln!(
            log,
            "row106 get(obj,NULL) insens_null={} sens_null={} has={}",
            (api.cJSON_GetObjectItem)(o, null_mut()).is_null(),
            (api.cJSON_GetObjectItemCaseSensitive)(o, null_mut()).is_null(),
            (api.cJSON_HasObjectItem)(o, null_mut())
        );
        for probe in ["b", "A", "missing", "", "a "] {
            let p = cs(probe);
            let _ = writeln!(
                log,
                "row107 {probe:?}: insens_null={} sens_null={} has={}",
                (api.cJSON_GetObjectItem)(o, p.as_ptr()).is_null(),
                (api.cJSON_GetObjectItemCaseSensitive)(o, p.as_ptr()).is_null(),
                (api.cJSON_HasObjectItem)(o, p.as_ptr())
            );
        }
        (api.cJSON_Delete)(o);

        // row 108: array children have no `string` at all
        let a = make_array(api, 3);
        for probe in ["a", "", "0"] {
            let p = cs(probe);
            let _ = writeln!(
                log,
                "row108 array lookup {probe:?}: insens_null={} sens_null={} has={}",
                (api.cJSON_GetObjectItem)(a, p.as_ptr()).is_null(),
                (api.cJSON_GetObjectItemCaseSensitive)(a, p.as_ptr()).is_null(),
                (api.cJSON_HasObjectItem)(a, p.as_ptr())
            );
        }
        (api.cJSON_Delete)(a);
        log
    });
}

/* ============ rows 110..125: add / reference rejections ============ */

#[test]
fn rows_110_to_125_add_rejections() {
    diff("ERRORS 110-125", |api| unsafe {
        let mut log = String::new();
        let key = cs("k");

        // rows 112..114
        let arr = make_array(api, 2);
        let item = (api.cJSON_CreateNumber)(1.0);
        let _ = writeln!(
            log,
            "row112 AddItemToArray(arr,NULL)={}",
            (api.cJSON_AddItemToArray)(arr, null_mut())
        );
        let _ = writeln!(
            log,
            "row113 AddItemToArray(NULL,item)={}",
            (api.cJSON_AddItemToArray)(null_mut(), item)
        );
        let _ = writeln!(
            log,
            "row114 AddItemToArray(arr,arr)={}",
            (api.cJSON_AddItemToArray)(arr, arr)
        );
        let _ = write!(log, "  arr unchanged: {}", dump(arr));

        // rows 115..118
        let obj = (api.cJSON_CreateObject)();
        let _ = writeln!(
            log,
            "row115 AddItemToObject(NULL,k,item)={} CS={}",
            (api.cJSON_AddItemToObject)(null_mut(), key.as_ptr(), item),
            (api.cJSON_AddItemToObjectCS)(null_mut(), key.as_ptr(), item)
        );
        let _ = writeln!(
            log,
            "row116 AddItemToObject(obj,NULL,item)={} CS={}",
            (api.cJSON_AddItemToObject)(obj, null_mut(), item),
            (api.cJSON_AddItemToObjectCS)(obj, null_mut(), item)
        );
        let _ = writeln!(
            log,
            "row117 AddItemToObject(obj,k,NULL)={} CS={}",
            (api.cJSON_AddItemToObject)(obj, key.as_ptr(), null_mut()),
            (api.cJSON_AddItemToObjectCS)(obj, key.as_ptr(), null_mut())
        );
        let _ = writeln!(
            log,
            "row118 AddItemToObject(obj,k,obj)={}",
            (api.cJSON_AddItemToObject)(obj, key.as_ptr(), obj)
        );
        let _ = write!(log, "  obj after self-add: {}", dump(obj));
        (api.cJSON_Delete)(item);

        // rows 110/120/121/122
        let _ = writeln!(
            log,
            "row110 AddItemReferenceToArray(arr,NULL)={}",
            (api.cJSON_AddItemReferenceToArray)(arr, null_mut())
        );
        let _ = writeln!(
            log,
            "row120 AddItemReferenceToArray(NULL,item)={}",
            (api.cJSON_AddItemReferenceToArray)(null_mut(), arr)
        );
        let _ = writeln!(
            log,
            "row121 AddItemReferenceToObject(NULL,k,item)={}",
            (api.cJSON_AddItemReferenceToObject)(null_mut(), key.as_ptr(), arr)
        );
        let _ = writeln!(
            log,
            "row122 AddItemReferenceToObject(obj,NULL,item)={}",
            (api.cJSON_AddItemReferenceToObject)(obj, null_mut(), arr)
        );
        let _ = writeln!(
            log,
            "row110b AddItemReferenceToObject(obj,k,NULL)={}",
            (api.cJSON_AddItemReferenceToObject)(obj, key.as_ptr(), null_mut())
        );
        let _ = write!(log, "  obj: {}", dump(obj));
        (api.cJSON_Delete)(obj);
        (api.cJSON_Delete)(arr);

        // rows 123..125: the nine Add*ToObject helpers with invalid arguments
        let good = (api.cJSON_CreateObject)();
        let sval = cs("v");
        for (label, object, name) in [
            ("null_object", null_mut::<CJson>(), Some(&key)),
            ("null_name", good, None),
            ("both_null", null_mut::<CJson>(), None),
        ] {
            let np: *const c_char = match name {
                Some(k) => k.as_ptr(),
                None => null_mut(),
            };
            let _ = writeln!(
                log,
                "row123 {label}: null={} true={} false={} bool={} number={} string={} raw={} object={} array={}",
                (api.cJSON_AddNullToObject)(object, np).is_null(),
                (api.cJSON_AddTrueToObject)(object, np).is_null(),
                (api.cJSON_AddFalseToObject)(object, np).is_null(),
                (api.cJSON_AddBoolToObject)(object, np, 1).is_null(),
                (api.cJSON_AddNumberToObject)(object, np, 1.0).is_null(),
                (api.cJSON_AddStringToObject)(object, np, sval.as_ptr()).is_null(),
                (api.cJSON_AddRawToObject)(object, np, sval.as_ptr()).is_null(),
                (api.cJSON_AddObjectToObject)(object, np).is_null(),
                (api.cJSON_AddArrayToObject)(object, np).is_null(),
            );
        }
        // rows 124/125: NULL payload strings
        let _ = writeln!(
            log,
            "row124 AddStringToObject(obj,k,NULL) null={}",
            (api.cJSON_AddStringToObject)(good, key.as_ptr(), null_mut()).is_null()
        );
        let _ = writeln!(
            log,
            "row125 AddRawToObject(obj,k,NULL) null={}",
            (api.cJSON_AddRawToObject)(good, key.as_ptr(), null_mut()).is_null()
        );
        let _ = write!(log, "  object still: {}", dump(good));
        (api.cJSON_Delete)(good);
        log
    });
}

/* ======== rows 126..147: detach / insert / replace rejections ======== */

#[test]
fn rows_126_to_147_mutation_rejections() {
    diff("ERRORS 126-147", |api| unsafe {
        let mut log = String::new();
        let key = cs("a");

        // rows 126..128
        let arr = make_array(api, 3);
        let foreign = (api.cJSON_CreateNumber)(9.0);
        let _ = writeln!(
            log,
            "row126 Detach(NULL,item) null={}",
            (api.cJSON_DetachItemViaPointer)(null_mut(), foreign).is_null()
        );
        let _ = writeln!(
            log,
            "row127 Detach(arr,NULL) null={}",
            (api.cJSON_DetachItemViaPointer)(arr, null_mut()).is_null()
        );
        let _ = writeln!(
            log,
            "row128 Detach(arr,foreign) null={}",
            (api.cJSON_DetachItemViaPointer)(arr, foreign).is_null()
        );
        // an item that is already detached (prev == NULL, != parent->child)
        let second = (api.cJSON_GetArrayItem)(arr, 1);
        let det = (api.cJSON_DetachItemViaPointer)(arr, second);
        let _ = writeln!(
            log,
            "row128 re-detach null={}",
            (api.cJSON_DetachItemViaPointer)(arr, det).is_null()
        );
        (api.cJSON_Delete)(det);
        (api.cJSON_Delete)(foreign);
        let _ = write!(log, "  arr: {}", dump(arr));

        // rows 129/130
        for which in [-1i32, c_int::MIN, 2, 3, 100] {
            let _ = writeln!(
                log,
                "row129/130 DetachItemFromArray({which}) null={}",
                (api.cJSON_DetachItemFromArray)(arr, which).is_null()
            );
        }
        let _ = writeln!(
            log,
            "row129 DetachItemFromArray(NULL,0) null={}",
            (api.cJSON_DetachItemFromArray)(null_mut(), 0).is_null()
        );
        (api.cJSON_Delete)(arr);

        // row 131/132
        let obj = make_object(api, &["a", "B"]);
        for probe in ["missing", "b", "A"] {
            let p = cs(probe);
            let _ = writeln!(
                log,
                "row131 {probe:?}: detach_null={} detach_cs_null={}",
                (api.cJSON_DetachItemFromObject)(obj, p.as_ptr()).is_null(),
                (api.cJSON_DetachItemFromObjectCaseSensitive)(obj, p.as_ptr()).is_null()
            );
        }
        let _ = writeln!(
            log,
            "row131 NULL object: {} {}",
            (api.cJSON_DetachItemFromObject)(null_mut(), key.as_ptr()).is_null(),
            (api.cJSON_DetachItemFromObjectCaseSensitive)(null_mut(), key.as_ptr()).is_null()
        );
        let _ = writeln!(
            log,
            "row131 NULL key: {} {}",
            (api.cJSON_DetachItemFromObject)(obj, null_mut()).is_null(),
            (api.cJSON_DetachItemFromObjectCaseSensitive)(obj, null_mut()).is_null()
        );
        // row 132: the Delete* variants are no-ops for the same inputs
        (api.cJSON_DeleteItemFromArray)(null_mut(), 0);
        (api.cJSON_DeleteItemFromObject)(null_mut(), key.as_ptr());
        (api.cJSON_DeleteItemFromObjectCaseSensitive)(null_mut(), key.as_ptr());
        (api.cJSON_DeleteItemFromObject)(obj, null_mut());
        (api.cJSON_DeleteItemFromObjectCaseSensitive)(obj, null_mut());
        let missing = cs("missing");
        (api.cJSON_DeleteItemFromObject)(obj, missing.as_ptr());
        (api.cJSON_DeleteItemFromObjectCaseSensitive)(obj, missing.as_ptr());
        let _ = write!(log, "row132 obj intact: {}", dump(obj));
        (api.cJSON_Delete)(obj);

        // rows 133..136
        let arr = make_array(api, 2);
        let item = (api.cJSON_CreateNumber)(5.0);
        for which in [-1i32, c_int::MIN, -100] {
            let _ = writeln!(
                log,
                "row133 Insert({which}) rc={}",
                (api.cJSON_InsertItemInArray)(arr, which, item)
            );
        }
        let _ = writeln!(
            log,
            "row134 Insert(NULL item) rc={}",
            (api.cJSON_InsertItemInArray)(arr, 0, null_mut())
        );
        let _ = writeln!(
            log,
            "row135 Insert(NULL array) rc={}",
            (api.cJSON_InsertItemInArray)(null_mut(), 0, item)
        );
        let _ = write!(log, "  arr: {}", dump(arr));
        (api.cJSON_Delete)(item);
        (api.cJSON_Delete)(arr);

        // rows 137..143
        let arr = make_array(api, 2);
        let empty = (api.cJSON_CreateArray)();
        let rep = (api.cJSON_CreateNumber)(7.0);
        let first = (api.cJSON_GetArrayItem)(arr, 0);
        let _ = writeln!(
            log,
            "row137 Replace(NULL,item,rep) rc={}",
            (api.cJSON_ReplaceItemViaPointer)(null_mut(), first, rep)
        );
        let _ = writeln!(
            log,
            "row138 Replace(empty,item,rep) rc={}",
            (api.cJSON_ReplaceItemViaPointer)(empty, first, rep)
        );
        let _ = writeln!(
            log,
            "row139 Replace(arr,item,NULL) rc={}",
            (api.cJSON_ReplaceItemViaPointer)(arr, first, null_mut())
        );
        let _ = writeln!(
            log,
            "row140 Replace(arr,NULL,rep) rc={}",
            (api.cJSON_ReplaceItemViaPointer)(arr, null_mut(), rep)
        );
        let _ = writeln!(
            log,
            "row141 Replace(arr,item,item) rc={}",
            (api.cJSON_ReplaceItemViaPointer)(arr, first, first)
        );
        for which in [-1i32, c_int::MIN, 2, 3, 100] {
            let _ = writeln!(
                log,
                "row142/143 ReplaceItemInArray({which}) rc={}",
                (api.cJSON_ReplaceItemInArray)(arr, which, rep)
            );
        }
        let _ = writeln!(
            log,
            "row143 ReplaceItemInArray(NULL) rc={}",
            (api.cJSON_ReplaceItemInArray)(null_mut(), 0, rep)
        );
        let _ = write!(log, "  arr: {}", dump(arr));
        (api.cJSON_Delete)(rep);
        (api.cJSON_Delete)(empty);
        (api.cJSON_Delete)(arr);

        // rows 144..147
        let obj = make_object(api, &["a", "B"]);
        let rep = (api.cJSON_CreateNumber)(8.0);
        let _ = writeln!(
            log,
            "row144 ReplaceItemInObject(obj,k,NULL) rc={} CS={}",
            (api.cJSON_ReplaceItemInObject)(obj, key.as_ptr(), null_mut()),
            (api.cJSON_ReplaceItemInObjectCaseSensitive)(obj, key.as_ptr(), null_mut())
        );
        let _ = writeln!(
            log,
            "row145 ReplaceItemInObject(obj,NULL,rep) rc={} CS={}",
            (api.cJSON_ReplaceItemInObject)(obj, null_mut(), rep),
            (api.cJSON_ReplaceItemInObjectCaseSensitive)(obj, null_mut(), rep)
        );
        // row 147: absent key still overwrites replacement->string
        let missing = cs("nope");
        let rc = (api.cJSON_ReplaceItemInObject)(obj, missing.as_ptr(), rep);
        let _ = writeln!(
            log,
            "row147 absent key rc={} rep_key={:?} rep_type=0x{:x}",
            rc,
            read_cstr((*rep).string).map(|v| show(&v)),
            (*rep).type_
        );
        let rc = (api.cJSON_ReplaceItemInObjectCaseSensitive)(obj, missing.as_ptr(), rep);
        let _ = writeln!(
            log,
            "row147 absent key CS rc={} rep_key={:?}",
            rc,
            read_cstr((*rep).string).map(|v| show(&v))
        );
        // NULL object with a valid key: the key is still replaced first
        let rc = (api.cJSON_ReplaceItemInObject)(null_mut(), key.as_ptr(), rep);
        let _ = writeln!(
            log,
            "row147 NULL object rc={} rep_key={:?}",
            rc,
            read_cstr((*rep).string).map(|v| show(&v))
        );
        let _ = write!(log, "  obj: {}", dump(obj));
        (api.cJSON_Delete)(rep);
        (api.cJSON_Delete)(obj);
        log
    });
}

/* ============ rows 148..153: create rejections ============ */

#[test]
fn rows_148_to_153_create_rejections() {
    diff("ERRORS 148-153", |api| unsafe {
        let mut log = String::new();

        // rows 148/149
        let _ = writeln!(
            log,
            "row148 CreateString(NULL) null={} row149 CreateRaw(NULL) null={}",
            (api.cJSON_CreateString)(null_mut()).is_null(),
            (api.cJSON_CreateRaw)(null_mut()).is_null()
        );

        // rows 150/151/153
        let ints = [1i32, 2, 3];
        let floats = [1.0f32, 2.0, 3.0];
        let doubles = [1.0f64, 2.0, 3.0];
        let s1 = cs("a");
        let s2 = cs("b");
        let strs: [*const c_char; 2] = [s1.as_ptr(), s2.as_ptr()];
        for count in [-1i32, -100, c_int::MIN] {
            let _ = writeln!(
                log,
                "row150 count={count}: int={} float={} double={} string={}",
                (api.cJSON_CreateIntArray)(ints.as_ptr(), count).is_null(),
                (api.cJSON_CreateFloatArray)(floats.as_ptr(), count).is_null(),
                (api.cJSON_CreateDoubleArray)(doubles.as_ptr(), count).is_null(),
                (api.cJSON_CreateStringArray)(strs.as_ptr(), count).is_null(),
            );
        }
        for count in [0i32, 1, 2] {
            let _ = writeln!(
                log,
                "row151 NULL data count={count}: int={} float={} double={} string={}",
                (api.cJSON_CreateIntArray)(null_mut(), count).is_null(),
                (api.cJSON_CreateFloatArray)(null_mut(), count).is_null(),
                (api.cJSON_CreateDoubleArray)(null_mut(), count).is_null(),
                (api.cJSON_CreateStringArray)(null_mut(), count).is_null(),
            );
        }
        // row 153: count == 0 is valid and yields an empty array
        for (n, a) in [
            ("int", (api.cJSON_CreateIntArray)(ints.as_ptr(), 0)),
            ("float", (api.cJSON_CreateFloatArray)(floats.as_ptr(), 0)),
            ("double", (api.cJSON_CreateDoubleArray)(doubles.as_ptr(), 0)),
            ("string", (api.cJSON_CreateStringArray)(strs.as_ptr(), 0)),
        ] {
            let _ = write!(log, "row153 {n} count=0: {}", dump(a));
            let p = take_print(api, (api.cJSON_PrintUnformatted)(a));
            let _ = writeln!(log, "  print={}", p.map(|v| show(&v)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(a);
        }
        // row 152: an element that cannot be created (NULL string in the array)
        for bad_at in [0usize, 1, 2] {
            let mut ptrs: [*const c_char; 3] = [s1.as_ptr(), s2.as_ptr(), s1.as_ptr()];
            ptrs[bad_at] = null_mut();
            let a = (api.cJSON_CreateStringArray)(ptrs.as_ptr(), 3);
            let _ = writeln!(log, "row152 NULL at {bad_at}: null={}", a.is_null());
            (api.cJSON_Delete)(a);
        }
        log
    });
}

/* ============ rows 154, 159, 160: duplicate / minify / predicates ==== */

#[test]
fn rows_154_159_160_misc_rejections() {
    diff("ERRORS 154/159/160", |api| unsafe {
        let mut log = String::new();
        for recurse in [0i32, 1, 2, -1] {
            let _ = writeln!(
                log,
                "row154 Duplicate(NULL,{recurse}) null={}",
                (api.cJSON_Duplicate)(null_mut(), recurse).is_null()
            );
        }
        (api.cJSON_Minify)(null_mut());
        let _ = writeln!(log, "row159 Minify(NULL) survived");

        let preds: [(&str, unsafe extern "C" fn(*const CJson) -> c_int); 10] = [
            ("IsInvalid", api.cJSON_IsInvalid),
            ("IsFalse", api.cJSON_IsFalse),
            ("IsTrue", api.cJSON_IsTrue),
            ("IsBool", api.cJSON_IsBool),
            ("IsNull", api.cJSON_IsNull),
            ("IsNumber", api.cJSON_IsNumber),
            ("IsString", api.cJSON_IsString),
            ("IsArray", api.cJSON_IsArray),
            ("IsObject", api.cJSON_IsObject),
            ("IsRaw", api.cJSON_IsRaw),
        ];
        for (n, f) in preds {
            let _ = writeln!(log, "row160 {n}(NULL)={}", f(null_mut()));
        }
        log
    });
}

/* ============ rows 161..172: cJSON_Compare rejections ============ */

#[test]
fn rows_161_to_172_compare_rejections() {
    diff("ERRORS 161-172", |api| unsafe {
        let mut log = String::new();
        let a = (api.cJSON_CreateNumber)(1.0);

        // rows 161/162
        for cs_ in [0i32, 1] {
            let _ = writeln!(
                log,
                "row161/162 cs={cs_}: NULL_a={} NULL_b={} both={}",
                (api.cJSON_Compare)(null_mut(), a, cs_),
                (api.cJSON_Compare)(a, null_mut(), cs_),
                (api.cJSON_Compare)(null_mut(), null_mut(), cs_)
            );
        }
        (api.cJSON_Delete)(a);

        // row 163: type mismatch
        let items: Vec<(String, *mut CJson)> = vec![
            ("null".into(), (api.cJSON_CreateNull)()),
            ("true".into(), (api.cJSON_CreateTrue)()),
            ("false".into(), (api.cJSON_CreateFalse)()),
            ("number".into(), (api.cJSON_CreateNumber)(1.0)),
            ("string".into(), (api.cJSON_CreateString)(cs("x").as_ptr())),
            ("raw".into(), (api.cJSON_CreateRaw)(cs("x").as_ptr())),
            ("array".into(), (api.cJSON_CreateArray)()),
            ("object".into(), (api.cJSON_CreateObject)()),
        ];
        for (na, ia) in &items {
            for (nb, ib) in &items {
                if na != nb {
                    let _ = writeln!(
                        log,
                        "row163 {na} vs {nb} = {}",
                        (api.cJSON_Compare)(*ia, *ib, 1)
                    );
                }
            }
        }
        for (_, it) in &items {
            (api.cJSON_Delete)(*it);
        }

        // row 164: equal but invalid types
        for t in [CJSON_INVALID, 3, 5, 0x0F, 0xFF, 0x100, c_int::MIN, -1] {
            let x = (api.cJSON_CreateNumber)(1.0);
            let y = (api.cJSON_CreateNumber)(1.0);
            (*x).type_ = t;
            (*y).type_ = t;
            let _ = writeln!(
                log,
                "row164 type=0x{t:x}: cmp={} self_cmp={}",
                (api.cJSON_Compare)(x, y, 1),
                (api.cJSON_Compare)(x, x, 1)
            );
            (*x).type_ = CJSON_NUMBER;
            (*y).type_ = CJSON_NUMBER;
            (api.cJSON_Delete)(x);
            (api.cJSON_Delete)(y);
        }

        // row 165: numbers outside the tolerance
        for (x, y) in [(1.0, 2.0), (0.0, 1e-300), (1e300, -1e300), (f64::NAN, f64::NAN)] {
            let a = (api.cJSON_CreateNumber)(x);
            let b = (api.cJSON_CreateNumber)(y);
            let _ = writeln!(log, "row165 {x} vs {y} = {}", (api.cJSON_Compare)(a, b, 1));
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(b);
        }

        // rows 166..168: string/raw with NULL valuestring
        for ty in [CJSON_STRING, CJSON_RAW] {
            let with = (api.cJSON_CreateString)(cs("v").as_ptr());
            (*with).type_ = ty;
            let without = (api.cJSON_CreateNull)();
            (*without).type_ = ty;
            let _ = writeln!(
                log,
                "row166/167 type=0x{ty:x}: a_null_b_ok={} a_ok_b_null={} both_null={}",
                (api.cJSON_Compare)(without, with, 1),
                (api.cJSON_Compare)(with, without, 1),
                (api.cJSON_Compare)(without, without, 1)
            );
            let other = (api.cJSON_CreateString)(cs("w").as_ptr());
            (*other).type_ = ty;
            let _ = writeln!(log, "row168 differ={}", (api.cJSON_Compare)(with, other, 1));
            (api.cJSON_Delete)(with);
            (api.cJSON_Delete)(without);
            (api.cJSON_Delete)(other);
        }

        // rows 169..172: containers
        let pairs = [
            ("[1]", "[1,2]"),
            ("[1,2]", "[1]"),
            ("[]", "[1]"),
            ("[1]", "[]"),
            ("{\"a\":1}", "{\"a\":1,\"b\":2}"),
            ("{\"a\":1,\"b\":2}", "{\"a\":1}"),
            ("{\"a\":1}", "{\"b\":1}"),
            ("[[1]]", "[[2]]"),
            ("{\"a\":[1]}", "{\"a\":[2]}"),
            ("{\"a\":{\"b\":1}}", "{\"a\":{\"c\":1}}"),
        ];
        for (x, y) in pairs {
            let xb = cs(x);
            let yb = cs(y);
            let a = (api.cJSON_Parse)(xb.as_ptr());
            let b = (api.cJSON_Parse)(yb.as_ptr());
            for cs_ in [0i32, 1] {
                let _ = writeln!(
                    log,
                    "row169-172 {x} vs {y} cs={cs_} = {}",
                    (api.cJSON_Compare)(a, b, cs_)
                );
            }
            (api.cJSON_Delete)(a);
            (api.cJSON_Delete)(b);
        }
        // an object member with a NULL key inside a compared object
        let o1 = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToArray)(o1, (api.cJSON_CreateNumber)(1.0));
        let o2 = (api.cJSON_CreateObject)();
        (api.cJSON_AddItemToArray)(o2, (api.cJSON_CreateNumber)(1.0));
        for cs_ in [0i32, 1] {
            let _ = writeln!(
                log,
                "row170/171 null-key objects cs={cs_} = {}",
                (api.cJSON_Compare)(o1, o2, cs_)
            );
        }
        (api.cJSON_Delete)(o1);
        (api.cJSON_Delete)(o2);
        log
    });
}

/* ============ rows 175..181: free/malloc, saturation, FFI values ==== */

#[test]
fn rows_175_to_181_boundaries() {
    diff("ERRORS 175-181", |api| unsafe {
        let mut log = String::new();

        // rows 175/176
        (api.cJSON_free)(null_mut());
        let _ = writeln!(log, "row175 free(NULL) survived");
        let p = (api.cJSON_malloc)(0);
        let _ = writeln!(log, "row176 malloc(0) null={}", p.is_null());
        (api.cJSON_free)(p);

        // rows 178/179: saturation and NaN
        let values: [f64; 16] = [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            2147483647.0,
            2147483647.5,
            2147483648.0,
            -2147483648.0,
            -2147483648.5,
            -2147483649.0,
            1e300,
            -1e300,
            0.0,
            -0.0,
            0.9999999999,
            -0.9999999999,
        ];
        for v in values {
            let it = (api.cJSON_CreateNumber)(v);
            let _ = writeln!(
                log,
                "row179 CreateNumber(0x{:016x}) int={} dbl=0x{:016x}",
                v.to_bits(),
                (*it).valueint,
                (*it).valuedouble.to_bits()
            );
            let ret = (api.cJSON_SetNumberHelper)(it, v);
            let _ = writeln!(
                log,
                "row178 SetNumberHelper(0x{:016x}) ret=0x{:016x} int={} dbl=0x{:016x}",
                v.to_bits(),
                ret.to_bits(),
                (*it).valueint,
                (*it).valuedouble.to_bits()
            );
            let pr = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let _ = writeln!(log, "  print={}", pr.map(|x| show(&x)).unwrap_or("NULL".into()));
            (api.cJSON_Delete)(it);
        }

        // row 180: out-of-range cJSON_bool values across the FFI boundary
        let doc = cs("{\"a\":[1,2],\"B\":\"x\"}");
        let root = (api.cJSON_Parse)(doc.as_ptr());
        let dup = (api.cJSON_Duplicate)(root, 1);
        let mut pbuf = vec![0x11u8; 256];
        for b in [0i32, 1, 2, -1, 255, 256, 0x1_0000, c_int::MIN, c_int::MAX] {
            let bi = (api.cJSON_CreateBool)(b);
            let pr = take_print(api, (api.cJSON_PrintUnformatted)(bi));
            let _ = writeln!(
                log,
                "row180 CreateBool({b}) type=0x{:x} print={}",
                (*bi).type_,
                pr.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            (api.cJSON_Delete)(bi);
            let _ = writeln!(
                log,
                "row180 Compare(cs={b})={} Duplicate(recurse={b}) null={}",
                (api.cJSON_Compare)(root, dup, b),
                {
                    let d = (api.cJSON_Duplicate)(root, b);
                    let n = d.is_null();
                    (api.cJSON_Delete)(d);
                    n
                }
            );
            let pb = take_print(api, (api.cJSON_PrintBuffered)(root, 8, b));
            let _ = writeln!(
                log,
                "row180 PrintBuffered(fmt={b})={}",
                pb.map(|v| show(&v)).unwrap_or("NULL".into())
            );
            let rc = (api.cJSON_PrintPreallocated)(
                root,
                pbuf.as_mut_ptr() as *mut c_char,
                256,
                b,
            );
            let _ = writeln!(log, "row180 PrintPreallocated(format={b}) rc={rc} buf={}", show(&pbuf));
            let r2 = (api.cJSON_ParseWithOpts)(doc.as_ptr(), null_mut(), b);
            let _ = writeln!(log, "row180 ParseWithOpts(rnt={b}) null={}", r2.is_null());
            (api.cJSON_Delete)(r2);
            let bo = (api.cJSON_CreateObject)();
            let k = cs("k");
            let added = (api.cJSON_AddBoolToObject)(bo, k.as_ptr(), b);
            let _ = writeln!(
                log,
                "row180 AddBoolToObject({b}) type=0x{:x}",
                if added.is_null() { 0 } else { (*added).type_ }
            );
            (api.cJSON_Delete)(bo);
        }
        (api.cJSON_Delete)(dup);
        (api.cJSON_Delete)(root);

        // row 181: out-of-range `type` values across the FFI boundary
        for t in [
            CJSON_INVALID, 1, 2, 3, 4, 7, 8, 0x0F, 0x10, 0x1F, 0x20, 0x40, 0x80, 0xC0,
            0xFF, 0x100, 0x1FF, 0x200, 0x3FF, -1, c_int::MIN, c_int::MAX,
        ] {
            let it = (api.cJSON_CreateNumber)(4.5);
            (*it).type_ = t;
            let pr = take_print(api, (api.cJSON_PrintUnformatted)(it));
            let other = (api.cJSON_CreateNumber)(4.5);
            (*other).type_ = t;
            let _ = writeln!(
                log,
                "row181 type=0x{t:x}: print={} cmp={} size={} getnum=0x{:016x} getstr_null={} setvs_null={}",
                pr.map(|v| show(&v)).unwrap_or("NULL".into()),
                (api.cJSON_Compare)(it, other, 1),
                (api.cJSON_GetArraySize)(it),
                (api.cJSON_GetNumberValue)(it).to_bits(),
                (api.cJSON_GetStringValue)(it).is_null(),
                (api.cJSON_SetValuestring)(it, cs("v").as_ptr()).is_null()
            );
            let d = (api.cJSON_Duplicate)(it, 1);
            let _ = writeln!(
                log,
                "  dup type=0x{:x}",
                if d.is_null() { 0 } else { (*d).type_ }
            );
            (*it).type_ = CJSON_NUMBER;
            (*other).type_ = CJSON_NUMBER;
            if !d.is_null() {
                (*d).type_ = CJSON_NUMBER;
            }
            (api.cJSON_Delete)(d);
            (api.cJSON_Delete)(it);
            (api.cJSON_Delete)(other);
        }
        log
    });
}
