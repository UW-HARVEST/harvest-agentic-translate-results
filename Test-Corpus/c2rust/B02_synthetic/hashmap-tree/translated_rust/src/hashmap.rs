extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint64_t = u64;
pub type uint8_t = __uint8_t;
pub type uint64_t = __uint64_t;
pub type tree_id_t = uint64_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct hashmap_entry {
    pub key: tree_id_t,
    pub value: *mut ::core::ffi::c_void,
    pub occupied: ::core::ffi::c_int,
    pub deleted: ::core::ffi::c_int,
}
pub type hashmap_entry_t = hashmap_entry;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct hashmap_t {
    pub entries: *mut hashmap_entry_t,
    pub capacity: size_t,
    pub size: size_t,
    pub deleted_count: size_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const HASHMAP_INITIAL_CAPACITY: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const HASHMAP_LOAD_FACTOR: ::core::ffi::c_double = 0.75f64;
unsafe extern "C" fn hash_function(mut key: tree_id_t) -> uint64_t {
    let mut hash: uint64_t = 14695981039346656037 as uint64_t;
    let mut bytes: *mut uint8_t = &raw mut key as *mut uint8_t;
    let mut i: size_t = 0 as size_t;
    while i < ::core::mem::size_of::<tree_id_t>() as usize {
        hash = (hash as ::core::ffi::c_ulong ^ *bytes.offset(i as isize) as ::core::ffi::c_ulong)
            as uint64_t;
        hash = (hash as ::core::ffi::c_ulonglong)
            .wrapping_mul(1099511628211 as ::core::ffi::c_ulonglong) as uint64_t
            as uint64_t;
        i = i.wrapping_add(1);
    }
    return hash;
}
unsafe extern "C" fn should_resize(mut map: *mut hashmap_t) -> ::core::ffi::c_int {
    let mut load: ::core::ffi::c_double = (*map).size.wrapping_add((*map).deleted_count)
        as ::core::ffi::c_double
        / (*map).capacity as ::core::ffi::c_double;
    return (load > HASHMAP_LOAD_FACTOR) as ::core::ffi::c_int;
}
unsafe extern "C" fn hashmap_resize(mut map: *mut hashmap_t) -> ::core::ffi::c_int {
    let mut old_capacity: size_t = (*map).capacity;
    let mut old_entries: *mut hashmap_entry_t = (*map).entries;
    (*map).capacity = ((*map).capacity as ::core::ffi::c_ulong)
        .wrapping_mul(2 as ::core::ffi::c_ulong) as size_t as size_t;
    (*map).entries = calloc(
        (*map).capacity,
        ::core::mem::size_of::<hashmap_entry_t>() as size_t,
    ) as *mut hashmap_entry_t;
    if (*map).entries.is_null() {
        (*map).entries = old_entries;
        (*map).capacity = old_capacity;
        return -(1 as ::core::ffi::c_int);
    }
    (*map).size = 0 as size_t;
    (*map).deleted_count = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < old_capacity {
        if (*old_entries.offset(i as isize)).occupied != 0
            && (*old_entries.offset(i as isize)).deleted == 0
        {
            hashmap_put(
                map,
                (*old_entries.offset(i as isize)).key,
                (*old_entries.offset(i as isize)).value,
            );
        }
        i = i.wrapping_add(1);
    }
    free(old_entries as *mut ::core::ffi::c_void);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_create() -> *mut hashmap_t {
    let mut map: *mut hashmap_t =
        malloc(::core::mem::size_of::<hashmap_t>() as size_t) as *mut hashmap_t;
    if map.is_null() {
        return ::core::ptr::null_mut::<hashmap_t>();
    }
    (*map).capacity = HASHMAP_INITIAL_CAPACITY as size_t;
    (*map).size = 0 as size_t;
    (*map).deleted_count = 0 as size_t;
    (*map).entries = calloc(
        (*map).capacity,
        ::core::mem::size_of::<hashmap_entry_t>() as size_t,
    ) as *mut hashmap_entry_t;
    if (*map).entries.is_null() {
        free(map as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<hashmap_t>();
    }
    return map;
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_destroy(mut map: *mut hashmap_t) {
    if !map.is_null() {
        free((*map).entries as *mut ::core::ffi::c_void);
        free(map as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_put(
    mut map: *mut hashmap_t,
    mut key: tree_id_t,
    mut value: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if map.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    if should_resize(map) != 0 {
        if hashmap_resize(map) != 0 as ::core::ffi::c_int {
            return -(1 as ::core::ffi::c_int);
        }
    }
    let mut hash: uint64_t = hash_function(key);
    let mut index: size_t = (hash as size_t).wrapping_rem((*map).capacity);
    let mut probe: size_t = 0 as size_t;
    while probe < (*map).capacity {
        let mut current: size_t = index.wrapping_add(probe).wrapping_rem((*map).capacity);
        if (*(*map).entries.offset(current as isize)).occupied == 0 {
            (*(*map).entries.offset(current as isize)).key = key;
            let ref mut fresh0 = (*(*map).entries.offset(current as isize)).value;
            *fresh0 = value;
            (*(*map).entries.offset(current as isize)).occupied = 1 as ::core::ffi::c_int;
            (*(*map).entries.offset(current as isize)).deleted = 0 as ::core::ffi::c_int;
            (*map).size = (*map).size.wrapping_add(1);
            return 0 as ::core::ffi::c_int;
        } else if (*(*map).entries.offset(current as isize)).deleted != 0 {
            (*(*map).entries.offset(current as isize)).key = key;
            let ref mut fresh1 = (*(*map).entries.offset(current as isize)).value;
            *fresh1 = value;
            (*(*map).entries.offset(current as isize)).deleted = 0 as ::core::ffi::c_int;
            (*map).size = (*map).size.wrapping_add(1);
            (*map).deleted_count = (*map).deleted_count.wrapping_sub(1);
            return 0 as ::core::ffi::c_int;
        } else if (*(*map).entries.offset(current as isize)).key == key {
            let ref mut fresh2 = (*(*map).entries.offset(current as isize)).value;
            *fresh2 = value;
            return 0 as ::core::ffi::c_int;
        }
        probe = probe.wrapping_add(1);
    }
    return -(1 as ::core::ffi::c_int);
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_get(
    mut map: *mut hashmap_t,
    mut key: tree_id_t,
) -> *mut ::core::ffi::c_void {
    if map.is_null() {
        return NULL;
    }
    let mut hash: uint64_t = hash_function(key);
    let mut index: size_t = (hash as size_t).wrapping_rem((*map).capacity);
    let mut probe: size_t = 0 as size_t;
    while probe < (*map).capacity {
        let mut current: size_t = index.wrapping_add(probe).wrapping_rem((*map).capacity);
        if (*(*map).entries.offset(current as isize)).occupied == 0 {
            return NULL;
        }
        if (*(*map).entries.offset(current as isize)).deleted == 0
            && (*(*map).entries.offset(current as isize)).key == key
        {
            return (*(*map).entries.offset(current as isize)).value;
        }
        probe = probe.wrapping_add(1);
    }
    return NULL;
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_remove(
    mut map: *mut hashmap_t,
    mut key: tree_id_t,
) -> *mut ::core::ffi::c_void {
    if map.is_null() {
        return NULL;
    }
    let mut hash: uint64_t = hash_function(key);
    let mut index: size_t = (hash as size_t).wrapping_rem((*map).capacity);
    let mut probe: size_t = 0 as size_t;
    while probe < (*map).capacity {
        let mut current: size_t = index.wrapping_add(probe).wrapping_rem((*map).capacity);
        if (*(*map).entries.offset(current as isize)).occupied == 0 {
            return NULL;
        }
        if (*(*map).entries.offset(current as isize)).deleted == 0
            && (*(*map).entries.offset(current as isize)).key == key
        {
            let mut value: *mut ::core::ffi::c_void =
                (*(*map).entries.offset(current as isize)).value;
            (*(*map).entries.offset(current as isize)).deleted = 1 as ::core::ffi::c_int;
            (*map).size = (*map).size.wrapping_sub(1);
            (*map).deleted_count = (*map).deleted_count.wrapping_add(1);
            return value;
        }
        probe = probe.wrapping_add(1);
    }
    return NULL;
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_contains(
    mut map: *mut hashmap_t,
    mut key: tree_id_t,
) -> ::core::ffi::c_int {
    return (hashmap_get(map, key) != NULL) as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_size(mut map: *mut hashmap_t) -> size_t {
    return if !map.is_null() {
        (*map).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub unsafe extern "C" fn hashmap_clear(mut map: *mut hashmap_t) {
    if map.is_null() {
        return;
    }
    let mut i: size_t = 0 as size_t;
    while i < (*map).capacity {
        (*(*map).entries.offset(i as isize)).occupied = 0 as ::core::ffi::c_int;
        (*(*map).entries.offset(i as isize)).deleted = 0 as ::core::ffi::c_int;
        i = i.wrapping_add(1);
    }
    (*map).size = 0 as size_t;
    (*map).deleted_count = 0 as size_t;
}
