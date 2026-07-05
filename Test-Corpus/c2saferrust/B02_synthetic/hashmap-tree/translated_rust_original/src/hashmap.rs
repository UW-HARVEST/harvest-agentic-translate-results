




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
fn hash_function(key: tree_id_t) -> u64 {
    let mut hash: u64 = 14695981039346656037;
    for byte in key.to_ne_bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1099511628211);
    }
    hash
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
pub fn hashmap_put(
    map: *mut hashmap_t,
    key: tree_id_t,
    value: *mut ::core::ffi::c_void,
) -> i32 {
    if map.is_null() {
        return -1;
    }

    if unsafe { should_resize(map) } != 0 && unsafe { hashmap_resize(map) } != 0 {
        return -1;
    }

    let map_ref = unsafe { &mut *map };
    let hash = hash_function(key);
    let index = (hash as usize).wrapping_rem(map_ref.capacity);
    let mut probe = 0usize;

    while probe < map_ref.capacity {
        let current = index.wrapping_add(probe).wrapping_rem(map_ref.capacity);
        let entry = unsafe { &mut *map_ref.entries.add(current) };

        if entry.occupied == 0 {
            entry.key = key;
            entry.value = value;
            entry.occupied = 1;
            entry.deleted = 0;
            map_ref.size = map_ref.size.wrapping_add(1);
            return 0;
        } else if entry.deleted != 0 {
            entry.key = key;
            entry.value = value;
            entry.deleted = 0;
            map_ref.size = map_ref.size.wrapping_add(1);
            map_ref.deleted_count = map_ref.deleted_count.wrapping_sub(1);
            return 0;
        } else if entry.key == key {
            entry.value = value;
            return 0;
        }

        probe = probe.wrapping_add(1);
    }

    -1
}

#[no_mangle]
pub fn hashmap_get(map: Option<&hashmap_t>, key: tree_id_t) -> Option<*mut ::core::ffi::c_void> {
    let map = match map {
        Some(map) => map,
        None => return None,
    };

    let hash: uint64_t = hash_function(key);
    let index: size_t = (hash as size_t).wrapping_rem(map.capacity);
    let mut probe: size_t = 0 as size_t;

    while probe < map.capacity {
        let current: size_t = index.wrapping_add(probe).wrapping_rem(map.capacity);
        let entry = unsafe { &*map.entries.add(current) };

        if entry.occupied == 0 {
            return None;
        }

        if entry.deleted == 0 && entry.key == key {
            return Some(entry.value);
        }

        probe = probe.wrapping_add(1);
    }

    None
}

#[no_mangle]
pub fn hashmap_remove(
    map: Option<&mut hashmap_t>,
    key: tree_id_t,
) -> Option<*mut ::core::ffi::c_void> {
    let map = map?;

    let hash: uint64_t = hash_function(key);
    let index: size_t = (hash as size_t).wrapping_rem(map.capacity);
    let mut probe: size_t = 0 as size_t;

    while probe < map.capacity {
        let current: size_t = index.wrapping_add(probe).wrapping_rem(map.capacity);

        let entry = match unsafe { map.entries.add(current as usize).as_mut() } {
            Some(entry) => entry,
            None => return None,
        };

        if entry.occupied == 0 {
            return None;
        }

        if entry.deleted == 0 && entry.key == key {
            let value = entry.value;
            entry.deleted = 1 as ::core::ffi::c_int;
            map.size = map.size.wrapping_sub(1);
            map.deleted_count = map.deleted_count.wrapping_add(1);
            return Some(value);
        }

        probe = probe.wrapping_add(1);
    }

    None
}

#[no_mangle]
pub fn hashmap_contains(map: Option<&hashmap_t>, key: tree_id_t) -> bool {
    hashmap_get(map, key).is_some()
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
