use crate::internal::*;
use std::ffi::{c_char, c_double, c_float, c_int, c_void};
use std::ptr;

static mut VERSION: [c_char; 15] = [0; 15];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    GLOBAL_ERROR.json.wrapping_add(GLOBAL_ERROR.position).cast()
}

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
        return -c_double::NAN;
    }
    (*item).valuedouble
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    let version = (&raw mut VERSION).cast::<c_char>();
    sprintf(
        version,
        c"%i.%i.%i".as_ptr(),
        1 as c_int,
        7 as c_int,
        19 as c_int,
    );
    version
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        GLOBAL_HOOKS = InternalHooks {
            allocate: Some(malloc),
            deallocate: Some(free),
            reallocate: Some(realloc),
        };
        return;
    }

    GLOBAL_HOOKS.allocate = (*hooks).malloc_fn.or(Some(malloc));
    GLOBAL_HOOKS.deallocate = (*hooks).free_fn.or(Some(free));
    GLOBAL_HOOKS.reallocate = None;

    let default_malloc = GLOBAL_HOOKS.allocate.map(|function| function as usize)
        == Some(malloc as AllocateFn as usize);
    let default_free = GLOBAL_HOOKS.deallocate.map(|function| function as usize)
        == Some(free as DeallocateFn as usize);
    if default_malloc && default_free {
        GLOBAL_HOOKS.reallocate = Some(realloc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    while !item.is_null() {
        let next = (*item).next;
        if ((*item).type_ & CJSON_IS_REFERENCE) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & CJSON_IS_REFERENCE) == 0 && !(*item).valuestring.is_null() {
            deallocate(&GLOBAL_HOOKS, (*item).valuestring.cast());
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & CJSON_STRING_IS_CONST) == 0 && !(*item).string.is_null() {
            deallocate(&GLOBAL_HOOKS, (*item).string.cast());
            (*item).string = ptr::null_mut();
        }
        deallocate(&GLOBAL_HOOKS, item.cast());
        item = next;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    (*object).valueint = clamp_int(number);
    (*object).valuedouble = number;
    number
}

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

    let v1_len = strlen(valuestring);
    let v2_len = strlen((*object).valuestring);
    if v1_len <= v2_len {
        let source_end = valuestring.add(v1_len) as usize;
        let destination = (*object).valuestring as usize;
        let destination_end = (*object).valuestring.add(v2_len) as usize;
        let source = valuestring as usize;
        if !(source_end < destination || destination_end < source) {
            return ptr::null_mut();
        }
        strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }

    let copy = duplicate_string(valuestring.cast(), &GLOBAL_HOOKS).cast::<c_char>();
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring.cast());
    }
    (*object).valuestring = copy;
    copy
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() {
        return 0;
    }
    let mut child = (*array).child;
    let mut size = 0usize;
    while !child.is_null() {
        size = size.wrapping_add(1);
        child = (*child).next;
    }
    size as c_int
}

unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 {
        return ptr::null_mut();
    }
    get_array_item(array, index as usize)
}

unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if object.is_null() || name.is_null() {
        return ptr::null_mut();
    }
    let mut current = (*object).child;
    if case_sensitive != 0 {
        while !current.is_null()
            && !(*current).string.is_null()
            && strcmp(name, (*current).string) != 0
        {
            current = (*current).next;
        }
    } else {
        while !current.is_null()
            && case_insensitive_strcmp(name.cast(), (*current).string.cast()) != 0
        {
            current = (*current).next;
        }
    }
    if current.is_null() || (*current).string.is_null() {
        ptr::null_mut()
    } else {
        current
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    (!cJSON_GetObjectItem(object, string).is_null()) as cJSON_bool
}

unsafe fn suffix_object(previous: *mut cJSON, item: *mut cJSON) {
    (*previous).next = item;
    (*item).prev = previous;
}

unsafe fn create_reference(item: *const cJSON, hooks: &InternalHooks) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }
    let reference = new_item(hooks);
    if reference.is_null() {
        return ptr::null_mut();
    }
    memcpy(reference.cast(), item.cast(), size_of::<cJSON>());
    (*reference).string = ptr::null_mut();
    (*reference).type_ |= CJSON_IS_REFERENCE;
    (*reference).next = ptr::null_mut();
    (*reference).prev = ptr::null_mut();
    reference
}

unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if item.is_null() || array.is_null() || array == item {
        return 0;
    }
    let child = (*array).child;
    if child.is_null() {
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else if !(*child).prev.is_null() {
        suffix_object((*child).prev, item);
        (*(*array).child).prev = item;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    add_item_to_array(array, item)
}

unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
    hooks: &InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return 0;
    }

    let (new_key, new_type) = if constant_key != 0 {
        (string.cast_mut(), (*item).type_ | CJSON_STRING_IS_CONST)
    } else {
        let key = duplicate_string(string.cast(), hooks).cast::<c_char>();
        if key.is_null() {
            return 0;
        }
        (key, (*item).type_ & !CJSON_STRING_IS_CONST)
    };

    if ((*item).type_ & CJSON_STRING_IS_CONST) == 0 && !(*item).string.is_null() {
        deallocate(hooks, (*item).string.cast());
    }
    (*item).string = new_key;
    (*item).type_ = new_type;
    add_item_to_array(object, item)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &GLOBAL_HOOKS, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &GLOBAL_HOOKS, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() {
        return 0;
    }
    add_item_to_array(array, create_reference(item, &GLOBAL_HOOKS))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    if object.is_null() || string.is_null() {
        return 0;
    }
    add_item_to_object(
        object,
        string,
        create_reference(item, &GLOBAL_HOOKS),
        &GLOBAL_HOOKS,
        0,
    )
}

unsafe fn add_created_to_object(
    object: *mut cJSON,
    name: *const c_char,
    item: *mut cJSON,
) -> *mut cJSON {
    if add_item_to_object(object, name, item, &GLOBAL_HOOKS, 0) != 0 {
        item
    } else {
        cJSON_Delete(item);
        ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateNull())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateTrue())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateFalse())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateBool(boolean))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateNumber(number))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateString(string))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateRaw(raw))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateObject())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    add_created_to_object(object, name, cJSON_CreateArray())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
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
    if which < 0 {
        return ptr::null_mut();
    }
    cJSON_DetachItemViaPointer(array, get_array_item(array, which as usize))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    cJSON_DetachItemViaPointer(object, cJSON_GetObjectItem(object, string))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    cJSON_DetachItemViaPointer(object, cJSON_GetObjectItemCaseSensitive(object, string))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 || newitem.is_null() {
        return 0;
    }
    let after = get_array_item(array, which as usize);
    if after.is_null() {
        return add_item_to_array(array, newitem);
    }
    if after != (*array).child && (*after).prev.is_null() {
        return 0;
    }
    (*newitem).next = after;
    (*newitem).prev = (*after).prev;
    (*after).prev = newitem;
    if after == (*array).child {
        (*array).child = newitem;
    } else {
        (*(*newitem).prev).next = newitem;
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
    if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
        return 0;
    }
    if replacement == item {
        return 1;
    }
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
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 {
        return 0;
    }
    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if replacement.is_null() || string.is_null() {
        return 0;
    }
    if ((*replacement).type_ & CJSON_STRING_IS_CONST) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string.cast());
    }
    (*replacement).string = duplicate_string(string.cast(), &GLOBAL_HOOKS).cast();
    if (*replacement).string.is_null() {
        return 0;
    }
    (*replacement).type_ &= !CJSON_STRING_IS_CONST;
    cJSON_ReplaceItemViaPointer(
        object,
        get_object_item(object, string, case_sensitive),
        replacement,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 1)
}

unsafe fn create_type(type_: c_int) -> *mut cJSON {
    let item = new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = type_;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    create_type(CJSON_NULL)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    create_type(CJSON_TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    create_type(CJSON_FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    create_type(if boolean != 0 {
        CJSON_TRUE
    } else {
        CJSON_FALSE
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(number: c_double) -> *mut cJSON {
    let item = new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_NUMBER;
        (*item).valuedouble = number;
        (*item).valueint = clamp_int(number);
    }
    item
}

unsafe fn create_owned_string(string: *const c_char, type_: c_int) -> *mut cJSON {
    let item = new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = type_;
        (*item).valuestring = duplicate_string(string.cast(), &GLOBAL_HOOKS).cast();
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    create_owned_string(string, CJSON_STRING)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_STRING | CJSON_IS_REFERENCE;
        (*item).valuestring = string.cast_mut();
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_OBJECT | CJSON_IS_REFERENCE;
        (*item).child = child.cast_mut();
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = new_item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = CJSON_ARRAY | CJSON_IS_REFERENCE;
        (*item).child = child.cast_mut();
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    create_owned_string(raw, CJSON_RAW)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    create_type(CJSON_ARRAY)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    create_type(CJSON_OBJECT)
}

unsafe fn finish_number_array(
    array: *mut cJSON,
    mut make_number: impl FnMut(usize) -> c_double,
    count: c_int,
) -> *mut cJSON {
    let mut previous = ptr::null_mut();
    let mut last = ptr::null_mut();
    for index in 0..count as usize {
        let number = cJSON_CreateNumber(make_number(index));
        if number.is_null() {
            cJSON_Delete(array);
            return ptr::null_mut();
        }
        if index == 0 {
            (*array).child = number;
        } else {
            suffix_object(previous, number);
        }
        previous = number;
        last = number;
    }
    if !array.is_null() && !(*array).child.is_null() {
        (*(*array).child).prev = last;
    }
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let array = cJSON_CreateArray();
    if array.is_null() {
        return array;
    }
    finish_number_array(array, |index| *numbers.add(index) as c_double, count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(
    numbers: *const c_float,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let array = cJSON_CreateArray();
    if array.is_null() {
        return array;
    }
    finish_number_array(array, |index| *numbers.add(index) as c_double, count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let array = cJSON_CreateArray();
    if array.is_null() {
        return array;
    }
    finish_number_array(array, |index| *numbers.add(index), count)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || strings.is_null() {
        return ptr::null_mut();
    }
    let array = cJSON_CreateArray();
    let mut previous = ptr::null_mut();
    let mut last = ptr::null_mut();
    for index in 0..count as usize {
        if array.is_null() {
            break;
        }
        let string = cJSON_CreateString(*strings.add(index));
        if string.is_null() {
            cJSON_Delete(array);
            return ptr::null_mut();
        }
        if index == 0 {
            (*array).child = string;
        } else {
            suffix_object(previous, string);
        }
        previous = string;
        last = string;
    }
    if !array.is_null() && !(*array).child.is_null() {
        (*(*array).child).prev = last;
    }
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: usize,
    recurse: cJSON_bool,
) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }
    let newitem = new_item(&GLOBAL_HOOKS);
    if newitem.is_null() {
        return ptr::null_mut();
    }
    (*newitem).type_ = (*item).type_ & !CJSON_IS_REFERENCE;
    (*newitem).valueint = (*item).valueint;
    (*newitem).valuedouble = (*item).valuedouble;

    if !(*item).valuestring.is_null() {
        (*newitem).valuestring = duplicate_string((*item).valuestring.cast(), &GLOBAL_HOOKS).cast();
        if (*newitem).valuestring.is_null() {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
    }
    if !(*item).string.is_null() {
        (*newitem).string = if ((*item).type_ & CJSON_STRING_IS_CONST) != 0 {
            (*item).string
        } else {
            duplicate_string((*item).string.cast(), &GLOBAL_HOOKS).cast()
        };
        if (*newitem).string.is_null() {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
    }
    if recurse == 0 {
        return newitem;
    }

    let mut child = (*item).child;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();
    while !child.is_null() {
        if depth >= CJSON_CIRCULAR_LIMIT {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
        newchild = cJSON_Duplicate_rec(child, depth + 1, 1);
        if newchild.is_null() {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
        if !next.is_null() {
            (*next).next = newchild;
            (*newchild).prev = next;
            next = newchild;
        } else {
            (*newitem).child = newchild;
            next = newchild;
        }
        child = (*child).next;
    }
    if !(*newitem).child.is_null() {
        (*(*newitem).child).prev = newchild;
    }
    newitem
}

unsafe fn skip_oneline_comment(input: &mut *mut c_char) {
    *input = (*input).add(2);
    while **input != 0 {
        if **input == b'\n' as c_char {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: &mut *mut c_char) {
    *input = (*input).add(2);
    while **input != 0 {
        if **input == b'*' as c_char && *(*input).add(1) == b'/' as c_char {
            *input = (*input).add(2);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn minify_string(input: &mut *mut c_char, output: &mut *mut c_char) {
    **output = **input;
    *input = (*input).add(1);
    *output = (*output).add(1);
    while **input != 0 {
        **output = **input;
        if **input == b'"' as c_char {
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        }
        if **input == b'\\' as c_char && *(*input).add(1) == b'"' as c_char {
            *(*output).add(1) = *(*input).add(1);
            *input = (*input).add(1);
            *output = (*output).add(1);
        }
        *input = (*input).add(1);
        *output = (*output).add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Minify(mut json: *mut c_char) {
    let mut into = json;
    if json.is_null() {
        return;
    }
    while *json != 0 {
        match *json as u8 {
            b' ' | b'\t' | b'\r' | b'\n' => json = json.add(1),
            b'/' => {
                if *json.add(1) == b'/' as c_char {
                    skip_oneline_comment(&mut json);
                } else if *json.add(1) == b'*' as c_char {
                    skip_multiline_comment(&mut json);
                } else {
                    json = json.add(1);
                }
            }
            b'"' => minify_string(&mut json, &mut into),
            _ => {
                *into = *json;
                json = json.add(1);
                into = into.add(1);
            }
        }
    }
    *into = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_INVALID)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        0
    } else {
        (((*item).type_ & (CJSON_TRUE | CJSON_FALSE)) != 0) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_NULL)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_NUMBER)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_STRING)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_ARRAY)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_OBJECT)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    is_type(item, CJSON_RAW)
}

fn compare_double(left: c_double, right: c_double) -> bool {
    let maximum = left.abs().max(right.abs());
    (left - right).abs() <= maximum * c_double::EPSILON
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null() || b.is_null() || ((*a).type_ & 0xff) != ((*b).type_ & 0xff) {
        return 0;
    }
    match (*a).type_ & 0xff {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL | CJSON_NUMBER | CJSON_STRING | CJSON_RAW
        | CJSON_ARRAY | CJSON_OBJECT => {}
        _ => return 0,
    }
    if a == b {
        return 1;
    }

    match (*a).type_ & 0xff {
        CJSON_FALSE | CJSON_TRUE | CJSON_NULL => 1,
        CJSON_NUMBER => compare_double((*a).valuedouble, (*b).valuedouble) as cJSON_bool,
        CJSON_STRING | CJSON_RAW => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() {
                0
            } else {
                (strcmp((*a).valuestring, (*b).valuestring) == 0) as cJSON_bool
            }
        }
        CJSON_ARRAY => {
            let mut a_element = (*a).child;
            let mut b_element = (*b).child;
            while !a_element.is_null() && !b_element.is_null() {
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                    return 0;
                }
                a_element = (*a_element).next;
                b_element = (*b_element).next;
            }
            (a_element == b_element) as cJSON_bool
        }
        CJSON_OBJECT => {
            let mut a_element = (*a).child;
            while !a_element.is_null() {
                let b_element = get_object_item(b, (*a_element).string, case_sensitive);
                if b_element.is_null() || cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                    return 0;
                }
                a_element = (*a_element).next;
            }
            let mut b_element = (*b).child;
            while !b_element.is_null() {
                a_element = get_object_item(a, (*b_element).string, case_sensitive);
                if a_element.is_null() || cJSON_Compare(b_element, a_element, case_sensitive) == 0 {
                    return 0;
                }
                b_element = (*b_element).next;
            }
            1
        }
        _ => 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    allocate(&GLOBAL_HOOKS, size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    deallocate(&GLOBAL_HOOKS, object);
}
