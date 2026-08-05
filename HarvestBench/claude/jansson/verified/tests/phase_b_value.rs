//! Phase B — value API differential tests. CONFIGS.md rows 21-25, 31.
//! Builds identical json_t structures via each .so's constructors, manipulates
//! them through both, and compares observable results (dumps, sizes, getters).
mod common;
#[path = "gen.rs"]
mod gen;

use common::*;
use gen::Rng;
use std::os::raw::{c_char, c_double, c_int, c_void};

const JSON_SORT_KEYS: usize = 0x80;

// Constructor signatures
type FnObject = unsafe extern "C" fn() -> *mut c_void;
type FnArray = unsafe extern "C" fn() -> *mut c_void;
type FnInteger = unsafe extern "C" fn(JsonInt) -> *mut c_void;
type FnReal = unsafe extern "C" fn(c_double) -> *mut c_void;
type FnString = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type FnStringn = unsafe extern "C" fn(*const c_char, usize) -> *mut c_void;
type FnObjSetNew = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_void) -> c_int;
type FnObjGet = unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_void;
type FnObjDel = unsafe extern "C" fn(*mut c_void, *const c_char) -> c_int;
type FnObjSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnArrAppendNew = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;
type FnArrInsertNew = unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> c_int;
type FnArrSetNew = unsafe extern "C" fn(*mut c_void, usize, *mut c_void) -> c_int;
type FnArrRemove = unsafe extern "C" fn(*mut c_void, usize) -> c_int;
type FnArrExtend = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;
type FnArrGet = unsafe extern "C" fn(*const c_void, usize) -> *mut c_void;
type FnArrSize = unsafe extern "C" fn(*const c_void) -> usize;
type FnEqual = unsafe extern "C" fn(*const c_void, *const c_void) -> c_int;
type FnCopy = unsafe extern "C" fn(*mut c_void) -> *mut c_void;
type FnUpdate = unsafe extern "C" fn(*mut c_void, *mut c_void) -> c_int;

/// A small helper: dump a value via `lib`'s json_dumps(SORT_KEYS) → bytes.
unsafe fn dump(lib: &libloading::Library, v: *const c_void, flags: usize) -> Option<Vec<u8>> {
    let dumps: libloading::Symbol<FnDumps> = sym(lib, b"json_dumps");
    let s = dumps(v, flags);
    let out = cstr_to_vec(s);
    if !s.is_null() {
        libc_free(s as *mut c_void);
    }
    out
}

/// Build the SAME object in both libs and run `f`, comparing the returned dump.
/// `build` gets (lib, symbols-accessor) — but simplest: run a closure per-lib.
fn on_both<F>(f: F)
where
    F: Fn(&libloading::Library) -> Option<Vec<u8>>,
{
    let l = libs();
    let c = f(&l.c);
    let r = f(&l.r);
    assert_eq!(c, r, "value-API mismatch\nC   ={:?}\nRust={:?}",
        c.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
        r.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()));
}

#[test]
fn row_21_object_set_get_del() {
    for seed in 0..200u64 {
        on_both(|lib| unsafe {
            let obj_new: libloading::Symbol<FnObject> = sym(lib, b"json_object");
            let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
            let set_new: libloading::Symbol<FnObjSetNew> = sym(lib, b"json_object_set_new");
            let del: libloading::Symbol<FnObjDel> = sym(lib, b"json_object_del");
            let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

            let mut rng = Rng::new(seed);
            let obj = obj_new();
            let n = rng.below(12);
            for i in 0..n {
                let key = format!("key{}\0", rng.below(20));
                let iv = int_new(rng.next() as JsonInt);
                set_new(obj, key.as_ptr() as *const c_char, iv);
                // occasionally delete
                if rng.below(4) == 0 {
                    let dk = format!("key{}\0", rng.below(20));
                    let _ = del(obj, dk.as_ptr() as *const c_char);
                }
                let _ = i;
            }
            let out = dump(lib, obj, JSON_SORT_KEYS);
            delete(obj);
            out
        });
    }
}

#[test]
fn row_23_array_ops() {
    for seed in 0..200u64 {
        on_both(|lib| unsafe {
            let arr_new: libloading::Symbol<FnArray> = sym(lib, b"json_array");
            let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
            let append: libloading::Symbol<FnArrAppendNew> = sym(lib, b"json_array_append_new");
            let insert: libloading::Symbol<FnArrInsertNew> = sym(lib, b"json_array_insert_new");
            let set: libloading::Symbol<FnArrSetNew> = sym(lib, b"json_array_set_new");
            let remove: libloading::Symbol<FnArrRemove> = sym(lib, b"json_array_remove");
            let size: libloading::Symbol<FnArrSize> = sym(lib, b"json_array_size");
            let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

            let mut rng = Rng::new(seed ^ 0xAA);
            let arr = arr_new();
            let n = rng.below(20);
            for _ in 0..n {
                match rng.below(4) {
                    0 => {
                        append(arr, int_new(rng.next() as JsonInt));
                    }
                    1 => {
                        let sz = size(arr);
                        let idx = if sz == 0 { 0 } else { rng.below(sz as u64 + 1) as usize };
                        insert(arr, idx, int_new(rng.next() as JsonInt));
                    }
                    2 => {
                        let sz = size(arr);
                        if sz > 0 {
                            let idx = rng.below(sz as u64) as usize;
                            set(arr, idx, int_new(rng.next() as JsonInt));
                        }
                    }
                    _ => {
                        let sz = size(arr);
                        if sz > 0 {
                            let idx = rng.below(sz as u64) as usize;
                            let _ = remove(arr, idx);
                        }
                    }
                }
            }
            let out = dump(lib, arr, 0);
            delete(arr);
            out
        });
    }
}

#[test]
fn row_23_array_extend() {
    for seed in 0..100u64 {
        on_both(|lib| unsafe {
            let arr_new: libloading::Symbol<FnArray> = sym(lib, b"json_array");
            let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
            let append: libloading::Symbol<FnArrAppendNew> = sym(lib, b"json_array_append_new");
            let extend: libloading::Symbol<FnArrExtend> = sym(lib, b"json_array_extend");
            let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

            let mut rng = Rng::new(seed ^ 0xBB);
            let a = arr_new();
            let b = arr_new();
            for _ in 0..rng.below(6) {
                append(a, int_new(rng.next() as JsonInt));
            }
            for _ in 0..rng.below(6) {
                append(b, int_new(rng.next() as JsonInt));
            }
            extend(a, b);
            let out = dump(lib, a, 0);
            delete(a);
            delete(b);
            out
        });
    }
}

#[test]
fn row_22_object_update() {
    for which in ["json_object_update", "json_object_update_existing", "json_object_update_missing", "json_object_update_recursive"] {
        for seed in 0..80u64 {
            let wname = which.as_bytes().to_vec();
            on_both(move |lib| unsafe {
                let obj_new: libloading::Symbol<FnObject> = sym(lib, b"json_object");
                let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
                let set_new: libloading::Symbol<FnObjSetNew> = sym(lib, b"json_object_set_new");
                let update: libloading::Symbol<FnUpdate> = sym(lib, &wname);
                let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

                let mut rng = Rng::new(seed ^ 0xCC);
                let a = obj_new();
                let b = obj_new();
                for _ in 0..rng.below(8) {
                    let k = format!("k{}\0", rng.below(12));
                    set_new(a, k.as_ptr() as *const c_char, int_new(rng.next() as JsonInt));
                }
                for _ in 0..rng.below(8) {
                    let k = format!("k{}\0", rng.below(12));
                    set_new(b, k.as_ptr() as *const c_char, int_new(rng.next() as JsonInt));
                }
                update(a, b);
                let out = dump(lib, a, JSON_SORT_KEYS);
                delete(a);
                delete(b);
                out
            });
        }
    }
}

#[test]
fn row_24_copy_deep_copy() {
    let l = libs();
    for &fname in &[b"json_copy".as_slice(), b"json_deep_copy".as_slice()] {
        for seed in 0..100u64 {
            let compare = |lib: &libloading::Library| unsafe {
                let obj_new: libloading::Symbol<FnObject> = sym(lib, b"json_object");
                let arr_new: libloading::Symbol<FnArray> = sym(lib, b"json_array");
                let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
                let str_new: libloading::Symbol<FnString> = sym(lib, b"json_string");
                let set_new: libloading::Symbol<FnObjSetNew> = sym(lib, b"json_object_set_new");
                let append: libloading::Symbol<FnArrAppendNew> = sym(lib, b"json_array_append_new");
                let copy: libloading::Symbol<FnCopy> = sym(lib, fname);
                let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

                let mut rng = Rng::new(seed ^ 0xDD);
                let root = obj_new();
                for _ in 0..rng.below(5) {
                    let k = format!("k{}\0", rng.below(8));
                    let inner = arr_new();
                    for _ in 0..rng.below(4) {
                        append(inner, int_new(rng.next() as JsonInt));
                    }
                    append(inner, str_new(b"hi\0".as_ptr() as *const c_char));
                    set_new(root, k.as_ptr() as *const c_char, inner);
                }
                let cp = copy(root);
                let out = dump(lib, cp, JSON_SORT_KEYS);
                delete(cp);
                delete(root);
                out
            };
            let c = compare(&l.c);
            let r = compare(&l.r);
            assert_eq!(c, r, "copy mismatch fn={}", String::from_utf8_lossy(fname));
        }
    }
}

#[test]
fn row_25_equal() {
    // Build two structures per seed and compare json_equal result between libs.
    let l = libs();
    for seed in 0..200u64 {
        let build_and_eq = |lib: &libloading::Library| unsafe {
            let arr_new: libloading::Symbol<FnArray> = sym(lib, b"json_array");
            let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
            let str_new: libloading::Symbol<FnString> = sym(lib, b"json_string");
            let real_new: libloading::Symbol<FnReal> = sym(lib, b"json_real");
            let append: libloading::Symbol<FnArrAppendNew> = sym(lib, b"json_array_append_new");
            let equal: libloading::Symbol<FnEqual> = sym(lib, b"json_equal");
            let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

            let mut rng = Rng::new(seed ^ 0xEE);
            let mk = |rng: &mut Rng| -> *mut c_void {
                let a = arr_new();
                for _ in 0..rng.below(5) {
                    match rng.below(3) {
                        0 => { append(a, int_new(rng.next() as JsonInt)); }
                        1 => { append(a, real_new((rng.next() % 1000) as c_double)); }
                        _ => { append(a, str_new(b"x\0".as_ptr() as *const c_char)); }
                    }
                }
                a
            };
            let mut rng2 = Rng::new(seed ^ 0xEE);
            let a = mk(&mut rng);
            let b = mk(&mut rng2); // identical construction → equal
            let eq = equal(a, b);
            delete(a);
            delete(b);
            eq
        };
        let c = build_and_eq(&l.c);
        let r = build_and_eq(&l.r);
        assert_eq!(c, r, "json_equal mismatch seed={seed}");
    }
}

#[test]
fn row_31_getters() {
    // integer/real/string getters roundtrip identically.
    let l = libs();
    for seed in 0..300u64 {
        let mut rng = Rng::new(seed ^ 0xF0);
        let iv = rng.next() as JsonInt;
        let rv = f64::from_bits(rng.next());
        let rv = if rv.is_finite() { rv } else { 1.25 };

        let getvals = |lib: &libloading::Library| unsafe {
            let int_new: libloading::Symbol<FnInteger> = sym(lib, b"json_integer");
            let real_new: libloading::Symbol<FnReal> = sym(lib, b"json_real");
            let str_new: libloading::Symbol<FnStringn> = sym(lib, b"json_stringn");
            let iget: libloading::Symbol<FnPtrToInt> = sym(lib, b"json_integer_value");
            let rget: libloading::Symbol<FnPtrToDouble> = sym(lib, b"json_real_value");
            let nget: libloading::Symbol<FnPtrToDouble> = sym(lib, b"json_number_value");
            let sval: libloading::Symbol<unsafe extern "C" fn(*const c_void) -> *const c_char> = sym(lib, b"json_string_value");
            let slen: libloading::Symbol<FnPtrToSize> = sym(lib, b"json_string_length");
            let delete: libloading::Symbol<FnDelete> = sym(lib, b"json_delete");

            let ji = int_new(iv);
            let jr = real_new(rv);
            let sbytes = b"he\0llo\xc3\xa9";
            let js = str_new(sbytes.as_ptr() as *const c_char, sbytes.len());

            let gi = iget(ji);
            let gr = rget(jr).to_bits();
            let gn = nget(ji).to_bits();
            let sl = if js.is_null() { 0 } else { slen(js) };
            let sv = if js.is_null() { None } else {
                let p = sval(js);
                Some(std::slice::from_raw_parts(p as *const u8, sl).to_vec())
            };
            delete(ji);
            delete(jr);
            if !js.is_null() { delete(js); }
            (gi, gr, gn, sl, sv)
        };
        let c = getvals(&l.c);
        let r = getvals(&l.r);
        assert_eq!(c, r, "getters mismatch seed={seed}");
    }
}
