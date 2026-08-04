extern "C" {
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn exit(__status: ::core::ffi::c_int) -> !;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn cJSON_Version() -> *const ::core::ffi::c_char;
    fn cJSON_Print(item: *const cJSON) -> *mut ::core::ffi::c_char;
    fn cJSON_PrintPreallocated(
        item: *mut cJSON,
        buffer: *mut ::core::ffi::c_char,
        length: ::core::ffi::c_int,
        format: cJSON_bool,
    ) -> cJSON_bool;
    fn cJSON_Delete(item: *mut cJSON);
    fn cJSON_CreateString(string: *const ::core::ffi::c_char) -> *mut cJSON;
    fn cJSON_CreateArray() -> *mut cJSON;
    fn cJSON_CreateObject() -> *mut cJSON;
    fn cJSON_CreateIntArray(
        numbers: *const ::core::ffi::c_int,
        count: ::core::ffi::c_int,
    ) -> *mut cJSON;
    fn cJSON_CreateStringArray(
        strings: *const *const ::core::ffi::c_char,
        count: ::core::ffi::c_int,
    ) -> *mut cJSON;
    fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool;
    fn cJSON_AddItemToObject(
        object: *mut cJSON,
        string: *const ::core::ffi::c_char,
        item: *mut cJSON,
    ) -> cJSON_bool;
    fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const ::core::ffi::c_char) -> *mut cJSON;
    fn cJSON_AddNumberToObject(
        object: *mut cJSON,
        name: *const ::core::ffi::c_char,
        number: ::core::ffi::c_double,
    ) -> *mut cJSON;
    fn cJSON_AddStringToObject(
        object: *mut cJSON,
        name: *const ::core::ffi::c_char,
        string: *const ::core::ffi::c_char,
    ) -> *mut cJSON;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_0: ::core::ffi::c_int,
    pub valuestring: *mut ::core::ffi::c_char,
    pub valueint: ::core::ffi::c_int,
    pub valuedouble: ::core::ffi::c_double,
    pub string: *mut ::core::ffi::c_char,
}
pub type cJSON_bool = ::core::ffi::c_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct record {
    pub precision: *const ::core::ffi::c_char,
    pub lat: ::core::ffi::c_double,
    pub lon: ::core::ffi::c_double,
    pub address: *const ::core::ffi::c_char,
    pub city: *const ::core::ffi::c_char,
    pub state: *const ::core::ffi::c_char,
    pub zip: *const ::core::ffi::c_char,
    pub country: *const ::core::ffi::c_char,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const EXIT_FAILURE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
unsafe extern "C" fn print_preallocated(mut root: *mut cJSON) -> ::core::ffi::c_int {
    let mut out: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut buf: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut buf_fail: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut len: size_t = 0 as size_t;
    let mut len_fail: size_t = 0 as size_t;
    out = cJSON_Print(root);
    len = strlen(out).wrapping_add(5 as size_t);
    buf = malloc(len) as *mut ::core::ffi::c_char;
    if buf.is_null() {
        printf(b"Failed to allocate memory.\n\0" as *const u8 as *const ::core::ffi::c_char);
        exit(1 as ::core::ffi::c_int);
    }
    len_fail = strlen(out);
    buf_fail = malloc(len_fail) as *mut ::core::ffi::c_char;
    if buf_fail.is_null() {
        printf(b"Failed to allocate memory.\n\0" as *const u8 as *const ::core::ffi::c_char);
        exit(1 as ::core::ffi::c_int);
    }
    if cJSON_PrintPreallocated(root, buf, len as ::core::ffi::c_int, 1 as cJSON_bool) == 0 {
        printf(b"cJSON_PrintPreallocated failed!\n\0" as *const u8 as *const ::core::ffi::c_char);
        if strcmp(out, buf) != 0 as ::core::ffi::c_int {
            printf(
                b"cJSON_PrintPreallocated not the same as cJSON_Print!\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            printf(
                b"cJSON_Print result:\n%s\n\0" as *const u8 as *const ::core::ffi::c_char,
                out,
            );
            printf(
                b"cJSON_PrintPreallocated result:\n%s\n\0" as *const u8
                    as *const ::core::ffi::c_char,
                buf,
            );
        }
        free(out as *mut ::core::ffi::c_void);
        free(buf_fail as *mut ::core::ffi::c_void);
        free(buf as *mut ::core::ffi::c_void);
        return -(1 as ::core::ffi::c_int);
    }
    printf(b"%s\n\0" as *const u8 as *const ::core::ffi::c_char, buf);
    if cJSON_PrintPreallocated(
        root,
        buf_fail,
        len_fail as ::core::ffi::c_int,
        1 as cJSON_bool,
    ) != 0
    {
        printf(
            b"cJSON_PrintPreallocated failed to show error with insufficient memory!\n\0"
                as *const u8 as *const ::core::ffi::c_char,
        );
        printf(
            b"cJSON_Print result:\n%s\n\0" as *const u8 as *const ::core::ffi::c_char,
            out,
        );
        printf(
            b"cJSON_PrintPreallocated result:\n%s\n\0" as *const u8 as *const ::core::ffi::c_char,
            buf_fail,
        );
        free(out as *mut ::core::ffi::c_void);
        free(buf_fail as *mut ::core::ffi::c_void);
        free(buf as *mut ::core::ffi::c_void);
        return -(1 as ::core::ffi::c_int);
    }
    free(out as *mut ::core::ffi::c_void);
    free(buf_fail as *mut ::core::ffi::c_void);
    free(buf as *mut ::core::ffi::c_void);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn create_objects(
    mut strings: *mut *const ::core::ffi::c_char,
    mut numbers: *mut [::core::ffi::c_int; 3],
    mut ids: *mut ::core::ffi::c_int,
    mut fields: *mut record,
) {
    let mut root: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut fmt: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut img: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut thm: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut fld: *mut cJSON = ::core::ptr::null_mut::<cJSON>();
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut zero: ::core::ffi::c_double = 0.0f64;
    root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"name\0" as *const u8 as *const ::core::ffi::c_char,
        cJSON_CreateString(b"Jack (\"Bee\") Nimble\0" as *const u8 as *const ::core::ffi::c_char),
    );
    fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"format\0" as *const u8 as *const ::core::ffi::c_char,
        fmt,
    );
    cJSON_AddStringToObject(
        fmt,
        b"type\0" as *const u8 as *const ::core::ffi::c_char,
        b"rect\0" as *const u8 as *const ::core::ffi::c_char,
    );
    cJSON_AddNumberToObject(
        fmt,
        b"width\0" as *const u8 as *const ::core::ffi::c_char,
        1920 as ::core::ffi::c_int as ::core::ffi::c_double,
    );
    cJSON_AddNumberToObject(
        fmt,
        b"height\0" as *const u8 as *const ::core::ffi::c_char,
        1080 as ::core::ffi::c_int as ::core::ffi::c_double,
    );
    cJSON_AddFalseToObject(
        fmt,
        b"interlace\0" as *const u8 as *const ::core::ffi::c_char,
    );
    cJSON_AddNumberToObject(
        fmt,
        b"frame rate\0" as *const u8 as *const ::core::ffi::c_char,
        24 as ::core::ffi::c_int as ::core::ffi::c_double,
    );
    if print_preallocated(root) != 0 as ::core::ffi::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateStringArray(
        strings as *const *const ::core::ffi::c_char,
        7 as ::core::ffi::c_int,
    );
    if print_preallocated(root) != 0 as ::core::ffi::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateArray();
    i = 0 as ::core::ffi::c_int;
    while i < 3 as ::core::ffi::c_int {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray(
                &raw mut *numbers.offset(i as isize) as *mut ::core::ffi::c_int,
                3 as ::core::ffi::c_int,
            ),
        );
        i += 1;
    }
    if print_preallocated(root) != 0 as ::core::ffi::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateObject();
    img = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"Image\0" as *const u8 as *const ::core::ffi::c_char,
        img,
    );
    cJSON_AddNumberToObject(
        img,
        b"Width\0" as *const u8 as *const ::core::ffi::c_char,
        800 as ::core::ffi::c_int as ::core::ffi::c_double,
    );
    cJSON_AddNumberToObject(
        img,
        b"Height\0" as *const u8 as *const ::core::ffi::c_char,
        600 as ::core::ffi::c_int as ::core::ffi::c_double,
    );
    cJSON_AddStringToObject(
        img,
        b"Title\0" as *const u8 as *const ::core::ffi::c_char,
        b"View from 15th Floor\0" as *const u8 as *const ::core::ffi::c_char,
    );
    thm = cJSON_CreateObject();
    cJSON_AddItemToObject(
        img,
        b"Thumbnail\0" as *const u8 as *const ::core::ffi::c_char,
        thm,
    );
    cJSON_AddStringToObject(
        thm,
        b"Url\0" as *const u8 as *const ::core::ffi::c_char,
        b"http:/*www.example.com/image/481989943\0" as *const u8 as *const ::core::ffi::c_char,
    );
    cJSON_AddNumberToObject(
        thm,
        b"Height\0" as *const u8 as *const ::core::ffi::c_char,
        125 as ::core::ffi::c_int as ::core::ffi::c_double,
    );
    cJSON_AddStringToObject(
        thm,
        b"Width\0" as *const u8 as *const ::core::ffi::c_char,
        b"100\0" as *const u8 as *const ::core::ffi::c_char,
    );
    cJSON_AddItemToObject(
        img,
        b"IDs\0" as *const u8 as *const ::core::ffi::c_char,
        cJSON_CreateIntArray(ids as *const ::core::ffi::c_int, 4 as ::core::ffi::c_int),
    );
    if print_preallocated(root) != 0 as ::core::ffi::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateArray();
    i = 0 as ::core::ffi::c_int;
    while i < 2 as ::core::ffi::c_int {
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        cJSON_AddStringToObject(
            fld,
            b"precision\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).precision,
        );
        cJSON_AddNumberToObject(
            fld,
            b"Latitude\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).lat,
        );
        cJSON_AddNumberToObject(
            fld,
            b"Longitude\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).lon,
        );
        cJSON_AddStringToObject(
            fld,
            b"Address\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).address,
        );
        cJSON_AddStringToObject(
            fld,
            b"City\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).city,
        );
        cJSON_AddStringToObject(
            fld,
            b"State\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).state,
        );
        cJSON_AddStringToObject(
            fld,
            b"Zip\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).zip,
        );
        cJSON_AddStringToObject(
            fld,
            b"Country\0" as *const u8 as *const ::core::ffi::c_char,
            (*fields.offset(i as isize)).country,
        );
        i += 1;
    }
    if print_preallocated(root) != 0 as ::core::ffi::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
    root = cJSON_CreateObject();
    cJSON_AddNumberToObject(
        root,
        b"number\0" as *const u8 as *const ::core::ffi::c_char,
        1.0f64 / zero,
    );
    if print_preallocated(root) != 0 as ::core::ffi::c_int {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
}
#[no_mangle]
pub unsafe extern "C" fn driver(
    mut strings: *mut *const ::core::ffi::c_char,
    mut numbers: *mut [::core::ffi::c_int; 3],
    mut ids: *mut ::core::ffi::c_int,
    mut fields: *mut record,
) -> ::core::ffi::c_int {
    printf(
        b"Version: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
        cJSON_Version(),
    );
    create_objects(strings, numbers, ids, fields);
    return 0 as ::core::ffi::c_int;
}
