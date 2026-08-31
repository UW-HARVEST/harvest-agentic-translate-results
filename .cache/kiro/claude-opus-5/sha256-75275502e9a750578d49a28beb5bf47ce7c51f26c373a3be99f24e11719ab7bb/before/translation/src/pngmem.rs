//! Translation of pngmem.c

use crate::*;
use core::ffi::{c_char, c_void};

/* Free a png_struct */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_destroy_png_struct(png_ptr: png_structrp) {
    unsafe {
        if !png_ptr.is_null() {
            /* png_free might call png_error and may certainly call
             * png_get_mem_ptr, so fake a temporary png_struct to support this.
             */
            let mut dummy_struct: png_struct = core::mem::MaybeUninit::zeroed().assume_init();
            core::ptr::copy_nonoverlapping(
                png_ptr as *const u8,
                &mut dummy_struct as *mut png_struct as *mut u8,
                core::mem::size_of::<png_struct>(),
            );
            memset(
                png_ptr as *mut c_void,
                0,
                core::mem::size_of::<png_struct>(),
            );
            png_free(&dummy_struct, png_ptr as png_voidp);

            /* We may have a jmp_buf left to deallocate. */
            png_free_jmpbuf(&mut dummy_struct);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_calloc(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    unsafe {
        let ret: png_voidp = png_malloc(png_ptr, size);

        if !ret.is_null() {
            memset(ret, 0, size);
        }

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_malloc_base(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    unsafe {
        /* This is checked too because the system malloc call below takes a
         * (size_t).
         */
        if size > PNG_SIZE_MAX {
            return core::ptr::null_mut();
        }

        if !png_ptr.is_null() && (*png_ptr).malloc_fn.is_some() {
            return ((*png_ptr).malloc_fn.unwrap())(png_ptr as png_structrp, size);
        }

        /* Use the system malloc */
        malloc(size)
    }
}

unsafe fn png_malloc_array_checked(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: usize,
) -> png_voidp {
    unsafe {
        let req: png_alloc_size_t = nelements as png_alloc_size_t; /* known to be > 0 */

        if req <= PNG_SIZE_MAX / element_size {
            return png_malloc_base(png_ptr, req * element_size);
        }

        /* The failure case when the request is too large */
        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_malloc_array(
    png_ptr: png_const_structrp,
    nelements: c_int,
    element_size: usize,
) -> png_voidp {
    unsafe {
        if nelements <= 0 || element_size == 0 {
            png_error(png_ptr, c"internal error: array alloc".as_ptr());
        }

        png_malloc_array_checked(png_ptr, nelements, element_size)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_realloc_array(
    png_ptr: png_const_structrp,
    old_array: png_const_voidp,
    old_elements: c_int,
    add_elements: c_int,
    element_size: usize,
) -> png_voidp {
    unsafe {
        /* These are internal errors: */
        if add_elements <= 0
            || element_size == 0
            || old_elements < 0
            || (old_array.is_null() && old_elements > 0)
        {
            png_error(png_ptr, c"internal error: array realloc".as_ptr());
        }

        /* Check for overflow on the elements count (so the caller does not have
         * to check.)
         */
        if add_elements <= c_int::MAX - old_elements {
            let new_array: png_voidp =
                png_malloc_array_checked(png_ptr, old_elements + add_elements, element_size);

            if !new_array.is_null() {
                /* Because png_malloc_array worked the size calculations below
                 * cannot overflow.
                 */
                if old_elements > 0 {
                    memcpy(
                        new_array,
                        old_array,
                        element_size * (old_elements as u32 as usize),
                    );
                }

                memset(
                    (new_array as *mut c_char).add(element_size * (old_elements as u32 as usize))
                        as *mut c_void,
                    0,
                    element_size * (add_elements as u32 as usize),
                );

                return new_array;
            }
        }

        core::ptr::null_mut() /* error */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_malloc(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    unsafe {
        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        let ret: png_voidp = png_malloc_base(png_ptr, size);

        if ret.is_null() {
            png_error(png_ptr, c"Out of memory".as_ptr()); /* 'm' means png_malloc */
        }

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_malloc_default(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    unsafe {
        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        /* Passing 'NULL' here bypasses the application provided memory handler. */
        let ret: png_voidp = png_malloc_base(core::ptr::null(), size);

        if ret.is_null() {
            png_error(png_ptr, c"Out of Memory".as_ptr()); /* 'M' means png_malloc_default */
        }

        ret
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_malloc_warn(
    png_ptr: png_const_structrp,
    size: png_alloc_size_t,
) -> png_voidp {
    unsafe {
        if !png_ptr.is_null() {
            let ret: png_voidp = png_malloc_base(png_ptr, size);

            if !ret.is_null() {
                return ret;
            }

            png_warning(png_ptr, c"Out of memory".as_ptr());
        }

        core::ptr::null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_free(png_ptr: png_const_structrp, ptr: png_voidp) {
    unsafe {
        if png_ptr.is_null() || ptr.is_null() {
            return;
        }

        if (*png_ptr).free_fn.is_some() {
            ((*png_ptr).free_fn.unwrap())(png_ptr as png_structrp, ptr);
        } else {
            png_free_default(png_ptr, ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_free_default(png_ptr: png_const_structrp, ptr: png_voidp) {
    unsafe {
        if png_ptr.is_null() || ptr.is_null() {
            return;
        }

        free(ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_mem_fn(
    png_ptr: png_structrp,
    mem_ptr: png_voidp,
    malloc_fn: png_malloc_ptr,
    free_fn: png_free_ptr,
) {
    unsafe {
        if !png_ptr.is_null() {
            (*png_ptr).mem_ptr = mem_ptr;
            (*png_ptr).malloc_fn = malloc_fn;
            (*png_ptr).free_fn = free_fn;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_get_mem_ptr(png_ptr: png_const_structrp) -> png_voidp {
    unsafe {
        if png_ptr.is_null() {
            return core::ptr::null_mut();
        }

        (*png_ptr).mem_ptr
    }
}
