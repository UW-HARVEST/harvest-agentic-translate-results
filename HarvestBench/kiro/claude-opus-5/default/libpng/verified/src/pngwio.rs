//! Translation of pngwio.c

use crate::*;

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_write_data(
    png_ptr: png_structrp,
    data: png_const_bytep,
    length: usize,
) {
    unsafe {
        /* NOTE: write_data_fn must not change the buffer! */
        if (*png_ptr).write_data_fn.is_some() {
            ((*png_ptr).write_data_fn.unwrap())(png_ptr, data as png_bytep, length);
        } else {
            png_error(png_ptr, c"Call to NULL write function".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_default_write_data(
    png_ptr: png_structp,
    data: png_bytep,
    length: usize,
) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        let check: usize = fwrite(
            data as *const c_void,
            1,
            length,
            (*png_ptr).io_ptr as *mut FILE,
        );

        if check != length {
            png_error(png_ptr, c"Write Error".as_ptr());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_flush(png_ptr: png_structrp) {
    unsafe {
        if (*png_ptr).output_flush_fn.is_some() {
            ((*png_ptr).output_flush_fn.unwrap())(png_ptr);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_default_flush(png_ptr: png_structp) {
    unsafe {
        if png_ptr.is_null() {
            return;
        }

        let io_ptr: *mut FILE = (*png_ptr).io_ptr as *mut FILE;
        fflush(io_ptr);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn png_set_write_fn(
    png_ptr: png_structrp,
    io_ptr: png_voidp,
    write_data_fn: png_rw_ptr,
    output_flush_fn: png_flush_ptr,
) {
    unsafe {
        if png_ptr.is_null() {
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
                c"Can't set both read_data_fn and write_data_fn in the same structure".as_ptr(),
            );
        }
    }
}
