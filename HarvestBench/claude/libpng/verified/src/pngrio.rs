//! Translation of `c_src/src/pngrio.c`

use crate::*;

/* png_read_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_data(png_ptr: png_structrp, data: png_bytep, length: usize) {
    if (*png_ptr).read_data_fn.is_some() {
        ((*png_ptr).read_data_fn.unwrap())(png_ptr, data, length);
    } else {
        png_error(
            png_ptr,
            b"Call to NULL read function\0".as_ptr() as png_const_charp,
        );
    }
}

/* png_default_read_data */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_read_data(
    png_ptr: png_structp,
    data: png_bytep,
    length: usize,
) {
    let check: usize;

    if png_ptr == core::ptr::null_mut() {
        return;
    }

    check = fread(data as *mut c_void, 1, length, (*png_ptr).io_ptr as *mut FILE);

    if check != length {
        png_error(png_ptr, b"Read Error\0".as_ptr() as png_const_charp);
    }
}

/* png_set_read_fn */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_fn(
    png_ptr: png_structrp,
    io_ptr: png_voidp,
    read_data_fn: png_rw_ptr,
) {
    if png_ptr == core::ptr::null_mut() {
        return;
    }

    (*png_ptr).io_ptr = io_ptr;

    if read_data_fn.is_some() {
        (*png_ptr).read_data_fn = read_data_fn;
    } else {
        (*png_ptr).read_data_fn = Some(png_default_read_data);
    }

    /* It is an error to write to a read device */
    if (*png_ptr).write_data_fn.is_some() {
        (*png_ptr).write_data_fn = None;
        png_warning(
            png_ptr,
            b"Can't set both read_data_fn and write_data_fn in the same structure\0".as_ptr()
                as png_const_charp,
        );
    }

    (*png_ptr).output_flush_fn = None;
}
