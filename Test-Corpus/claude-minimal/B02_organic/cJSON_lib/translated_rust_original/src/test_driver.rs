// Direct Rust translation of test.c
use crate::cjson::*;
use libc::{c_char, c_double, c_int};
use std::ptr;

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
    let out: *mut c_char;
    let buf: *mut c_char;
    let buf_fail: *mut c_char;
    let len: usize;
    let len_fail: usize;

    out = cJSON_Print(root);

    len = libc::strlen(out) + 5;
    buf = libc::malloc(len) as *mut c_char;
    if buf.is_null() {
        libc::printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        libc::exit(1);
    }

    len_fail = libc::strlen(out);
    buf_fail = libc::malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        libc::printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        libc::exit(1);
    }

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
        libc::free(out as *mut libc::c_void);
        libc::free(buf_fail as *mut libc::c_void);
        libc::free(buf as *mut libc::c_void);
        return -1;
    }

    libc::printf(b"%s\n\0".as_ptr() as *const c_char, buf);

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
        libc::free(out as *mut libc::c_void);
        libc::free(buf_fail as *mut libc::c_void);
        libc::free(buf as *mut libc::c_void);
        return -1;
    }

    libc::free(out as *mut libc::c_void);
    libc::free(buf_fail as *mut libc::c_void);
    libc::free(buf as *mut libc::c_void);
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
    let mut fld: *mut cJSON = ptr::null_mut();

    let zero: f64 = 0.0;

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
        libc::exit(libc::EXIT_FAILURE);
    }
    cJSON_Delete(root);

    root = cJSON_CreateStringArray(strings as *const *const c_char, 7);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(libc::EXIT_FAILURE);
    }
    cJSON_Delete(root);

    root = cJSON_CreateArray();
    for i in 0..3isize {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray((*numbers.offset(i)).as_ptr(), 3),
        );
    }
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(libc::EXIT_FAILURE);
    }
    cJSON_Delete(root);

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
        libc::exit(libc::EXIT_FAILURE);
    }
    cJSON_Delete(root);

    root = cJSON_CreateArray();
    for i in 0..2isize {
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        cJSON_AddStringToObject(
            fld,
            b"precision\0".as_ptr() as *const c_char,
            (*fields.offset(i)).precision,
        );
        cJSON_AddNumberToObject(
            fld,
            b"Latitude\0".as_ptr() as *const c_char,
            (*fields.offset(i)).lat,
        );
        cJSON_AddNumberToObject(
            fld,
            b"Longitude\0".as_ptr() as *const c_char,
            (*fields.offset(i)).lon,
        );
        cJSON_AddStringToObject(
            fld,
            b"Address\0".as_ptr() as *const c_char,
            (*fields.offset(i)).address,
        );
        cJSON_AddStringToObject(
            fld,
            b"City\0".as_ptr() as *const c_char,
            (*fields.offset(i)).city,
        );
        cJSON_AddStringToObject(
            fld,
            b"State\0".as_ptr() as *const c_char,
            (*fields.offset(i)).state,
        );
        cJSON_AddStringToObject(
            fld,
            b"Zip\0".as_ptr() as *const c_char,
            (*fields.offset(i)).zip,
        );
        cJSON_AddStringToObject(
            fld,
            b"Country\0".as_ptr() as *const c_char,
            (*fields.offset(i)).country,
        );
    }

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(libc::EXIT_FAILURE);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    cJSON_AddNumberToObject(
        root,
        b"number\0".as_ptr() as *const c_char,
        1.0 / zero,
    );

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(libc::EXIT_FAILURE);
    }
    cJSON_Delete(root);

    let _ = ptr::null::<()>();
    let _ = (fld, img, thm, fmt);
}

#[no_mangle]
pub unsafe extern "C" fn driver(
    strings: *mut *const c_char,
    numbers: *mut [c_int; 3],
    ids: *mut c_int,
    fields: *mut record,
) -> c_int {
    libc::printf(
        b"Version: %s\n\0".as_ptr() as *const c_char,
        cJSON_Version(),
    );
    create_objects(strings, numbers, ids, fields);
    0
}
