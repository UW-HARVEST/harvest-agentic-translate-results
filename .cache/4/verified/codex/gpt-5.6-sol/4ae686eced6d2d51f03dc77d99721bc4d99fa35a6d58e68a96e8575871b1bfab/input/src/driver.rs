use crate::internal::*;
use crate::print::{cJSON_Print, cJSON_PrintPreallocated};
use crate::tree::*;
use std::ffi::{c_char, c_double, c_int};
use std::ptr;

#[repr(C)]
pub struct Record {
    precision: *const c_char,
    lat: c_double,
    lon: c_double,
    address: *const c_char,
    city: *const c_char,
    state: *const c_char,
    zip: *const c_char,
    country: *const c_char,
}

unsafe fn print_preallocated(root: *mut cJSON) -> c_int {
    let output = cJSON_Print(root);
    let length = strlen(output) + 5;
    let buffer = malloc(length) as *mut c_char;
    if buffer.is_null() {
        printf(c"Failed to allocate memory.\n".as_ptr());
        exit(1);
    }

    let failed_length = strlen(output);
    let failed_buffer = malloc(failed_length) as *mut c_char;
    if failed_buffer.is_null() {
        printf(c"Failed to allocate memory.\n".as_ptr());
        exit(1);
    }

    if cJSON_PrintPreallocated(root, buffer, length as c_int, 1) == 0 {
        printf(c"cJSON_PrintPreallocated failed!\n".as_ptr());
        if strcmp(output, buffer) != 0 {
            printf(c"cJSON_PrintPreallocated not the same as cJSON_Print!\n".as_ptr());
            printf(c"cJSON_Print result:\n%s\n".as_ptr(), output);
            printf(c"cJSON_PrintPreallocated result:\n%s\n".as_ptr(), buffer);
        }
        free(output.cast());
        free(failed_buffer.cast());
        free(buffer.cast());
        return -1;
    }

    printf(c"%s\n".as_ptr(), buffer);
    if cJSON_PrintPreallocated(root, failed_buffer, failed_length as c_int, 1) != 0 {
        printf(
            c"cJSON_PrintPreallocated failed to show error with insufficient memory!\n".as_ptr(),
        );
        printf(c"cJSON_Print result:\n%s\n".as_ptr(), output);
        printf(
            c"cJSON_PrintPreallocated result:\n%s\n".as_ptr(),
            failed_buffer,
        );
        free(output.cast());
        free(failed_buffer.cast());
        free(buffer.cast());
        return -1;
    }

    free(output.cast());
    free(failed_buffer.cast());
    free(buffer.cast());
    0
}

unsafe fn create_objects(
    strings: *const *const c_char,
    numbers: *mut [c_int; 3],
    ids: *mut c_int,
    fields: *mut Record,
) {
    let mut root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        c"name".as_ptr(),
        cJSON_CreateString(c"Jack (\"Bee\") Nimble".as_ptr()),
    );
    let format = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"format".as_ptr(), format);
    cJSON_AddStringToObject(format, c"type".as_ptr(), c"rect".as_ptr());
    cJSON_AddNumberToObject(format, c"width".as_ptr(), 1920.0);
    cJSON_AddNumberToObject(format, c"height".as_ptr(), 1080.0);
    cJSON_AddFalseToObject(format, c"interlace".as_ptr());
    cJSON_AddNumberToObject(format, c"frame rate".as_ptr(), 24.0);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateStringArray(strings, 7);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateArray();
    for index in 0..3 {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray((*numbers.add(index)).as_ptr(), 3),
        );
    }
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    let image = cJSON_CreateObject();
    cJSON_AddItemToObject(root, c"Image".as_ptr(), image);
    cJSON_AddNumberToObject(image, c"Width".as_ptr(), 800.0);
    cJSON_AddNumberToObject(image, c"Height".as_ptr(), 600.0);
    cJSON_AddStringToObject(image, c"Title".as_ptr(), c"View from 15th Floor".as_ptr());
    let thumbnail = cJSON_CreateObject();
    cJSON_AddItemToObject(image, c"Thumbnail".as_ptr(), thumbnail);
    cJSON_AddStringToObject(
        thumbnail,
        c"Url".as_ptr(),
        c"http:/*www.example.com/image/481989943".as_ptr(),
    );
    cJSON_AddNumberToObject(thumbnail, c"Height".as_ptr(), 125.0);
    cJSON_AddStringToObject(thumbnail, c"Width".as_ptr(), c"100".as_ptr());
    cJSON_AddItemToObject(image, c"IDs".as_ptr(), cJSON_CreateIntArray(ids, 4));
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateArray();
    for index in 0..2 {
        let field = cJSON_CreateObject();
        let source = fields.add(index);
        cJSON_AddItemToArray(root, field);
        cJSON_AddStringToObject(field, c"precision".as_ptr(), (*source).precision);
        cJSON_AddNumberToObject(field, c"Latitude".as_ptr(), (*source).lat);
        cJSON_AddNumberToObject(field, c"Longitude".as_ptr(), (*source).lon);
        cJSON_AddStringToObject(field, c"Address".as_ptr(), (*source).address);
        cJSON_AddStringToObject(field, c"City".as_ptr(), (*source).city);
        cJSON_AddStringToObject(field, c"State".as_ptr(), (*source).state);
        cJSON_AddStringToObject(field, c"Zip".as_ptr(), (*source).zip);
        cJSON_AddStringToObject(field, c"Country".as_ptr(), (*source).country);
    }
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);

    root = cJSON_CreateObject();
    let zero = ptr::read_volatile(&0.0f64);
    cJSON_AddNumberToObject(root, c"number".as_ptr(), 1.0 / zero);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(1);
    }
    cJSON_Delete(root);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *mut [c_int; 3],
    ids: *mut c_int,
    fields: *mut Record,
) -> c_int {
    printf(c"Version: %s\n".as_ptr(), cJSON_Version());
    create_objects(strings, numbers, ids, fields);
    0
}
