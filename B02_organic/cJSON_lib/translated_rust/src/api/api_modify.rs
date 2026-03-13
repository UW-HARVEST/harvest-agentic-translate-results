use std::ffi::{c_char, c_int, c_void};
use std::ptr;
use crate::types::*;
use crate::globals::*;
use crate::helpers::*;
use super::api_core::{cJSON_Delete, cJSON_free};

// ---- AddItemToArray / Object ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    add_item_to_array(array, item)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool {
    add_item_to_object(object, string, item, &GLOBAL_HOOKS, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool {
    add_item_to_object(object, string, item, &GLOBAL_HOOKS, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if array.is_null() { return 0; }
    add_item_to_array(array, create_reference(item, &GLOBAL_HOOKS))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(object: *mut cJSON, string: *const c_char, item: *mut cJSON) -> cJSON_bool {
    if object.is_null() || string.is_null() { return 0; }
    add_item_to_object(object, string, create_reference(item, &GLOBAL_HOOKS), &GLOBAL_HOOKS, 0)
}

// ---- Detach ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON {
    if parent.is_null() || item.is_null() || (item != (*parent).child && (*item).prev.is_null()) {
        return ptr::null_mut();
    }
    if item != (*parent).child {
        (*(*item).prev).next = (*item).next;
    }
    if !(*item).next.is_null() {
        (*(*item).next).prev = (*item).prev;
    }
    if item == (*parent).child {
        (*parent).child = (*item).next;
    } else if (*item).next.is_null() {
        (*(*parent).child).prev = (*item).prev;
    }
    (*item).prev = ptr::null_mut();
    (*item).next = ptr::null_mut();
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    if which < 0 { return ptr::null_mut(); }
    cJSON_DetachItemViaPointer(array, get_array_item(array, which as usize))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    let to_detach = super::api_query::cJSON_GetObjectItem(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    let to_detach = super::api_query::cJSON_GetObjectItemCaseSensitive(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

// ---- Insert ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool {
    if which < 0 || newitem.is_null() { return 0; }
    let after_inserted = get_array_item(array, which as usize);
    if after_inserted.is_null() {
        return add_item_to_array(array, newitem);
    }
    if after_inserted != (*array).child && (*after_inserted).prev.is_null() {
        return 0;
    }
    (*newitem).next = after_inserted;
    (*newitem).prev = (*after_inserted).prev;
    (*after_inserted).prev = newitem;
    if after_inserted == (*array).child {
        (*array).child = newitem;
    } else {
        (*(*newitem).prev).next = newitem;
    }
    1
}

// ---- Replace ----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> cJSON_bool {
    if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
        return 0;
    }
    if replacement == item { return 1; }

    (*replacement).next = (*item).next;
    (*replacement).prev = (*item).prev;

    if !(*replacement).next.is_null() {
        (*(*replacement).next).prev = replacement;
    }
    if (*parent).child == item {
        if (*(*parent).child).prev == (*parent).child {
            (*replacement).prev = replacement;
        }
        (*parent).child = replacement;
    } else {
        if !(*replacement).prev.is_null() {
            (*(*replacement).prev).next = replacement;
        }
        if (*replacement).next.is_null() {
            (*(*parent).child).prev = replacement;
        }
    }

    (*item).next = ptr::null_mut();
    (*item).prev = ptr::null_mut();
    cJSON_Delete(item);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool {
    if which < 0 { return 0; }
    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
}

unsafe fn replace_item_in_object(object: *mut cJSON, string: *const c_char, replacement: *mut cJSON, case_sensitive: cJSON_bool) -> cJSON_bool {
    if replacement.is_null() || string.is_null() { return 0; }
    if ((*replacement).type_ & CJSON_STRING_IS_CONST) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string = cjson_strdup(string as *const u8, &GLOBAL_HOOKS) as *mut c_char;
    if (*replacement).string.is_null() { return 0; }
    (*replacement).type_ &= !CJSON_STRING_IS_CONST;
    cJSON_ReplaceItemViaPointer(object, get_object_item(object, string, case_sensitive), replacement)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 1)
}
