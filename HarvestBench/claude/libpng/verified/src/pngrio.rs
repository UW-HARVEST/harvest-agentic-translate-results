//! Translation of pngrio.c - functions for data input.
use crate::prelude::*;

/// Read the data from whatever input you are using.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_data(png_ptr: png_structrp, data: png_bytep, length: size_t) {
    if !png_ptr.is_null() && (*png_ptr).read_data_fn.is_some() {
        ((*png_ptr).read_data_fn.unwrap())(png_ptr, data, length);
    } else {
        png_error(png_ptr, c"Call to NULL read function".as_ptr());
    }
}

/// Default reader: reads from a C FILE* stored in io_ptr.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_read_data(png_ptr: png_structp, data: png_bytep, length: size_t) {
    if png_ptr.is_null() {
        return;
    }
    let check = fread(
        data as *mut c_void,
        1,
        length,
        (*png_ptr).io_ptr as *mut FILE,
    );
    if check != length {
        png_error(png_ptr, c"Read Error".as_ptr());
    }
}

/// Set a new input function for libpng.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_fn(
    png_ptr: png_structrp,
    io_ptr: png_voidp,
    read_data_fn: png_rw_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).io_ptr = io_ptr;

    if read_data_fn.is_some() {
        (*png_ptr).read_data_fn = read_data_fn;
    } else {
        (*png_ptr).read_data_fn = Some(png_default_read_data);
    }

    // It is an error to write to a read device.
    if (*png_ptr).write_data_fn.is_some() {
        (*png_ptr).write_data_fn = None;
        png_warning(
            png_ptr,
            c"Can't set both read_data_fn and write_data_fn in the same structure".as_ptr(),
        );
    }

    (*png_ptr).output_flush_fn = None;
}
