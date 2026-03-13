use std::ffi::c_int;
use std::ptr;
use crate::types::*;
use crate::globals::*;

pub(crate) unsafe fn cjson_strdup(string: *const u8, hooks: &InternalHooks) -> *mut u8 {
    if string.is_null() {
        return ptr::null_mut();
    }
    let length = libc::strlen(string as *const i8) + 1;
    let copy = (hooks.allocate)(length) as *mut u8;
    if copy.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(string, copy, length);
    copy
}

pub(crate) unsafe fn cjson_new_item(hooks: &InternalHooks) -> *mut cJSON {
    let node = (hooks.allocate)(std::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        ptr::write_bytes(node, 0, 1);
    }
    node
}

pub(crate) unsafe fn case_insensitive_strcmp(s1: *const u8, s2: *const u8) -> c_int {
    if s1.is_null() || s2.is_null() {
        return 1;
    }
    if s1 == s2 {
        return 0;
    }
    let mut p1 = s1;
    let mut p2 = s2;
    while libc::tolower(*p1 as c_int) == libc::tolower(*p2 as c_int) {
        if *p1 == 0 {
            return 0;
        }
        p1 = p1.add(1);
        p2 = p2.add(1);
    }
    libc::tolower(*p1 as c_int) - libc::tolower(*p2 as c_int)
}

// suffix_object: link prev -> item
pub(crate) unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {
    (*prev).next = item;
    (*item).prev = prev;
}

// create_reference
pub(crate) unsafe fn create_reference(item: *const cJSON, hooks: &InternalHooks) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }
    let reference = cjson_new_item(hooks);
    if reference.is_null() {
        return ptr::null_mut();
    }
    ptr::copy_nonoverlapping(item as *const u8, reference as *mut u8, std::mem::size_of::<cJSON>());
    (*reference).string = ptr::null_mut();
    (*reference).type_ |= CJSON_IS_REFERENCE;
    (*reference).next = ptr::null_mut();
    (*reference).prev = ptr::null_mut();
    reference
}

// compare_double
pub(crate) fn compare_double(a: f64, b: f64) -> cJSON_bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    if (a - b).abs() <= max_val * f64::EPSILON {
        1
    } else {
        0
    }
}

// ensure: realloc printbuffer if necessary
pub(crate) unsafe fn ensure(p: *mut PrintBuffer, needed: usize) -> *mut u8 {
    if p.is_null() || (*p).buffer.is_null() {
        return ptr::null_mut();
    }
    if (*p).length > 0 && (*p).offset >= (*p).length {
        return ptr::null_mut();
    }
    if needed > c_int::MAX as usize {
        return ptr::null_mut();
    }
    let needed_total = needed + (*p).offset + 1;
    if needed_total <= (*p).length {
        return (*p).buffer.add((*p).offset);
    }
    if (*p).noalloc != 0 {
        return ptr::null_mut();
    }
    let newsize: usize;
    if needed_total > (c_int::MAX as usize / 2) {
        if needed_total <= c_int::MAX as usize {
            newsize = c_int::MAX as usize;
        } else {
            return ptr::null_mut();
        }
    } else {
        newsize = needed_total * 2;
    }

    if let Some(reallocate) = (*p).hooks.reallocate {
        let newbuffer = reallocate((*p).buffer as *mut _, newsize) as *mut u8;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate)((*p).buffer as *mut _);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        (*p).length = newsize;
        (*p).buffer = newbuffer;
    } else {
        let newbuffer = ((*p).hooks.allocate)(newsize) as *mut u8;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate)((*p).buffer as *mut _);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        ptr::copy_nonoverlapping((*p).buffer, newbuffer, (*p).offset + 1);
        ((*p).hooks.deallocate)((*p).buffer as *mut _);
        (*p).length = newsize;
        (*p).buffer = newbuffer;
    }

    (*p).buffer.add((*p).offset)
}

pub(crate) unsafe fn update_offset(buffer: *mut PrintBuffer) {
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return;
    }
    let ptr = (*buffer).buffer.add((*buffer).offset);
    (*buffer).offset += libc::strlen(ptr as *const i8);
}

// ParseBuffer helpers
pub(crate) unsafe fn can_read(buffer: *const ParseBuffer, size: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + size) <= (*buffer).length
}

pub(crate) unsafe fn can_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

pub(crate) unsafe fn buffer_at_offset(buffer: *const ParseBuffer) -> *const u8 {
    (*buffer).content.add((*buffer).offset)
}

pub(crate) unsafe fn buffer_skip_whitespace(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() {
        return ptr::null_mut();
    }
    if !can_access_at_index(buffer, 0) {
        return buffer;
    }
    while can_access_at_index(buffer, 0) && *buffer_at_offset(buffer) <= 32 {
        (*buffer).offset += 1;
    }
    if (*buffer).offset == (*buffer).length {
        (*buffer).offset -= 1;
    }
    buffer
}

pub(crate) unsafe fn skip_utf8_bom(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() || (*buffer).offset != 0 {
        return ptr::null_mut();
    }
    if can_access_at_index(buffer, 4)
        && libc::strncmp(
            buffer_at_offset(buffer) as *const i8,
            b"\xEF\xBB\xBF\0".as_ptr() as *const i8,
            3,
        ) == 0
    {
        (*buffer).offset += 3;
    }
    buffer
}

// add_item_to_array
pub(crate) unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if item.is_null() || array.is_null() || array == item {
        return 0;
    }
    let child = (*array).child;
    if child.is_null() {
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else {
        if !(*child).prev.is_null() {
            suffix_object((*child).prev, item);
            (*(*array).child).prev = item;
        }
    }
    1
}

// add_item_to_object
pub(crate) unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const i8,
    item: *mut cJSON,
    hooks: &InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return 0;
    }
    let new_key: *mut i8;
    let new_type: c_int;
    if constant_key != 0 {
        new_key = string as *mut i8;
        new_type = (*item).type_ | CJSON_STRING_IS_CONST;
    } else {
        new_key = cjson_strdup(string as *const u8, hooks) as *mut i8;
        if new_key.is_null() {
            return 0;
        }
        new_type = (*item).type_ & !CJSON_STRING_IS_CONST;
    }
    if ((*item).type_ & CJSON_STRING_IS_CONST) == 0 && !(*item).string.is_null() {
        (hooks.deallocate)((*item).string as *mut _);
    }
    (*item).string = new_key;
    (*item).type_ = new_type;
    add_item_to_array(object, item)
}

// get_object_item
pub(crate) unsafe fn get_object_item(
    object: *const cJSON,
    name: *const i8,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if object.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let mut current = (*object).child;
    if case_sensitive != 0 {
        while !current.is_null()
            && !(*current).string.is_null()
            && libc::strcmp(name, (*current).string) != 0
        {
            current = (*current).next;
        }
    } else {
        while !current.is_null()
            && case_insensitive_strcmp(name as *const u8, (*current).string as *const u8) != 0
        {
            current = (*current).next;
        }
    }
    if current.is_null() || (*current).string.is_null() {
        return ptr::null_mut();
    }
    current
}

// get_array_item
pub(crate) unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
    if array.is_null() {
        return ptr::null_mut();
    }
    let mut current = (*array).child;
    while !current.is_null() && index > 0 {
        index -= 1;
        current = (*current).next;
    }
    current
}
