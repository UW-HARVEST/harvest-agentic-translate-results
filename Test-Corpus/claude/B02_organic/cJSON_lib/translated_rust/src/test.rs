//! Translation of test.c — exposes `driver` matching the C signature.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_double, c_int, c_void};
use core::ptr;

use crate::{
    cJSON, cJSON_AddFalseToObject, cJSON_AddItemToArray, cJSON_AddItemToObject,
    cJSON_AddNumberToObject, cJSON_AddStringToObject, cJSON_CreateArray, cJSON_CreateIntArray,
    cJSON_CreateObject, cJSON_CreateString, cJSON_CreateStringArray, cJSON_Delete, cJSON_Print,
    cJSON_PrintPreallocated, cJSON_Version,
};

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn exit(code: c_int) -> !;
}

#[repr(C)]
pub struct record {
    pub precision: *const c_char,
    pub lat: c_double,
    pub lon: c_double,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

unsafe fn print_preallocated(root: *mut cJSON) -> c_int {
    let out = cJSON_Print(root);

    let len = strlen(out) + 5;
    let buf = malloc(len) as *mut c_char;
    if buf.is_null() {
        printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        exit(1);
    }

    let len_fail = strlen(out);
    let buf_fail = malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        exit(1);
    }

    if cJSON_PrintPreallocated(root, buf, len as c_int, 1) == 0 {
        printf(b"cJSON_PrintPreallocated failed!\n\0".as_ptr() as *const c_char);
        if strcmp(out, buf) != 0 {
            printf(b"cJSON_PrintPreallocated not the same as cJSON_Print!\n\0".as_ptr()
                as *const c_char);
            printf(b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char, out);
            printf(
                b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char,
                buf,
            );
        }
        free(out as *mut c_void);
        free(buf_fail as *mut c_void);
        free(buf as *mut c_void);
        return -1;
    }

    printf(b"%s\n\0".as_ptr() as *const c_char, buf);

    if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {
        printf(
            b"cJSON_PrintPreallocated failed to show error with insufficient memory!\n\0".as_ptr()
                as *const c_char,
        );
        printf(b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char, out);
        printf(
            b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char,
            buf_fail,
        );
        free(out as *mut c_void);
        free(buf_fail as *mut c_void);
        free(buf as *mut c_void);
        return -1;
    }

    free(out as *mut c_void);
    free(buf_fail as *mut c_void);
    free(buf as *mut c_void);
    0
}

unsafe fn create_objects(
    strings: *mut *const c_char,
    numbers: *mut [c_int; 3],
    ids: *mut c_int,
    fields: *mut record,
) {
    let mut root: *mut cJSON;
    let fmt: *mut cJSON;
    let img: *mut cJSON;
    let thm: *mut cJSON;
    let mut fld: *mut cJSON;

    // C used 1.0/zero where zero is a volatile double 0.0 — produces +inf
    // Use the same construction so the float bit-pattern matches.
    let zero: f64 = {
        let mut z: f64 = 0.0;
        // mimic 'volatile double zero = 0.0' / 1.0/zero -> infinity
        let p: *mut f64 = &mut z;
        ptr::read_volatile(p)
    };

    // 1. "Video" object
    root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"name\0".as_ptr() as *const c_char,
        cJSON_CreateString(b"Jack (\"Bee\") Nimble\0".as_ptr() as *const c_char),
    );
    fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(root, b"format\0".as_ptr() as *const c_char, fmt);
    cJSON_AddStringToObject(
        fmt,
        b"type\0".as_ptr() as *const c_char,
        b"rect\0".as_ptr() as *const c_char,
    );
    cJSON_AddNumberToObject(fmt, b"width\0".as_ptr() as *const c_char, 1920.0);
    cJSON_AddNumberToObject(fmt, b"height\0".as_ptr() as *const c_char, 1080.0);
    cJSON_AddFalseToObject(fmt, b"interlace\0".as_ptr() as *const c_char);
    cJSON_AddNumberToObject(fmt, b"frame rate\0".as_ptr() as *const c_char, 24.0);

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1); // EXIT_FAILURE
    }
    cJSON_Delete(root);

    // 2. days of week array
    root = cJSON_CreateStringArray(strings, 7);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    // 3. matrix
    root = cJSON_CreateArray();
    for i in 0..3 {
        let row = numbers.add(i);
        cJSON_AddItemToArray(root, cJSON_CreateIntArray((*row).as_ptr(), 3));
    }

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    // 4. gallery item
    root = cJSON_CreateObject();
    img = cJSON_CreateObject();
    cJSON_AddItemToObject(root, b"Image\0".as_ptr() as *const c_char, img);
    cJSON_AddNumberToObject(img, b"Width\0".as_ptr() as *const c_char, 800.0);
    cJSON_AddNumberToObject(img, b"Height\0".as_ptr() as *const c_char, 600.0);
    cJSON_AddStringToObject(
        img,
        b"Title\0".as_ptr() as *const c_char,
        b"View from 15th Floor\0".as_ptr() as *const c_char,
    );
    thm = cJSON_CreateObject();
    cJSON_AddItemToObject(img, b"Thumbnail\0".as_ptr() as *const c_char, thm);
    cJSON_AddStringToObject(
        thm,
        b"Url\0".as_ptr() as *const c_char,
        b"http:/*www.example.com/image/481989943\0".as_ptr() as *const c_char,
    );
    cJSON_AddNumberToObject(thm, b"Height\0".as_ptr() as *const c_char, 125.0);
    cJSON_AddStringToObject(
        thm,
        b"Width\0".as_ptr() as *const c_char,
        b"100\0".as_ptr() as *const c_char,
    );
    cJSON_AddItemToObject(
        img,
        b"IDs\0".as_ptr() as *const c_char,
        cJSON_CreateIntArray(ids, 4),
    );

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    // 5. records array
    root = cJSON_CreateArray();
    for i in 0..2 {
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        let f = fields.add(i);
        cJSON_AddStringToObject(
            fld,
            b"precision\0".as_ptr() as *const c_char,
            (*f).precision,
        );
        cJSON_AddNumberToObject(fld, b"Latitude\0".as_ptr() as *const c_char, (*f).lat);
        cJSON_AddNumberToObject(fld, b"Longitude\0".as_ptr() as *const c_char, (*f).lon);
        cJSON_AddStringToObject(fld, b"Address\0".as_ptr() as *const c_char, (*f).address);
        cJSON_AddStringToObject(fld, b"City\0".as_ptr() as *const c_char, (*f).city);
        cJSON_AddStringToObject(fld, b"State\0".as_ptr() as *const c_char, (*f).state);
        cJSON_AddStringToObject(fld, b"Zip\0".as_ptr() as *const c_char, (*f).zip);
        cJSON_AddStringToObject(fld, b"Country\0".as_ptr() as *const c_char, (*f).country);
    }

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    // 6. number with infinity
    root = cJSON_CreateObject();
    cJSON_AddNumberToObject(root, b"number\0".as_ptr() as *const c_char, 1.0 / zero);

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *mut *const c_char,
    numbers: *mut [c_int; 3],
    ids: *mut c_int,
    fields: *mut record,
) -> c_int {
    printf(
        b"Version: %s\n\0".as_ptr() as *const c_char,
        cJSON_Version(),
    );

    create_objects(strings, numbers, ids, fields);

    0
}
