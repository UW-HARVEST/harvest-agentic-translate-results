//! Translation of pngmem.c - memory allocation.
use crate::prelude::*;

/// Free a png_struct.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_png_struct(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        // png_free might call png_error and may call png_get_mem_ptr, so fake a
        // temporary png_struct to support this.
        let mut dummy_struct: png_struct_def = core::ptr::read(png_ptr);
        memset(png_ptr as *mut c_void, 0, core::mem::size_of::<png_struct_def>());
        png_free(&mut dummy_struct, png_ptr as png_voidp);

        // We may have a jmp_buf left to deallocate.
        png_free_jmpbuf(&mut dummy_struct);
    }
}

/// Allocate memory and zero it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_calloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp {
    let ret = png_malloc(png_ptr, size);
    if !ret.is_null() {
        memset(ret, 0, size);
    }
    ret
}

/// Base allocator: honours user malloc_fn and limits, returns NULL on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_base(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    // size > PNG_SIZE_MAX check (PNG_SIZE_MAX == usize::MAX so always false, but
    // kept for fidelity).
    if size > PNG_SIZE_MAX {
        return ptr::null_mut();
    }

    if !png_ptr.is_null() && (*png_ptr).malloc_fn.is_some() {
        return ((*png_ptr).malloc_fn.unwrap())(png_ptr as png_structrp, size);
    }

    malloc(size as size_t)
}

unsafe fn png_malloc_array_checked(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: size_t,
) -> png_voidp {
    let req = nelements as png_alloc_size_t; // known to be > 0

    if req <= PNG_SIZE_MAX / element_size {
        return png_malloc_base(png_ptr, req * element_size);
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_array(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: size_t,
) -> png_voidp {
    if nelements <= 0 || element_size == 0 {
        png_error(png_ptr, c"internal error: array alloc".as_ptr());
    }

    png_malloc_array_checked(png_ptr, nelements, element_size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_realloc_array(
    png_ptr: png_const_structrp,
    old_array: png_const_voidp,
    old_elements: c_int,
    add_elements: c_int,
    element_size: size_t,
) -> png_voidp {
    // These are internal errors:
    if add_elements <= 0
        || element_size == 0
        || old_elements < 0
        || (old_array.is_null() && old_elements > 0)
    {
        png_error(png_ptr, c"internal error: array realloc".as_ptr());
    }

    if add_elements <= c_int::MAX - old_elements {
        let new_array =
            png_malloc_array_checked(png_ptr, old_elements + add_elements, element_size);

        if !new_array.is_null() {
            if old_elements > 0 {
                memcpy(
                    new_array,
                    old_array,
                    element_size * (old_elements as usize),
                );
            }

            memset(
                (new_array as *mut u8).add(element_size * (old_elements as usize)) as *mut c_void,
                0,
                element_size * (add_elements as usize),
            );

            return new_array;
        }
    }

    ptr::null_mut()
}

/// Allocate memory, error out on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp {
    if png_ptr.is_null() {
        return ptr::null_mut();
    }

    let ret = png_malloc_base(png_ptr, size);

    if ret.is_null() {
        png_error(png_ptr, c"Out of memory".as_ptr());
    }

    ret
}

/// Bypass any user allocator (default allocator).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_default(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    if png_ptr.is_null() {
        return ptr::null_mut();
    }

    let ret = png_malloc_base(ptr::null(), size);

    if ret.is_null() {
        png_error(png_ptr, c"Out of Memory".as_ptr());
    }

    ret
}

/// Allocate memory, warn and return NULL on failure.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_warn(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    if !png_ptr.is_null() {
        let ret = png_malloc_base(png_ptr, size);
        if !ret.is_null() {
            return ret;
        }
        png_warning(png_ptr, c"Out of memory".as_ptr());
    }
    ptr::null_mut()
}

/// Free memory allocated by png_malloc.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free(png_ptr: png_const_structrp, ptr_: png_voidp) {
    if png_ptr.is_null() || ptr_.is_null() {
        return;
    }

    if (*png_ptr).free_fn.is_some() {
        ((*png_ptr).free_fn.unwrap())(png_ptr as png_structrp, ptr_);
    } else {
        png_free_default(png_ptr, ptr_);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_default(png_ptr: png_const_structrp, ptr_: png_voidp) {
    if png_ptr.is_null() || ptr_.is_null() {
        return;
    }

    free(ptr_);
}

/// Set user memory functions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mem_fn(
    png_ptr: png_structrp,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) {
    if !png_ptr.is_null() {
        (*png_ptr).mem_ptr = mem_ptr;
        (*png_ptr).malloc_fn = malloc_fn;
        (*png_ptr).free_fn = free_fn;
    }
}

/// Get the user mem_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mem_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return ptr::null_mut();
    }
    (*png_ptr).mem_ptr
}
