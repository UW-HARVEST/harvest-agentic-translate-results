use crate::*;
use std::os::raw::{c_char, c_double, c_int};

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
    let len = libc::strlen(out) + 5;
    let buf = libc::malloc(len) as *mut c_char;
    if buf.is_null() {
        libc::printf(c"Failed to allocate memory.\n".as_ptr());
        libc::exit(1);
    }
    let len_fail = libc::strlen(out);
    let buf_fail = libc::malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        libc::printf(c"Failed to allocate memory.\n".as_ptr());
        libc::exit(1);
    }
    if cJSON_PrintPreallocated(root, buf, len as c_int, 1) == 0 {
        libc::printf(c"cJSON_PrintPreallocated failed!\n".as_ptr());
        if libc::strcmp(out, buf) != 0 {
            libc::printf(c"cJSON_PrintPreallocated not the same as cJSON_Print!\n".as_ptr());
            libc::printf(c"cJSON_Print result:\n%s\n".as_ptr(), out);
            libc::printf(c"cJSON_PrintPreallocated result:\n%s\n".as_ptr(), buf);
        }
        libc::free(out as *mut libc::c_void);
        libc::free(buf_fail as *mut libc::c_void);
        libc::free(buf as *mut libc::c_void);
        return -1;
    }
    libc::printf(c"%s\n".as_ptr(), buf);
    if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {
        libc::printf(c"cJSON_PrintPreallocated failed to show error with insufficient memory!\n".as_ptr());
        libc::printf(c"cJSON_Print result:\n%s\n".as_ptr(), out);
        libc::printf(c"cJSON_PrintPreallocated result:\n%s\n".as_ptr(), buf_fail);
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
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) {
    let mut root = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"name".as_ptr(), cJSON_CreateString(c"Jack (\"Bee\") Nimble".as_ptr()));
    let fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"format".as_ptr(), fmt);
    cJSON_AddStringToObject(fmt, c"type".as_ptr(), c"rect".as_ptr());
    cJSON_AddNumberToObject(fmt, c"width".as_ptr(), 1920.0);
    cJSON_AddNumberToObject(fmt, c"height".as_ptr(), 1080.0);
    cJSON_AddFalseToObject(fmt, c"interlace".as_ptr());
    cJSON_AddNumberToObject(fmt, c"frame rate".as_ptr(), 24.0);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateStringArray(strings, 7);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateArray();
    for i in 0..3 {
        cJSON_AddItemToArray(root, cJSON_CreateIntArray((*numbers.add(i)).as_ptr(), 3));
    }
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    let img = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"Image".as_ptr(), img);
    cJSON_AddNumberToObject(img, c"Width".as_ptr(), 800.0);
    cJSON_AddNumberToObject(img, c"Height".as_ptr(), 600.0);
    cJSON_AddStringToObject(img, c"Title".as_ptr(), c"View from 15th Floor".as_ptr());
    let thm = cJSON_CreateObject();
    cJSON_AddItemToObject(img, c"Thumbnail".as_ptr(), thm);
    cJSON_AddStringToObject(thm, c"Url".as_ptr(), c"http:/*www.example.com/image/481989943".as_ptr());
    cJSON_AddNumberToObject(thm, c"Height".as_ptr(), 125.0);
    cJSON_AddStringToObject(thm, c"Width".as_ptr(), c"100".as_ptr());
    cJSON_AddItemToObject(img, c"IDs".as_ptr(), cJSON_CreateIntArray(ids, 4));
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateArray();
    for i in 0..2 {
        let fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        cJSON_AddStringToObject(fld, c"precision".as_ptr(), (*fields.add(i)).precision);
        cJSON_AddNumberToObject(fld, c"Latitude".as_ptr(), (*fields.add(i)).lat);
        cJSON_AddNumberToObject(fld, c"Longitude".as_ptr(), (*fields.add(i)).lon);
        cJSON_AddStringToObject(fld, c"Address".as_ptr(), (*fields.add(i)).address);
        cJSON_AddStringToObject(fld, c"City".as_ptr(), (*fields.add(i)).city);
        cJSON_AddStringToObject(fld, c"State".as_ptr(), (*fields.add(i)).state);
        cJSON_AddStringToObject(fld, c"Zip".as_ptr(), (*fields.add(i)).zip);
        cJSON_AddStringToObject(fld, c"Country".as_ptr(), (*fields.add(i)).country);
    }
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    let zero: f64 = 0.0;
    cJSON_AddNumberToObject(root, c"number".as_ptr(), 1.0 / zero);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        libc::exit(1);
    }
    cJSON_Delete(root);
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) -> c_int {
    unsafe {
        libc::printf(c"Version: %s\n".as_ptr(), cJSON_Version());
        create_objects(strings, numbers, ids, fields);
    }
    0
}
