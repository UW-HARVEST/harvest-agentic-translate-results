extern "C" {
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn exit(__status: libc::c_int) -> !;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn strlen(__s: *const libc::c_char) -> size_t;
    
    
    
    
    
    
    
    
    
    
    
    
    
    
}
pub use crate::src::cJSON::cJSON_AddFalseToObject;
pub use crate::src::cJSON::cJSON_AddItemToArray;
pub use crate::src::cJSON::cJSON_AddItemToObject;
pub use crate::src::cJSON::cJSON_AddNumberToObject;
pub use crate::src::cJSON::cJSON_AddStringToObject;
pub use crate::src::cJSON::cJSON_CreateArray;
pub use crate::src::cJSON::cJSON_CreateIntArray;
pub use crate::src::cJSON::cJSON_CreateObject;
pub use crate::src::cJSON::cJSON_CreateString;
pub use crate::src::cJSON::cJSON_CreateStringArray;
pub use crate::src::cJSON::cJSON_Delete;
pub use crate::src::cJSON::cJSON_Print;
pub use crate::src::cJSON::cJSON_PrintPreallocated;
pub use crate::src::cJSON::cJSON_Version;
pub use crate::src::cJSON::size_t;
// #[derive(Copy, Clone)]

pub use crate::src::cJSON::cJSON;
pub use crate::src::cJSON::cJSON_bool;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct record {
    pub precision: *const libc::c_char,
    pub lat: libc::c_double,
    pub lon: libc::c_double,
    pub address: *const libc::c_char,
    pub city: *const libc::c_char,
    pub state: *const libc::c_char,
    pub zip: *const libc::c_char,
    pub country: *const libc::c_char,
}
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const EXIT_FAILURE: libc::c_int = 1 as libc::c_int;
unsafe extern "C" fn print_preallocated(mut root: *mut cJSON) -> libc::c_int {
    let mut out: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut buf: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut buf_fail: *mut libc::c_char = std::ptr::null_mut::<libc::c_char>();
    let mut len: size_t = 0 as size_t;
    let mut len_fail: size_t = 0 as size_t;
    out = cJSON_Print(root);
    len = strlen(out).wrapping_add(5 as size_t);
    buf = malloc(len) as *mut libc::c_char;
    if buf.is_null() {
        printf(b"Failed to allocate memory.\n\0" as *const u8 as *const libc::c_char);
        exit(1 as libc::c_int);
    }
    len_fail = strlen(out);
    buf_fail = malloc(len_fail) as *mut libc::c_char;
    if buf_fail.is_null() {
        printf(b"Failed to allocate memory.\n\0" as *const u8 as *const libc::c_char);
        exit(1 as libc::c_int);
    }
    if cJSON_PrintPreallocated(root, buf, len as libc::c_int, 1 as cJSON_bool) == 0 {
        printf(b"cJSON_PrintPreallocated failed!\n\0" as *const u8 as *const libc::c_char);
        if strcmp(out, buf) != 0 as libc::c_int {
            printf(
                b"cJSON_PrintPreallocated not the same as cJSON_Print!\n\0" as *const u8
                    as *const libc::c_char,
            );
            printf(
                b"cJSON_Print result:\n%s\n\0" as *const u8 as *const libc::c_char,
                out,
            );
            printf(
                b"cJSON_PrintPreallocated result:\n%s\n\0" as *const u8
                    as *const libc::c_char,
                buf,
            );
        }
        free(out as *mut libc::c_void);
        free(buf_fail as *mut libc::c_void);
        free(buf as *mut libc::c_void);
        return -(1 as libc::c_int);
    }
    printf(b"%s\n\0" as *const u8 as *const libc::c_char, buf);
    if cJSON_PrintPreallocated(
        root,
        buf_fail,
        len_fail as libc::c_int,
        1 as cJSON_bool,
    ) != 0
    {
        printf(
            b"cJSON_PrintPreallocated failed to show error with insufficient memory!\n\0"
                as *const u8 as *const libc::c_char,
        );
        printf(
            b"cJSON_Print result:\n%s\n\0" as *const u8 as *const libc::c_char,
            out,
        );
        printf(
            b"cJSON_PrintPreallocated result:\n%s\n\0" as *const u8 as *const libc::c_char,
            buf_fail,
        );
        free(out as *mut libc::c_void);
        free(buf_fail as *mut libc::c_void);
        free(buf as *mut libc::c_void);
        return -(1 as libc::c_int);
    }
    free(out as *mut libc::c_void);
    free(buf_fail as *mut libc::c_void);
    free(buf as *mut libc::c_void);
    return 0 as libc::c_int;
}
unsafe extern "C" fn create_objects(
    mut strings: *mut *const libc::c_char,
    mut numbers: *mut [libc::c_int; 3],
    mut ids: *mut libc::c_int,
    mut fields: *mut record,
) {
    let mut root: *mut cJSON = std::ptr::null_mut::<cJSON>();
    let mut fmt: *mut cJSON = std::ptr::null_mut::<cJSON>();
    let mut img: *mut cJSON = std::ptr::null_mut::<cJSON>();
    let mut thm: *mut cJSON = std::ptr::null_mut::<cJSON>();
    let mut fld: *mut cJSON = std::ptr::null_mut::<cJSON>();
    let mut i: libc::c_int = 0 as libc::c_int;
    let mut zero: libc::c_double = 0.0f64;
    root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"name\0" as *const u8 as *const libc::c_char,
        cJSON_CreateString(b"Jack (\"Bee\") Nimble\0" as *const u8 as *const libc::c_char),
    );
    fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"format\0" as *const u8 as *const libc::c_char,
        fmt,
    );
    cJSON_AddStringToObject(
        fmt,
        b"type\0" as *const u8 as *const libc::c_char,
        b"rect\0" as *const u8 as *const libc::c_char,
    );
    cJSON_AddNumberToObject(
        fmt,
        b"width\0" as *const u8 as *const libc::c_char,
        1920 as libc::c_int as libc::c_double,
    );
    cJSON_AddNumberToObject(
        fmt,
        b"height\0" as *const u8 as *const libc::c_char,
        1080 as libc::c_int as libc::c_double,
    );
    cJSON_AddFalseToObject(
        fmt,
        b"interlace\0" as *const u8 as *const libc::c_char,
    );
    cJSON_AddNumberToObject(
        fmt,
        b"frame rate\0" as *const u8 as *const libc::c_char,
        24 as libc::c_int as libc::c_double,
    );
    if print_preallocated(root) != 0 as libc::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateStringArray(
        strings as *const *const libc::c_char,
        7 as libc::c_int,
    );
    if print_preallocated(root) != 0 as libc::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateArray();
    i = 0 as libc::c_int;
    while i < 3 as libc::c_int {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray(
                &raw mut *numbers.offset(i as isize) as *mut libc::c_int,
                3 as libc::c_int,
            ),
        );
        i += 1;
    }
    if print_preallocated(root) != 0 as libc::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateObject();
    img = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"Image\0" as *const u8 as *const libc::c_char,
        img,
    );
    cJSON_AddNumberToObject(
        img,
        b"Width\0" as *const u8 as *const libc::c_char,
        800 as libc::c_int as libc::c_double,
    );
    cJSON_AddNumberToObject(
        img,
        b"Height\0" as *const u8 as *const libc::c_char,
        600 as libc::c_int as libc::c_double,
    );
    cJSON_AddStringToObject(
        img,
        b"Title\0" as *const u8 as *const libc::c_char,
        b"View from 15th Floor\0" as *const u8 as *const libc::c_char,
    );
    thm = cJSON_CreateObject();
    cJSON_AddItemToObject(
        img,
        b"Thumbnail\0" as *const u8 as *const libc::c_char,
        thm,
    );
    cJSON_AddStringToObject(
        thm,
        b"Url\0" as *const u8 as *const libc::c_char,
        b"http:/*www.example.com/image/481989943\0" as *const u8 as *const libc::c_char,
    );
    cJSON_AddNumberToObject(
        thm,
        b"Height\0" as *const u8 as *const libc::c_char,
        125 as libc::c_int as libc::c_double,
    );
    cJSON_AddStringToObject(
        thm,
        b"Width\0" as *const u8 as *const libc::c_char,
        b"100\0" as *const u8 as *const libc::c_char,
    );
    cJSON_AddItemToObject(
        img,
        b"IDs\0" as *const u8 as *const libc::c_char,
        cJSON_CreateIntArray(ids as *const libc::c_int, 4 as libc::c_int),
    );
    if print_preallocated(root) != 0 as libc::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateArray();
    i = 0 as libc::c_int;
    while i < 2 as libc::c_int {
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        cJSON_AddStringToObject(
            fld,
            b"precision\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).precision,
        );
        cJSON_AddNumberToObject(
            fld,
            b"Latitude\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).lat,
        );
        cJSON_AddNumberToObject(
            fld,
            b"Longitude\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).lon,
        );
        cJSON_AddStringToObject(
            fld,
            b"Address\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).address,
        );
        cJSON_AddStringToObject(
            fld,
            b"City\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).city,
        );
        cJSON_AddStringToObject(
            fld,
            b"State\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).state,
        );
        cJSON_AddStringToObject(
            fld,
            b"Zip\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).zip,
        );
        cJSON_AddStringToObject(
            fld,
            b"Country\0" as *const u8 as *const libc::c_char,
            (*fields.offset(i as isize)).country,
        );
        i += 1;
    }
    if print_preallocated(root) != 0 as libc::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateObject();
    cJSON_AddNumberToObject(
        root,
        b"number\0" as *const u8 as *const libc::c_char,
        1.0f64 / zero,
    );
    if print_preallocated(root) != 0 as libc::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut strings: *mut *const libc::c_char,
    mut numbers: *mut [libc::c_int; 3],
    mut ids: *mut libc::c_int,
    mut fields: *mut record,
) -> libc::c_int {
    printf(
        b"Version: %s\n\0" as *const u8 as *const libc::c_char,
        cJSON_Version(),
    );
    create_objects(strings, numbers, ids, fields);
    return 0 as libc::c_int;
}
