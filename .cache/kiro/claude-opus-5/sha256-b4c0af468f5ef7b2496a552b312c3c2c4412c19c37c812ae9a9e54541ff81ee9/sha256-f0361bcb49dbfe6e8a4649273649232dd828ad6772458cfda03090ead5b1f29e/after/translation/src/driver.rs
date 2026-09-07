//! Rust translation of c_src/test.c
//!
//! Exposes the `driver` symbol with the exact same behaviour (and stdout
//! output) as the C original.

use core::ffi::{c_char, c_double, c_int, c_void};
use core::ptr;

use crate::libc;
use crate::{
    cJSON, cJSON_AddFalseToObject, cJSON_AddItemToArray, cJSON_AddItemToObject,
    cJSON_AddNumberToObject, cJSON_AddStringToObject, cJSON_CreateArray, cJSON_CreateIntArray,
    cJSON_CreateObject, cJSON_CreateString, cJSON_CreateStringArray, cJSON_Delete, cJSON_Print,
    cJSON_PrintPreallocated, cJSON_Version,
};

/* Used by some code below as an example datatype. */
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

const EXIT_FAILURE: c_int = 1;

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
    len = libc::strlen(out) + 5;
    buf = libc::malloc(len) as *mut c_char;
    if buf.is_null() {
        libc::printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        libc::exit(1);
    }

    /* create buffer to fail */
    len_fail = libc::strlen(out);
    buf_fail = libc::malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        libc::printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        libc::exit(1);
    }

    /* Print to buffer */
    if cJSON_PrintPreallocated(root, buf, len as c_int, 1) == 0 {
        libc::printf(b"cJSON_PrintPreallocated failed!\n\0".as_ptr() as *const c_char);
        if libc::strcmp(out, buf) != 0 {
            libc::printf(
                b"cJSON_PrintPreallocated not the same as cJSON_Print!\n\0".as_ptr()
                    as *const c_char,
            );
            libc::printf(
                b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char,
                out,
            );
            libc::printf(
                b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char,
                buf,
            );
        }
        libc::free(out as *mut c_void);
        libc::free(buf_fail as *mut c_void);
        libc::free(buf as *mut c_void);
        return -1;
    }

    /* success */
    libc::printf(b"%s\n\0".as_ptr() as *const c_char, buf);

    /* force it to fail */
    if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {
        libc::printf(
            b"cJSON_PrintPreallocated failed to show error with insufficient memory!\n\0".as_ptr()
                as *const c_char,
        );
        libc::printf(
            b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char,
            out,
        );
        libc::printf(
            b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char,
            buf_fail,
        );
        libc::free(out as *mut c_void);
        libc::free(buf_fail as *mut c_void);
        libc::free(buf as *mut c_void);
        return -1;
    }

    libc::free(out as *mut c_void);
    libc::free(buf_fail as *mut c_void);
    libc::free(buf as *mut c_void);
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
    let mut fld: *mut cJSON = ptr::null_mut();
    let mut i: c_int;

    let zero: c_double = 0.0;

    /* Here we construct some JSON standards, from the JSON site. */

    /* Our "Video" datatype: */
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

    /* Print to text */
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    /* Our "days of the week" array: */
    root = cJSON_CreateStringArray(strings, 7);

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    /* Our matrix: */
    root = cJSON_CreateArray();
    i = 0;
    while i < 3 {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray(numbers.add(i as usize) as *const c_int, 3),
        );
        i += 1;
    }

    /* cJSON_ReplaceItemInArray(root, 1, cJSON_CreateString("Replacement")); */

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    /* Our "gallery" item: */
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
        libc::exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    /* Our array of "records": */
    root = cJSON_CreateArray();
    i = 0;
    while i < 2 {
        let f = fields.add(i as usize);
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
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
        i += 1;
    }

    let _ = &mut fld;

    /* cJSON_ReplaceItemInObject(cJSON_GetArrayItem(root, 1), "City", cJSON_CreateIntArray(ids, 4)); */

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    let zero_read = ptr::read_volatile(&zero);
    cJSON_AddNumberToObject(
        root,
        b"number\0".as_ptr() as *const c_char,
        1.0 / zero_read,
    );

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) -> c_int {
    /* print the version */
    libc::printf(
        b"Version: %s\n\0".as_ptr() as *const c_char,
        cJSON_Version(),
    );

    /* Now some samplecode for building objects concisely: */
    create_objects(strings, numbers, ids, fields);

    0
}
