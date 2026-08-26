//! Translation of `c_src/src/pngwio.c`

use crate::*;

/* png_write_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_data(
    png_ptr: png_structrp,
    data: png_const_bytep,
    length: usize,
) {
    /* NOTE: write_data_fn must not change the buffer! */
    if (*png_ptr).write_data_fn.is_some() {
        ((*png_ptr).write_data_fn.unwrap())(png_ptr, data as png_bytep, length);
    } else {
        png_error(
            png_ptr,
            b"Call to NULL write function\0".as_ptr() as png_const_charp,
        );
    }
}

/* png_default_write_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_write_data(
    png_ptr: png_structp,
    data: png_bytep,
    length: usize,
) {
    let check: usize;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    check = fwrite(
        data as *const c_void,
        1,
        length,
        (*png_ptr).io_ptr as *mut FILE,
    );

    if check != length {
        png_error(png_ptr, b"Write Error\0".as_ptr() as png_const_charp);
    }
}

/* png_flush */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_flush(png_ptr: png_structrp) {
    if (*png_ptr).output_flush_fn.is_some() {
        ((*png_ptr).output_flush_fn.unwrap())(png_ptr);
    }
}

/* png_default_flush */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_flush(png_ptr: png_structp) {
    let io_ptr: *mut FILE;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    io_ptr = (*png_ptr).io_ptr as *mut FILE;
    fflush(io_ptr);
}

/* png_set_write_fn */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_fn(
    png_ptr: png_structrp,
    io_ptr: png_voidp,
    write_data_fn: png_rw_ptr,
    output_flush_fn: png_flush_ptr,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).io_ptr = io_ptr;

    if write_data_fn.is_some() {
        (*png_ptr).write_data_fn = write_data_fn;
    } else {
        (*png_ptr).write_data_fn = Some(png_default_write_data);
    }

    if output_flush_fn.is_some() {
        (*png_ptr).output_flush_fn = output_flush_fn;
    } else {
        (*png_ptr).output_flush_fn = Some(png_default_flush);
    }

    /* It is an error to read while writing a png file */
    if (*png_ptr).read_data_fn.is_some() {
        (*png_ptr).read_data_fn = None;

        png_warning(
            png_ptr,
            b"Can't set both read_data_fn and write_data_fn in the same structure\0".as_ptr()
                as png_const_charp,
        );
    }
}
