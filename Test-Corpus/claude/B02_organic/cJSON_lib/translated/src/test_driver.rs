//! Translation of `c_src/test.c`.
//!
//! Exports the `driver` symbol (the C source declares it as
//! `int CJSON_CDECL CJSON_PUBLIC(driver)(...)`, which expands to the plain
//! linker name `driver`).

use core::ffi::{c_char, c_int, c_void};
use core::ptr::null_mut;

use crate::ffi::*;
use crate::*;

/* Used by some code below as an example datatype. */
#[repr(C)]
pub struct record {
    pub precision: *const c_char,
    pub lat: f64,
    pub lon: f64,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

/* Create a bunch of objects as demonstration. */
unsafe fn print_preallocated(root: *mut cJSON) -> c_int {
    /* declarations */
    let out: *mut c_char;
    let buf: *mut c_char;
    let buf_fail: *mut c_char;
    let len: usize;
    let len_fail: usize;

    /* formatted print */
    out = cJSON_Print(root);

    /* create buffer to succeed */
    /* the extra 5 bytes are because of inaccuracies when reserving memory */
    len = strlen(out) + 5;
    buf = malloc(len) as *mut c_char;
    if buf.is_null() {
        printf(c"Failed to allocate memory.\n".as_ptr());
        exit(1);
    }

    /* create buffer to fail */
    len_fail = strlen(out);
    buf_fail = malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        printf(c"Failed to allocate memory.\n".as_ptr());
        exit(1);
    }

    /* Print to buffer */
    if cJSON_PrintPreallocated(root, buf, len as c_int, 1) == 0 {
        printf(c"cJSON_PrintPreallocated failed!\n".as_ptr());
        if strcmp(out, buf) != 0 {
            printf(c"cJSON_PrintPreallocated not the same as cJSON_Print!\n".as_ptr());
            printf(c"cJSON_Print result:\n%s\n".as_ptr(), out);
            printf(c"cJSON_PrintPreallocated result:\n%s\n".as_ptr(), buf);
        }
        free(out as *mut c_void);
        free(buf_fail as *mut c_void);
        free(buf as *mut c_void);
        return -1;
    }

    /* success */
    printf(c"%s\n".as_ptr(), buf);

    /* force it to fail */
    if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {
        printf(c"cJSON_PrintPreallocated failed to show error with insufficient memory!\n".as_ptr());
        printf(c"cJSON_Print result:\n%s\n".as_ptr(), out);
        printf(c"cJSON_PrintPreallocated result:\n%s\n".as_ptr(), buf_fail);
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

/* Create a bunch of objects as demonstration. */
unsafe fn create_objects(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) {
    /* declare a few. */
    let mut root: *mut cJSON;
    let fmt: *mut cJSON;
    let img: *mut cJSON;
    let thm: *mut cJSON;
    let mut fld: *mut cJSON;
    let mut i: c_int;

    let zero: f64 = 0.0;

    /* Here we construct some JSON standards, from the JSON site. */

    /* Our "Video" datatype: */
    root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        c"name".as_ptr(),
        cJSON_CreateString(c"Jack (\"Bee\") Nimble".as_ptr()),
    );
    fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"format".as_ptr(), fmt);
    cJSON_AddStringToObject(fmt, c"type".as_ptr(), c"rect".as_ptr());
    cJSON_AddNumberToObject(fmt, c"width".as_ptr(), 1920.0);
    cJSON_AddNumberToObject(fmt, c"height".as_ptr(), 1080.0);
    cJSON_AddFalseToObject(fmt, c"interlace".as_ptr());
    cJSON_AddNumberToObject(fmt, c"frame rate".as_ptr(), 24.0);

    /* Print to text */
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    /* Our "days of the week" array: */
    root = cJSON_CreateStringArray(strings, 7);

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    /* Our matrix: */
    root = cJSON_CreateArray();
    i = 0;
    while i < 3 {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray((*numbers.offset(i as isize)).as_ptr(), 3),
        );
        i += 1;
    }

    /* cJSON_ReplaceItemInArray(root, 1, cJSON_CreateString("Replacement")); */

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    /* Our "gallery" item: */
    root = cJSON_CreateObject();
    img = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"Image".as_ptr(), img);
    cJSON_AddNumberToObject(img, c"Width".as_ptr(), 800.0);
    cJSON_AddNumberToObject(img, c"Height".as_ptr(), 600.0);
    cJSON_AddStringToObject(img, c"Title".as_ptr(), c"View from 15th Floor".as_ptr());
    thm = cJSON_CreateObject();
    cJSON_AddItemToObject(img, c"Thumbnail".as_ptr(), thm);
    cJSON_AddStringToObject(
        thm,
        c"Url".as_ptr(),
        c"http:/*www.example.com/image/481989943".as_ptr(),
    );
    cJSON_AddNumberToObject(thm, c"Height".as_ptr(), 125.0);
    cJSON_AddStringToObject(thm, c"Width".as_ptr(), c"100".as_ptr());
    cJSON_AddItemToObject(img, c"IDs".as_ptr(), cJSON_CreateIntArray(ids, 4));

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    /* Our array of "records": */
    root = cJSON_CreateArray();
    i = 0;
    while i < 2 {
        let f = fields.offset(i as isize);
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        cJSON_AddStringToObject(fld, c"precision".as_ptr(), (*f).precision);
        cJSON_AddNumberToObject(fld, c"Latitude".as_ptr(), (*f).lat);
        cJSON_AddNumberToObject(fld, c"Longitude".as_ptr(), (*f).lon);
        cJSON_AddStringToObject(fld, c"Address".as_ptr(), (*f).address);
        cJSON_AddStringToObject(fld, c"City".as_ptr(), (*f).city);
        cJSON_AddStringToObject(fld, c"State".as_ptr(), (*f).state);
        cJSON_AddStringToObject(fld, c"Zip".as_ptr(), (*f).zip);
        cJSON_AddStringToObject(fld, c"Country".as_ptr(), (*f).country);
        i += 1;
    }

    /* cJSON_ReplaceItemInObject(cJSON_GetArrayItem(root, 1), "City", cJSON_CreateIntArray(ids, 4)); */

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    let volatile_zero = core::ptr::read_volatile(&zero as *const f64);
    cJSON_AddNumberToObject(root, c"number".as_ptr(), 1.0 / volatile_zero);

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    let _ = null_mut::<cJSON>();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) -> c_int {
    /* print the version */
    printf(c"Version: %s\n".as_ptr(), cJSON_Version());

    /* Now some samplecode for building objects concisely: */
    create_objects(strings, numbers, ids, fields);

    0
}
