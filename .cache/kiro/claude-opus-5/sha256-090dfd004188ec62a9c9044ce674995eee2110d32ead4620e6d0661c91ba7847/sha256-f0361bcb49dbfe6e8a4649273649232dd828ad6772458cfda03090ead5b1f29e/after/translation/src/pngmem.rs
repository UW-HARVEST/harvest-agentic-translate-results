//! Translation of c_src/src/pngmem.c lines 1..287
use crate::prelude::*;

/* Free a png_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_png_struct(png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        /* png_free might call png_error and may certainly call
         * png_get_mem_ptr, so fake a temporary png_struct to support this.
         */
        let mut dummy_struct: png_struct = core::ptr::read(png_ptr);
        memset(
            png_ptr as *mut c_void,
            0,
            core::mem::size_of::<png_struct>(),
        );
        png_free(&mut dummy_struct, png_ptr as png_voidp);

        /* PNG_SETJMP_SUPPORTED */
        /* We may have a jmp_buf left to deallocate. */
        png_free_jmpbuf(&mut dummy_struct);
    }
}

/* Allocate memory.  For reasonable files, size should never exceed
 * 64K.  However, zlib may allocate more than 64K if you don't tell
 * it not to.  See zconf.h and png.h for more information.  zlib does
 * need to allocate exactly 64K, so whatever you call here must
 * have the ability to do that.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_calloc(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    let ret: png_voidp;

    ret = png_malloc(png_ptr, size);

    if !ret.is_null() {
        memset(ret, 0, size);
    }

    ret
}

/* png_malloc_base, an internal function added at libpng 1.6.0, does the work of
 * allocating memory, taking into account limits and PNG_USER_MEM_SUPPORTED.
 * Checking and error handling must happen outside this routine; it returns NULL
 * if the allocation cannot be done (for any reason.)
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_base(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    /* PNG_MAX_MALLOC_64K is NOT defined. */

    /* This is checked too because the system malloc call below takes a (size_t).
     */
    if size > PNG_SIZE_MAX {
        return core::ptr::null_mut();
    }

    /* PNG_USER_MEM_SUPPORTED */
    if !png_ptr.is_null() && (*png_ptr).malloc_fn.is_some() {
        if let Some(f) = (*png_ptr).malloc_fn {
            return f(png_ptr as png_structrp, size);
        }
    }

    /* Use the system malloc */
    malloc(size) /* checked for truncation above */
}

/* PNG_TEXT_SUPPORTED || PNG_sPLT_SUPPORTED || PNG_STORE_UNKNOWN_CHUNKS_SUPPORTED */

/* This is really here only to work round a spurious warning in GCC 4.6 and 4.7
 * that arises because of the checks in png_realloc_array that are repeated in
 * png_malloc_array.
 */
pub unsafe extern "C" fn png_malloc_array_checked(
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_array(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: usize,
) -> png_voidp {
    if nelements <= 0 || element_size == 0 {
        png_error(png_ptr, cstr(b"internal error: array alloc\0"));
    }

    png_malloc_array_checked(png_ptr, nelements, element_size)
}

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
        || (old_array.is_null() && old_elements > 0)
    {
        png_error(png_ptr, cstr(b"internal error: array realloc\0"));
    }

    /* Check for overflow on the elements count (so the caller does not have to
     * check.)
     */
    if add_elements <= c_int::MAX - old_elements {
        let new_array: png_voidp =
            png_malloc_array_checked(png_ptr, old_elements + add_elements, element_size);

        if !new_array.is_null() {
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

/* Various functions that have different error handling are derived from this.
 * png_malloc always exists, but if PNG_USER_MEM_SUPPORTED is defined a separate
 * function png_malloc_default is also provided.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    let ret: png_voidp;

    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    ret = png_malloc_base(png_ptr, size);

    if ret.is_null() {
        png_error(png_ptr, cstr(b"Out of memory\0")); /* 'm' means png_malloc */
    }

    ret
}

/* PNG_USER_MEM_SUPPORTED */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_default(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    let ret: png_voidp;

    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    /* Passing 'NULL' here bypasses the application provided memory handler. */
    ret = png_malloc_base(core::ptr::null() /*use malloc*/, size);

    if ret.is_null() {
        png_error(png_ptr, cstr(b"Out of Memory\0")); /* 'M' means png_malloc_default */
    }

    ret
}

/* This function was added at libpng version 1.2.3.  The png_malloc_warn()
 * function will issue a png_warning and return NULL instead of issuing a
 * png_error, if it fails to allocate the requested memory.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_malloc_warn(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    if !png_ptr.is_null() {
        let ret: png_voidp = png_malloc_base(png_ptr, size);

        if !ret.is_null() {
            return ret;
        }

        png_warning(png_ptr, cstr(b"Out of memory\0"));
    }

    core::ptr::null_mut()
}

/* Free a pointer allocated by png_malloc().  If ptr is NULL, return
 * without taking any action.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free(png_ptr: png_const_structrp, ptr: png_voidp) {
    if png_ptr.is_null() || ptr.is_null() {
        return;
    }

    /* PNG_USER_MEM_SUPPORTED */
    if (*png_ptr).free_fn.is_some() {
        if let Some(f) = (*png_ptr).free_fn {
            f(png_ptr as png_structrp, ptr);
        }
    } else {
        png_free_default(png_ptr, ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_default(png_ptr: png_const_structrp, ptr: png_voidp) {
    if png_ptr.is_null() || ptr.is_null() {
        return;
    }

    free(ptr);
}

/* PNG_USER_MEM_SUPPORTED */
/* This function is called when the application wants to use another method
 * of allocating and freeing memory.
 */
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

/* This function returns a pointer to the mem_ptr associated with the user
 * functions.  The application should free any memory associated with this
 * pointer before png_write_destroy and png_read_destroy are called.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mem_ptr(png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return core::ptr::null_mut();
    }

    (*png_ptr).mem_ptr
}
