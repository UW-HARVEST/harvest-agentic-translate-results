//! Translation of `c_src/src/pngmem.c`

use crate::*;

/* png_destroy_png_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_png_struct(png_ptr: png_structrp) {
    if png_ptr != core::ptr::null_mut() {
        /* png_free might call png_error and may certainly call
         * png_get_mem_ptr, so fake a temporary png_struct to support this.
         */
        let mut dummy_struct: png_struct = core::ptr::read(png_ptr);
        memset(png_ptr as *mut c_void, 0, core::mem::size_of::<png_struct>());
        png_free(&mut dummy_struct, png_ptr as png_voidp);

        /* We may have a jmp_buf left to deallocate. */
        png_free_jmpbuf(&mut dummy_struct);
    }
}

/* png_calloc */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_calloc(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    let ret: png_voidp;

    ret = png_malloc(png_ptr, size);

    if ret != core::ptr::null_mut() {
        memset(ret, 0, size);
    }

    ret
}

/* png_malloc_base */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_base(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    /* This is checked too because the system malloc call below takes a (size_t).
     */
    if size > PNG_SIZE_MAX {
        return core::ptr::null_mut();
    }

    if png_ptr != core::ptr::null_mut() && (*png_ptr).malloc_fn.is_some() {
        return ((*png_ptr).malloc_fn.unwrap())(png_ptr, size);
    }

    /* Use the system malloc */
    malloc(size)
}

/* png_malloc_array_checked */
unsafe fn png_malloc_array_checked(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: usize,
) -> png_voidp {
    let req: png_alloc_size_t = nelements as png_alloc_size_t; /* known to be > 0 */

    if req <= PNG_SIZE_MAX / element_size {
        return png_malloc_base(png_ptr, req * element_size);
    }

    /* The failure case when the request is too large */
    core::ptr::null_mut()
}

/* png_malloc_array */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_array(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: usize,
) -> png_voidp {
    if nelements <= 0 || element_size == 0 {
        png_error(
            png_ptr,
            b"internal error: array alloc\0".as_ptr() as png_const_charp,
        );
    }

    png_malloc_array_checked(png_ptr, nelements, element_size)
}

/* png_realloc_array */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_realloc_array(
    png_ptr: png_const_structrp,
    old_array: png_const_voidp,
    old_elements: c_int,
    add_elements: c_int,
    element_size: usize,
) -> png_voidp {
    /* These are internal errors: */
    if add_elements <= 0
        || element_size == 0
        || old_elements < 0
        || (old_array == core::ptr::null() && old_elements > 0)
    {
        png_error(
            png_ptr,
            b"internal error: array realloc\0".as_ptr() as png_const_charp,
        );
    }

    /* Check for overflow on the elements count (so the caller does not have to
     * check.)
     */
    if add_elements <= INT_MAX - old_elements {
        let new_array: png_voidp =
            png_malloc_array_checked(png_ptr, old_elements + add_elements, element_size);

        if new_array != core::ptr::null_mut() {
            /* Because png_malloc_array worked the size calculations below cannot
             * overflow.
             */
            if old_elements > 0 {
                memcpy(
                    new_array,
                    old_array,
                    element_size * (old_elements as c_uint) as usize,
                );
            }

            memset(
                (new_array as *mut c_char).add(element_size * (old_elements as c_uint) as usize)
                    as *mut c_void,
                0,
                element_size * (add_elements as c_uint) as usize,
            );

            return new_array;
        }
    }

    core::ptr::null_mut() /* error */
}

/* png_malloc */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    let ret: png_voidp;

    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    ret = png_malloc_base(png_ptr, size);

    if ret == core::ptr::null_mut() {
        png_error(png_ptr, b"Out of memory\0".as_ptr() as png_const_charp);
    }

    ret
}

/* png_malloc_default */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_default(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    let ret: png_voidp;

    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    /* Passing 'NULL' here bypasses the application provided memory handler. */
    ret = png_malloc_base(core::ptr::null_mut(), size);

    if ret == core::ptr::null_mut() {
        png_error(png_ptr, b"Out of Memory\0".as_ptr() as png_const_charp);
    }

    ret
}

/* png_malloc_warn */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_warn(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    if png_ptr != core::ptr::null_mut() {
        let ret: png_voidp = png_malloc_base(png_ptr, size);

        if ret != core::ptr::null_mut() {
            return ret;
        }

        png_warning(png_ptr, b"Out of memory\0".as_ptr() as png_const_charp);
    }

    core::ptr::null_mut()
}

/* png_free */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free(png_ptr: png_const_structrp, ptr: png_voidp) {
    if png_ptr == core::ptr::null_mut() || ptr == core::ptr::null_mut() {
        return;
    }

    if (*png_ptr).free_fn.is_some() {
        ((*png_ptr).free_fn.unwrap())(png_ptr, ptr);
    } else {
        png_free_default(png_ptr, ptr);
    }
}

/* png_free_default */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_default(png_ptr: png_const_structrp, ptr: png_voidp) {
    if png_ptr == core::ptr::null_mut() || ptr == core::ptr::null_mut() {
        return;
    }

    free(ptr);
}

/* png_set_mem_fn */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mem_fn(
    png_ptr: png_structrp,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) {
    if png_ptr != core::ptr::null_mut() {
        (*png_ptr).mem_ptr = mem_ptr;
        (*png_ptr).malloc_fn = malloc_fn;
        (*png_ptr).free_fn = free_fn;
    }
}

/* png_get_mem_ptr */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mem_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr == core::ptr::null_mut() {
        return core::ptr::null_mut();
    }

    (*png_ptr).mem_ptr
}
