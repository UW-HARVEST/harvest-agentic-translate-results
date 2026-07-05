









extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn realloc(__ptr: *mut ::core::ffi::c_void, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
}
pub type size_t = usize;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct item_t {
    pub id: ::core::ffi::c_int,
    pub name: [::core::ffi::c_char; 64],
    pub category: [::core::ffi::c_char; 32],
    pub price: ::core::ffi::c_double,
    pub quantity: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct order_t {
    pub customer_id: ::core::ffi::c_int,
    pub customer_name: [::core::ffi::c_char; 64],
    pub total_amount: ::core::ffi::c_double,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct array_int_t {
    pub data: *mut ::core::ffi::c_int,
    pub size: size_t,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct array_double_t {
    pub data: *mut ::core::ffi::c_double,
    pub size: size_t,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct array_item_t_t {
    pub data: *mut item_t,
    pub size: size_t,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct array_order_t_t {
    pub data: *mut order_t,
    pub size: size_t,
    pub capacity: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_node_int {
    pub data: ::core::ffi::c_int,
    pub next: *mut list_node_int,
}
pub type list_node_int_t = list_node_int;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_int_t {
    pub head: *mut list_node_int_t,
    pub tail: *mut list_node_int_t,
    pub size: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_node_double {
    pub data: ::core::ffi::c_double,
    pub next: *mut list_node_double,
}
pub type list_node_double_t = list_node_double;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_double_t {
    pub head: *mut list_node_double_t,
    pub tail: *mut list_node_double_t,
    pub size: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_node_item_t {
    pub data: item_t,
    pub next: *mut list_node_item_t,
}
pub type list_node_item_t_t = list_node_item_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_item_t_t {
    pub head: *mut list_node_item_t_t,
    pub tail: *mut list_node_item_t_t,
    pub size: size_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_node_order_t {
    pub data: order_t,
    pub next: *mut list_node_order_t,
}
pub type list_node_order_t_t = list_node_order_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct list_order_t_t {
    pub head: *mut list_node_order_t_t,
    pub tail: *mut list_node_order_t_t,
    pub size: size_t,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const MAX_NAME_LENGTH: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const MAX_CATEGORY_LENGTH: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
#[no_mangle]
pub fn array_int_create(initial_capacity: size_t) -> Option<array_int_t> {
    let capacity = if initial_capacity > 0 {
        initial_capacity
    } else {
        16
    };

    let data = vec![0; capacity as usize];

    Some(array_int_t {
        capacity,
        size: 0,
        data,
    })
}

#[no_mangle]
pub unsafe extern "C" fn array_int_destroy(mut arr: *mut array_int_t) {
    if !arr.is_null() {
        free((*arr).data as *mut ::core::ffi::c_void);
        free(arr as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn array_int_push(
    mut arr: *mut array_int_t,
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if arr.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    if (*arr).size >= (*arr).capacity {
        let mut new_capacity: size_t = (*arr).capacity.wrapping_mul(2 as size_t);
        let mut new_data: *mut ::core::ffi::c_int = realloc(
            (*arr).data as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<::core::ffi::c_int>() as size_t).wrapping_mul(new_capacity),
        ) as *mut ::core::ffi::c_int;
        if new_data.is_null() {
            return -(1 as ::core::ffi::c_int);
        }
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    let fresh0 = (*arr).size;
    (*arr).size = (*arr).size.wrapping_add(1);
    *(*arr).data.offset(fresh0 as isize) = value;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn array_int_size(mut arr: *mut array_int_t) -> size_t {
    return if !arr.is_null() {
        (*arr).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub unsafe extern "C" fn array_int_clear(mut arr: *mut array_int_t) {
    if !arr.is_null() {
        (*arr).size = 0 as size_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn array_int_get(
    mut arr: *mut array_int_t,
    mut index: size_t,
) -> ::core::ffi::c_int {
    return *(*arr).data.offset(index as isize);
}
#[no_mangle]
pub unsafe extern "C" fn array_double_create(mut initial_capacity: size_t) -> *mut array_double_t {
    let mut arr: *mut array_double_t =
        malloc(::core::mem::size_of::<array_double_t>() as size_t) as *mut array_double_t;
    if arr.is_null() {
        return ::core::ptr::null_mut::<array_double_t>();
    }
    (*arr).capacity = if initial_capacity > 0 as size_t {
        initial_capacity
    } else {
        16 as size_t
    };
    (*arr).size = 0 as size_t;
    (*arr).data = malloc(
        (::core::mem::size_of::<::core::ffi::c_double>() as size_t).wrapping_mul((*arr).capacity),
    ) as *mut ::core::ffi::c_double;
    if (*arr).data.is_null() {
        free(arr as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<array_double_t>();
    }
    return arr;
}
#[no_mangle]
pub fn array_double_clear(arr: Option<&mut array_double_t>) {
    if let Some(arr) = arr {
        arr.size = 0;
    }
}

#[no_mangle]
pub unsafe extern "C" fn array_double_size(mut arr: *mut array_double_t) -> size_t {
    return if !arr.is_null() {
        (*arr).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub unsafe extern "C" fn array_double_destroy(mut arr: *mut array_double_t) {
    if !arr.is_null() {
        free((*arr).data as *mut ::core::ffi::c_void);
        free(arr as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn array_double_push(
    mut arr: *mut array_double_t,
    mut value: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    if arr.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    if (*arr).size >= (*arr).capacity {
        let mut new_capacity: size_t = (*arr).capacity.wrapping_mul(2 as size_t);
        let mut new_data: *mut ::core::ffi::c_double = realloc(
            (*arr).data as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<::core::ffi::c_double>() as size_t).wrapping_mul(new_capacity),
        ) as *mut ::core::ffi::c_double;
        if new_data.is_null() {
            return -(1 as ::core::ffi::c_int);
        }
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    let fresh1 = (*arr).size;
    (*arr).size = (*arr).size.wrapping_add(1);
    *(*arr).data.offset(fresh1 as isize) = value;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn array_double_get(
    mut arr: *mut array_double_t,
    mut index: size_t,
) -> ::core::ffi::c_double {
    return *(*arr).data.offset(index as isize);
}
#[no_mangle]
pub fn array_item_t_create(initial_capacity: size_t) -> *mut array_item_t_t {
    let capacity = if initial_capacity > 0 as size_t {
        initial_capacity
    } else {
        16 as size_t
    };

    let mut data: Vec<item_t> = Vec::with_capacity(capacity as usize);
    let data_ptr = data.as_mut_ptr();
    ::core::mem::forget(data);

    Box::into_raw(Box::new(array_item_t_t {
        capacity,
        size: 0 as size_t,
        data: data_ptr,
    }))
}

#[no_mangle]
pub unsafe extern "C" fn array_item_t_destroy(mut arr: *mut array_item_t_t) {
    if !arr.is_null() {
        free((*arr).data as *mut ::core::ffi::c_void);
        free(arr as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn array_item_t_size(mut arr: *mut array_item_t_t) -> size_t {
    return if !arr.is_null() {
        (*arr).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub unsafe extern "C" fn array_item_t_push(
    mut arr: *mut array_item_t_t,
    mut value: item_t,
) -> ::core::ffi::c_int {
    if arr.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    if (*arr).size >= (*arr).capacity {
        let mut new_capacity: size_t = (*arr).capacity.wrapping_mul(2 as size_t);
        let mut new_data: *mut item_t = realloc(
            (*arr).data as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<item_t>() as size_t).wrapping_mul(new_capacity),
        ) as *mut item_t;
        if new_data.is_null() {
            return -(1 as ::core::ffi::c_int);
        }
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    let fresh2 = (*arr).size;
    (*arr).size = (*arr).size.wrapping_add(1);
    *(*arr).data.offset(fresh2 as isize) = value;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn array_item_t_clear(mut arr: *mut array_item_t_t) {
    if !arr.is_null() {
        (*arr).size = 0 as size_t;
    }
}
#[no_mangle]
pub unsafe extern "C" fn array_item_t_get(
    mut arr: *mut array_item_t_t,
    mut index: size_t,
) -> item_t {
    return *(*arr).data.offset(index as isize);
}
#[no_mangle]
pub unsafe extern "C" fn array_order_t_size(mut arr: *mut array_order_t_t) -> size_t {
    return if !arr.is_null() {
        (*arr).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub fn array_order_t_clear(arr: Option<&mut array_order_t_t>) {
    if let Some(arr) = arr {
        arr.size = 0 as size_t;
    }
}

#[no_mangle]
pub unsafe extern "C" fn array_order_t_create(
    mut initial_capacity: size_t,
) -> *mut array_order_t_t {
    let mut arr: *mut array_order_t_t =
        malloc(::core::mem::size_of::<array_order_t_t>() as size_t) as *mut array_order_t_t;
    if arr.is_null() {
        return ::core::ptr::null_mut::<array_order_t_t>();
    }
    (*arr).capacity = if initial_capacity > 0 as size_t {
        initial_capacity
    } else {
        16 as size_t
    };
    (*arr).size = 0 as size_t;
    (*arr).data =
        malloc((::core::mem::size_of::<order_t>() as size_t).wrapping_mul((*arr).capacity))
            as *mut order_t;
    if (*arr).data.is_null() {
        free(arr as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<array_order_t_t>();
    }
    return arr;
}
#[no_mangle]
pub unsafe extern "C" fn array_order_t_destroy(mut arr: *mut array_order_t_t) {
    if !arr.is_null() {
        free((*arr).data as *mut ::core::ffi::c_void);
        free(arr as *mut ::core::ffi::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn array_order_t_push(
    mut arr: *mut array_order_t_t,
    mut value: order_t,
) -> ::core::ffi::c_int {
    if arr.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    if (*arr).size >= (*arr).capacity {
        let mut new_capacity: size_t = (*arr).capacity.wrapping_mul(2 as size_t);
        let mut new_data: *mut order_t = realloc(
            (*arr).data as *mut ::core::ffi::c_void,
            (::core::mem::size_of::<order_t>() as size_t).wrapping_mul(new_capacity),
        ) as *mut order_t;
        if new_data.is_null() {
            return -(1 as ::core::ffi::c_int);
        }
        (*arr).data = new_data;
        (*arr).capacity = new_capacity;
    }
    let fresh3 = (*arr).size;
    (*arr).size = (*arr).size.wrapping_add(1);
    *(*arr).data.offset(fresh3 as isize) = value;
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe fn array_order_t_get(arr: &array_order_t_t, index: usize) -> order_t {
    *arr.data.add(index)
}

#[no_mangle]
pub unsafe extern "C" fn list_int_prepend(
    mut list: *mut list_int_t,
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if list.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut node: *mut list_node_int_t =
        malloc(::core::mem::size_of::<list_node_int_t>() as size_t) as *mut list_node_int_t;
    if node.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*node).data = value;
    (*node).next = (*list).head as *mut list_node_int;
    (*list).head = node;
    if (*list).tail.is_null() {
        (*list).tail = node;
    }
    (*list).size = (*list).size.wrapping_add(1);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn list_int_create() -> *mut list_int_t {
    let mut list: *mut list_int_t =
        malloc(::core::mem::size_of::<list_int_t>() as size_t) as *mut list_int_t;
    if list.is_null() {
        return ::core::ptr::null_mut::<list_int_t>();
    }
    (*list).head = ::core::ptr::null_mut::<list_node_int_t>();
    (*list).tail = ::core::ptr::null_mut::<list_node_int_t>();
    (*list).size = 0 as size_t;
    return list;
}
#[no_mangle]
pub unsafe extern "C" fn list_int_clear(mut list: *mut list_int_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_int_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_int_t = (*current).next as *mut list_node_int_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    (*list).tail = ::core::ptr::null_mut::<list_node_int_t>();
    (*list).head = (*list).tail;
    (*list).size = 0 as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn list_int_destroy(mut list: *mut list_int_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_int_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_int_t = (*current).next as *mut list_node_int_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    free(list as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn list_int_append(
    mut list: *mut list_int_t,
    mut value: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if list.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut node: *mut list_node_int_t =
        malloc(::core::mem::size_of::<list_node_int_t>() as size_t) as *mut list_node_int_t;
    if node.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*node).data = value;
    (*node).next = ::core::ptr::null_mut::<list_node_int>();
    if (*list).head.is_null() {
        (*list).tail = node;
        (*list).head = (*list).tail;
    } else {
        (*(*list).tail).next = node as *mut list_node_int;
        (*list).tail = node;
    }
    (*list).size = (*list).size.wrapping_add(1);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub fn list_int_size(list: Option<&list_int_t>) -> size_t {
    list.map_or(0 as size_t, |list| list.size)
}

#[no_mangle]
pub unsafe extern "C" fn list_double_clear(mut list: *mut list_double_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_double_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_double_t = (*current).next as *mut list_node_double_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    (*list).tail = ::core::ptr::null_mut::<list_node_double_t>();
    (*list).head = (*list).tail;
    (*list).size = 0 as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn list_double_create() -> *mut list_double_t {
    let mut list: *mut list_double_t =
        malloc(::core::mem::size_of::<list_double_t>() as size_t) as *mut list_double_t;
    if list.is_null() {
        return ::core::ptr::null_mut::<list_double_t>();
    }
    (*list).head = ::core::ptr::null_mut::<list_node_double_t>();
    (*list).tail = ::core::ptr::null_mut::<list_node_double_t>();
    (*list).size = 0 as size_t;
    return list;
}
#[no_mangle]
pub unsafe extern "C" fn list_double_destroy(mut list: *mut list_double_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_double_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_double_t = (*current).next as *mut list_node_double_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    free(list as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn list_double_append(
    mut list: *mut list_double_t,
    mut value: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    if list.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut node: *mut list_node_double_t =
        malloc(::core::mem::size_of::<list_node_double_t>() as size_t) as *mut list_node_double_t;
    if node.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*node).data = value;
    (*node).next = ::core::ptr::null_mut::<list_node_double>();
    if (*list).head.is_null() {
        (*list).tail = node;
        (*list).head = (*list).tail;
    } else {
        (*(*list).tail).next = node as *mut list_node_double;
        (*list).tail = node;
    }
    (*list).size = (*list).size.wrapping_add(1);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub fn list_double_size(list: Option<&list_double_t>) -> size_t {
    list.map_or(0 as size_t, |list| list.size)
}

#[no_mangle]
pub unsafe extern "C" fn list_double_prepend(
    mut list: *mut list_double_t,
    mut value: ::core::ffi::c_double,
) -> ::core::ffi::c_int {
    if list.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut node: *mut list_node_double_t =
        malloc(::core::mem::size_of::<list_node_double_t>() as size_t) as *mut list_node_double_t;
    if node.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*node).data = value;
    (*node).next = (*list).head as *mut list_node_double;
    (*list).head = node;
    if (*list).tail.is_null() {
        (*list).tail = node;
    }
    (*list).size = (*list).size.wrapping_add(1);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub fn list_item_t_destroy(list: *mut list_item_t_t) {
    if list.is_null() {
        return;
    }

    unsafe {
        let mut current = (*list).head;
        while !current.is_null() {
            let next = (*current).next;
            drop(Box::from_raw(current));
            current = next;
        }
        drop(Box::from_raw(list));
    }
}

#[no_mangle]
pub fn list_item_t_append(list: &mut list_item_t_t, value: item_t) -> i32 {
    let node = Box::into_raw(Box::new(list_node_item_t_t {
        data: value,
        next: core::ptr::null_mut(),
    }));

    if list.head.is_null() {
        list.head = node;
        list.tail = node;
    } else {
        unsafe {
            (*list.tail).next = node;
        }
        list.tail = node;
    }

    list.size = list.size.wrapping_add(1);
    0
}

#[no_mangle]
pub unsafe extern "C" fn list_item_t_size(mut list: *mut list_item_t_t) -> size_t {
    return if !list.is_null() {
        (*list).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub unsafe extern "C" fn list_item_t_clear(mut list: *mut list_item_t_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_item_t_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_item_t_t = (*current).next as *mut list_node_item_t_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    (*list).tail = ::core::ptr::null_mut::<list_node_item_t_t>();
    (*list).head = (*list).tail;
    (*list).size = 0 as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn list_item_t_prepend(
    mut list: *mut list_item_t_t,
    mut value: item_t,
) -> ::core::ffi::c_int {
    if list.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut node: *mut list_node_item_t_t =
        malloc(::core::mem::size_of::<list_node_item_t_t>() as size_t) as *mut list_node_item_t_t;
    if node.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*node).data = value;
    (*node).next = (*list).head as *mut list_node_item_t;
    (*list).head = node;
    if (*list).tail.is_null() {
        (*list).tail = node;
    }
    (*list).size = (*list).size.wrapping_add(1);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn list_item_t_create() -> *mut list_item_t_t {
    let mut list: *mut list_item_t_t =
        malloc(::core::mem::size_of::<list_item_t_t>() as size_t) as *mut list_item_t_t;
    if list.is_null() {
        return ::core::ptr::null_mut::<list_item_t_t>();
    }
    (*list).head = ::core::ptr::null_mut::<list_node_item_t_t>();
    (*list).tail = ::core::ptr::null_mut::<list_node_item_t_t>();
    (*list).size = 0 as size_t;
    return list;
}
#[no_mangle]
pub unsafe extern "C" fn list_order_t_create() -> *mut list_order_t_t {
    let mut list: *mut list_order_t_t =
        malloc(::core::mem::size_of::<list_order_t_t>() as size_t) as *mut list_order_t_t;
    if list.is_null() {
        return ::core::ptr::null_mut::<list_order_t_t>();
    }
    (*list).head = ::core::ptr::null_mut::<list_node_order_t_t>();
    (*list).tail = ::core::ptr::null_mut::<list_node_order_t_t>();
    (*list).size = 0 as size_t;
    return list;
}
#[no_mangle]
pub unsafe extern "C" fn list_order_t_prepend(
    mut list: *mut list_order_t_t,
    mut value: order_t,
) -> ::core::ffi::c_int {
    if list.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    let mut node: *mut list_node_order_t_t =
        malloc(::core::mem::size_of::<list_node_order_t_t>() as size_t) as *mut list_node_order_t_t;
    if node.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    (*node).data = value;
    (*node).next = (*list).head as *mut list_node_order_t;
    (*list).head = node;
    if (*list).tail.is_null() {
        (*list).tail = node;
    }
    (*list).size = (*list).size.wrapping_add(1);
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn list_order_t_size(mut list: *mut list_order_t_t) -> size_t {
    return if !list.is_null() {
        (*list).size
    } else {
        0 as size_t
    };
}
#[no_mangle]
pub unsafe extern "C" fn list_order_t_clear(mut list: *mut list_order_t_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_order_t_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_order_t_t = (*current).next as *mut list_node_order_t_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    (*list).tail = ::core::ptr::null_mut::<list_node_order_t_t>();
    (*list).head = (*list).tail;
    (*list).size = 0 as size_t;
}
#[no_mangle]
pub unsafe extern "C" fn list_order_t_destroy(mut list: *mut list_order_t_t) {
    if list.is_null() {
        return;
    }
    let mut current: *mut list_node_order_t_t = (*list).head;
    while !current.is_null() {
        let mut next: *mut list_node_order_t_t = (*current).next as *mut list_node_order_t_t;
        free(current as *mut ::core::ffi::c_void);
        current = next;
    }
    free(list as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub fn list_order_t_append(list: &mut list_order_t_t, value: order_t) -> i32 {
    let node = Box::new(list_node_order_t_t {
        data: value,
        next: std::ptr::null_mut(),
    });
    let node_ptr = Box::into_raw(node);

    if list.head.is_null() {
        list.head = node_ptr;
        list.tail = node_ptr;
    } else {
        unsafe {
            (*list.tail).next = node_ptr;
        }
        list.tail = node_ptr;
    }

    list.size = list.size.wrapping_add(1);
    0
}

#[no_mangle]
pub unsafe extern "C" fn print_item(mut item: item_t) {
    printf(
        b"  [%d] %s\n\0" as *const u8 as *const ::core::ffi::c_char,
        item.id,
        &raw mut item.name as *mut ::core::ffi::c_char,
    );
    printf(
        b"      Category: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut item.category as *mut ::core::ffi::c_char,
    );
    printf(
        b"      Price: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        item.price,
    );
    printf(
        b"      Quantity: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        item.quantity,
    );
}
#[no_mangle]
pub unsafe extern "C" fn print_order(mut order: order_t) {
    printf(
        b"  Order - Customer ID: %d, Name: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
        order.customer_id,
        &raw mut order.customer_name as *mut ::core::ffi::c_char,
    );
    printf(
        b"          Total: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        order.total_amount,
    );
}
#[no_mangle]
pub unsafe extern "C" fn create_item(
    mut id: ::core::ffi::c_int,
    mut name: *const ::core::ffi::c_char,
    mut category: *const ::core::ffi::c_char,
    mut price: ::core::ffi::c_double,
    mut quantity: ::core::ffi::c_int,
) -> item_t {
    let mut item: item_t = item_t {
        id: 0,
        name: [0; 64],
        category: [0; 32],
        price: 0.,
        quantity: 0,
    };
    item.id = id;
    strncpy(
        &raw mut item.name as *mut ::core::ffi::c_char,
        name,
        (MAX_NAME_LENGTH - 1 as ::core::ffi::c_int) as size_t,
    );
    item.name[(MAX_NAME_LENGTH - 1 as ::core::ffi::c_int) as usize] =
        '\0' as i32 as ::core::ffi::c_char;
    strncpy(
        &raw mut item.category as *mut ::core::ffi::c_char,
        category,
        (MAX_CATEGORY_LENGTH - 1 as ::core::ffi::c_int) as size_t,
    );
    item.category[(MAX_CATEGORY_LENGTH - 1 as ::core::ffi::c_int) as usize] =
        '\0' as i32 as ::core::ffi::c_char;
    item.price = price;
    item.quantity = quantity;
    return item;
}
#[no_mangle]
pub unsafe extern "C" fn create_order(
    mut customer_id: ::core::ffi::c_int,
    mut customer_name: *const ::core::ffi::c_char,
    mut total_amount: ::core::ffi::c_double,
) -> order_t {
    let mut order: order_t = order_t {
        customer_id: 0,
        customer_name: [0; 64],
        total_amount: 0.,
    };
    order.customer_id = customer_id;
    strncpy(
        &raw mut order.customer_name as *mut ::core::ffi::c_char,
        customer_name,
        (MAX_NAME_LENGTH - 1 as ::core::ffi::c_int) as size_t,
    );
    order.customer_name[(MAX_NAME_LENGTH - 1 as ::core::ffi::c_int) as usize] =
        '\0' as i32 as ::core::ffi::c_char;
    order.total_amount = total_amount;
    return order;
}
#[no_mangle]
pub unsafe extern "C" fn calculate_inventory_stats(mut items: *mut array_item_t_t) {
    if items.is_null() || (*items).size == 0 as size_t {
        printf(b"No items in inventory\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(
        b"\n=== Inventory Statistics (Array) ===\n\0" as *const u8 as *const ::core::ffi::c_char,
    );
    let mut total_value: ::core::ffi::c_double = 0.0f64;
    let mut total_items: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut max_price: ::core::ffi::c_double = 0.0f64;
    let mut min_price: ::core::ffi::c_double =
        (*(*items).data.offset(0 as ::core::ffi::c_int as isize)).price;
    let mut item: item_t = item_t {
        id: 0,
        name: [0; 64],
        category: [0; 32],
        price: 0.,
        quantity: 0,
    };
    let mut _i: size_t = 0 as size_t;
    while _i < (*items).size && {
        item = *(*items).data.offset(_i as isize);
        1 as ::core::ffi::c_int != 0
    } {
        total_value += item.price * item.quantity as ::core::ffi::c_double;
        total_items += item.quantity;
        if item.price > max_price {
            max_price = item.price;
        }
        if item.price < min_price {
            min_price = item.price;
        }
        _i = _i.wrapping_add(1);
    }
    printf(
        b"Total unique items: %zu\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*items).size,
    );
    printf(
        b"Total item count: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        total_items,
    );
    printf(
        b"Total inventory value: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        total_value,
    );
    printf(
        b"Average item price: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        total_value / total_items as ::core::ffi::c_double,
    );
    printf(
        b"Most expensive item: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        max_price,
    );
    printf(
        b"Least expensive item: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        min_price,
    );
}
#[no_mangle]
pub unsafe extern "C" fn calculate_order_stats(mut orders: *mut list_order_t_t) {
    if orders.is_null() || (*orders).size == 0 as size_t {
        printf(b"No orders to analyze\n\0" as *const u8 as *const ::core::ffi::c_char);
        return;
    }
    printf(b"\n=== Order Statistics (List) ===\n\0" as *const u8 as *const ::core::ffi::c_char);
    let mut total_revenue: ::core::ffi::c_double = 0.0f64;
    let mut max_order: ::core::ffi::c_double = 0.0f64;
    let mut min_order: ::core::ffi::c_double = -1.0f64;
    let mut order: order_t = order_t {
        customer_id: 0,
        customer_name: [0; 64],
        total_amount: 0.,
    };
    let mut _node: *mut list_node_order_t_t = (*orders).head;
    while !_node.is_null() && {
        order = (*_node).data;
        1 as ::core::ffi::c_int != 0
    } {
        total_revenue += order.total_amount;
        if order.total_amount > max_order {
            max_order = order.total_amount;
        }
        if min_order < 0 as ::core::ffi::c_int as ::core::ffi::c_double
            || order.total_amount < min_order
        {
            min_order = order.total_amount;
        }
        _node = (*_node).next as *mut list_node_order_t_t;
    }
    printf(
        b"Total orders: %zu\n\0" as *const u8 as *const ::core::ffi::c_char,
        (*orders).size,
    );
    printf(
        b"Total revenue: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        total_revenue,
    );
    printf(
        b"Average order value: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        total_revenue / (*orders).size as ::core::ffi::c_double,
    );
    printf(
        b"Largest order: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        max_order,
    );
    printf(
        b"Smallest order: $%.2f\n\0" as *const u8 as *const ::core::ffi::c_char,
        min_order,
    );
}
#[no_mangle]
pub unsafe extern "C" fn find_items_by_category(
    mut items: *mut array_item_t_t,
    mut category: *const ::core::ffi::c_char,
) {
    if items.is_null() || category.is_null() {
        return;
    }
    printf(
        b"\n=== Items in category '%s' ===\n\0" as *const u8 as *const ::core::ffi::c_char,
        category,
    );
    let mut found: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut item: item_t = item_t {
        id: 0,
        name: [0; 64],
        category: [0; 32],
        price: 0.,
        quantity: 0,
    };
    let mut _i: size_t = 0 as size_t;
    while _i < (*items).size && {
        item = *(*items).data.offset(_i as isize);
        1 as ::core::ffi::c_int != 0
    } {
        if strcmp(&raw mut item.category as *mut ::core::ffi::c_char, category)
            == 0 as ::core::ffi::c_int
        {
            print_item(item);
            found += 1;
        }
        _i = _i.wrapping_add(1);
    }
    if found == 0 as ::core::ffi::c_int {
        printf(b"No items found in this category\n\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        printf(
            b"Found %d items\n\0" as *const u8 as *const ::core::ffi::c_char,
            found,
        );
    };
}
#[no_mangle]
pub unsafe extern "C" fn find_expensive_items(
    mut items: *mut list_item_t_t,
    mut min_price: ::core::ffi::c_double,
) {
    if items.is_null() {
        return;
    }
    printf(
        b"\n=== Items priced above $%.2f ===\n\0" as *const u8 as *const ::core::ffi::c_char,
        min_price,
    );
    let mut found: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut item: item_t = item_t {
        id: 0,
        name: [0; 64],
        category: [0; 32],
        price: 0.,
        quantity: 0,
    };
    let mut _node: *mut list_node_item_t_t = (*items).head;
    while !_node.is_null() && {
        item = (*_node).data;
        1 as ::core::ffi::c_int != 0
    } {
        if item.price >= min_price {
            print_item(item);
            found += 1;
        }
        _node = (*_node).next as *mut list_node_item_t_t;
    }
    if found == 0 as ::core::ffi::c_int {
        printf(b"No items found above this price\n\0" as *const u8 as *const ::core::ffi::c_char);
    } else {
        printf(
            b"Found %d items\n\0" as *const u8 as *const ::core::ffi::c_char,
            found,
        );
    };
}
