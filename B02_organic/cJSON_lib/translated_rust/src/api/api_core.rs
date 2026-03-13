use std::ffi::{c_char, c_double, c_int, c_void};
use std::ptr;
use crate::types::*;
use crate::globals::*;
use crate::helpers::*;
use crate::parse::*;
use crate::print::print_value;

// ---- Version ----
static mut VERSION_BUF: [u8; 15] = [0; 15];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    libc::sprintf(
        VERSION_BUF.as_mut_ptr() as *mut i8,
        b"%i.%i.%i\0".as_ptr() as *const i8,
        CJSON_VERSION_MAJOR,
        CJSON_VERSION_MINOR,
        CJSON_VERSION_PATCH,
    );
    VERSION_BUF.as_ptr() as *const c_char
}

// ---- Error ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    GLOBAL_ERROR.json.add(GLOBAL_ERROR.position) as *const c_char
}

// ---- Hooks ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        GLOBAL_HOOKS.allocate = crate::globals::malloc;
        GLOBAL_HOOKS.deallocate = crate::globals::free;
        GLOBAL_HOOKS.reallocate = Some(crate::globals::realloc);
        return;
    }
    GLOBAL_HOOKS.allocate = crate::globals::malloc;
    if let Some(malloc_fn) = (*hooks).malloc_fn {
        GLOBAL_HOOKS.allocate = malloc_fn;
    }
    GLOBAL_HOOKS.deallocate = crate::globals::free;
    if let Some(free_fn) = (*hooks).free_fn {
        GLOBAL_HOOKS.deallocate = free_fn;
    }
    GLOBAL_HOOKS.reallocate = None;
    if GLOBAL_HOOKS.allocate as usize == crate::globals::malloc as usize
        && GLOBAL_HOOKS.deallocate as usize == crate::globals::free as usize
    {
        GLOBAL_HOOKS.reallocate = Some(crate::globals::realloc);
    }
}

// ---- Delete ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    while !item.is_null() {
        let next = (*item).next;
        if ((*item).type_ & CJSON_IS_REFERENCE) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & CJSON_IS_REFERENCE) == 0 && !(*item).valuestring.is_null() {
            (GLOBAL_HOOKS.deallocate)((*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & CJSON_STRING_IS_CONST) == 0 && !(*item).string.is_null() {
            (GLOBAL_HOOKS.deallocate)((*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        (GLOBAL_HOOKS.deallocate)(item as *mut c_void);
        item = next;
    }
}

// ---- GetStringValue / GetNumberValue ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if cJSON_IsString(item) == 0 {
        return ptr::null_mut();
    }
    (*item).valuestring
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    if cJSON_IsNumber(item) == 0 {
        return f64::NAN;
    }
    (*item).valuedouble
}

// ---- SetNumberHelper ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if number >= c_int::MAX as f64 {
        (*object).valueint = c_int::MAX;
    } else if number <= c_int::MIN as f64 {
        (*object).valueint = c_int::MIN;
    } else {
        (*object).valueint = number as c_int;
    }
    (*object).valuedouble = number;
    (*object).valuedouble
}

// ---- SetValuestring ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    if object.is_null()
        || ((*object).type_ & CJSON_STRING) == 0
        || ((*object).type_ & CJSON_IS_REFERENCE) != 0
    {
        return ptr::null_mut();
    }
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    let v1_len = libc::strlen(valuestring);
    let v2_len = libc::strlen((*object).valuestring);
    if v1_len <= v2_len {
        if !((valuestring as usize + v1_len) < (*object).valuestring as usize
            || ((*object).valuestring as usize + v2_len) < valuestring as usize)
        {
            return ptr::null_mut();
        }
        libc::strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    let copy = cjson_strdup(valuestring as *const u8, &GLOBAL_HOOKS) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;
    copy
}

// ---- Type checkers ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_INVALID) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_FALSE) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xff) == CJSON_TRUE) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & (CJSON_TRUE | CJSON_FALSE)) != 0) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_NULL) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_NUMBER) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_STRING) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_ARRAY) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_OBJECT) as cJSON_bool
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    (((*item).type_ & 0xFF) == CJSON_RAW) as cJSON_bool
}

// ---- malloc / free ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    (GLOBAL_HOOKS.allocate)(size)
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    (GLOBAL_HOOKS.deallocate)(object);
}

// ---- Parse API ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLength(value: *const c_char, buffer_length: usize) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    if value.is_null() {
        return ptr::null_mut();
    }
    let buffer_length = libc::strlen(value) + 1;
    cJSON_ParseWithLengthOpts(value, buffer_length, return_parse_end, require_null_terminated)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    let mut buffer = ParseBuffer {
        content: ptr::null(),
        length: 0,
        offset: 0,
        depth: 0,
        hooks: GLOBAL_HOOKS,
    };

    GLOBAL_ERROR.json = ptr::null();
    GLOBAL_ERROR.position = 0;

    if value.is_null() || buffer_length == 0 {
        // goto fail
        if !value.is_null() {
            let mut local_error = ErrorInfo { json: value as *const u8, position: 0 };
            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }
            if !return_parse_end.is_null() {
                *return_parse_end = (local_error.json.add(local_error.position)) as *const c_char;
            }
            GLOBAL_ERROR = local_error;
        }
        return ptr::null_mut();
    }

    buffer.content = value as *const u8;
    buffer.length = buffer_length;
    buffer.offset = 0;
    buffer.hooks = GLOBAL_HOOKS;

    let item = cjson_new_item(&GLOBAL_HOOKS);
    if item.is_null() {
        // goto fail
        if !value.is_null() {
            let mut local_error = ErrorInfo { json: value as *const u8, position: 0 };
            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }
            if !return_parse_end.is_null() {
                *return_parse_end = (local_error.json.add(local_error.position)) as *const c_char;
            }
            GLOBAL_ERROR = local_error;
        }
        return ptr::null_mut();
    }

    let bom_result = skip_utf8_bom(&mut buffer);
    let ws_result = buffer_skip_whitespace(bom_result);
    if parse_value(item, ws_result) == 0 {
        cJSON_Delete(item);
        if !value.is_null() {
            let mut local_error = ErrorInfo { json: value as *const u8, position: 0 };
            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }
            if !return_parse_end.is_null() {
                *return_parse_end = (local_error.json.add(local_error.position)) as *const c_char;
            }
            GLOBAL_ERROR = local_error;
        }
        return ptr::null_mut();
    }

    if require_null_terminated != 0 {
        buffer_skip_whitespace(&mut buffer);
        if buffer.offset >= buffer.length || *buffer.content.add(buffer.offset) != 0 {
            cJSON_Delete(item);
            if !value.is_null() {
                let mut local_error = ErrorInfo { json: value as *const u8, position: 0 };
                if buffer.offset < buffer.length {
                    local_error.position = buffer.offset;
                } else if buffer.length > 0 {
                    local_error.position = buffer.length - 1;
                }
                if !return_parse_end.is_null() {
                    *return_parse_end = (local_error.json.add(local_error.position)) as *const c_char;
                }
                GLOBAL_ERROR = local_error;
            }
            return ptr::null_mut();
        }
    }
    if !return_parse_end.is_null() {
        *return_parse_end = buffer.content.add(buffer.offset) as *const c_char;
    }
    item
}

// ---- Print API ----
unsafe fn print_internal(item: *const cJSON, format: cJSON_bool, hooks: &InternalHooks) -> *mut c_char {
    let default_buffer_size: usize = 256;
    let mut buffer = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format,
        hooks: *hooks,
    };

    buffer.buffer = (hooks.allocate)(default_buffer_size) as *mut u8;
    buffer.length = default_buffer_size;
    if buffer.buffer.is_null() {
        return ptr::null_mut();
    }

    if print_value(item, &mut buffer) == 0 {
        if !buffer.buffer.is_null() {
            (hooks.deallocate)(buffer.buffer as *mut c_void);
        }
        return ptr::null_mut();
    }
    update_offset(&mut buffer);

    let printed: *mut u8;
    if let Some(reallocate) = hooks.reallocate {
        printed = reallocate(buffer.buffer as *mut c_void, buffer.offset + 1) as *mut u8;
        if printed.is_null() {
            (hooks.deallocate)(buffer.buffer as *mut c_void);
            return ptr::null_mut();
        }
        buffer.buffer = ptr::null_mut();
    } else {
        printed = (hooks.allocate)(buffer.offset + 1) as *mut u8;
        if printed.is_null() {
            (hooks.deallocate)(buffer.buffer as *mut c_void);
            return ptr::null_mut();
        }
        let copy_len = if buffer.length < buffer.offset + 1 { buffer.length } else { buffer.offset + 1 };
        ptr::copy_nonoverlapping(buffer.buffer, printed, copy_len);
        *printed.add(buffer.offset) = 0;
        (hooks.deallocate)(buffer.buffer as *mut c_void);
        buffer.buffer = ptr::null_mut();
    }

    printed as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print_internal(item, 1, &GLOBAL_HOOKS)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print_internal(item, 0, &GLOBAL_HOOKS)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(item: *const cJSON, prebuffer: c_int, fmt: cJSON_bool) -> *mut c_char {
    if prebuffer < 0 {
        return ptr::null_mut();
    }
    let mut p = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: fmt,
        hooks: GLOBAL_HOOKS,
    };
    p.buffer = (GLOBAL_HOOKS.allocate)(prebuffer as usize) as *mut u8;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }
    p.length = prebuffer as usize;
    if print_value(item, &mut p) == 0 {
        (GLOBAL_HOOKS.deallocate)(p.buffer as *mut c_void);
        return ptr::null_mut();
    }
    p.buffer as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintPreallocated(
    item: *mut cJSON,
    buffer: *mut c_char,
    length: c_int,
    format: cJSON_bool,
) -> cJSON_bool {
    if length < 0 || buffer.is_null() {
        return 0;
    }
    let mut p = PrintBuffer {
        buffer: buffer as *mut u8,
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: GLOBAL_HOOKS,
    };
    print_value(item, &mut p)
}
