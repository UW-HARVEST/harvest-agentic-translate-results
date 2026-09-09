//! Translation of c_src/src/pngwio.c lines 1..167
use crate::prelude::*;

/* Write the data to whatever output you are using.  The default routine
 * writes to a file pointer.  Note that this routine sometimes gets called
 * with very small lengths, so you should implement some kind of simple
 * buffering if you are using unbuffered writes.  This should never be asked
 * to write more than 64K on a 16-bit machine.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_data(
    png_ptr: png_structrp,
    data: png_const_bytep,
    length: usize,
) {
    /* NOTE: write_data_fn must not change the buffer! */
    if (*png_ptr).write_data_fn.is_some() {
        if let Some(f) = (*png_ptr).write_data_fn {
            f(png_ptr as png_structp, data as png_bytep, length);
        }
    } else {
        png_error(png_ptr, cstr(b"Call to NULL write function\0"));
    }
}

/* This is the function that does the actual writing of data.  If you are
 * not writing to a standard C stream, you should create a replacement
 * write_data function and use it at run time with png_set_write_fn(), rather
 * than changing the library.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_write_data(
    png_ptr: png_structp,
    data: png_bytep,
    length: usize,
) {
    let check: usize;

    if png_ptr.is_null() {
        return;
    }

    check = fwrite(
        data as *const c_void,
        1,
        length,
        (*png_ptr).io_ptr as *mut FILE,
    );

    if check != length {
        png_error(png_ptr, cstr(b"Write Error\0"));
    }
}

/* This function is called to output any data pending writing (normally
 * to disk).  After png_flush is called, there should be no data pending
 * writing in any buffers.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_flush(png_ptr: png_structrp) {
    if (*png_ptr).output_flush_fn.is_some() {
        if let Some(f) = (*png_ptr).output_flush_fn {
            f(png_ptr as png_structp);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_flush(png_ptr: png_structp) {
    let io_ptr: *mut FILE;

    if png_ptr.is_null() {
        return;
    }

    io_ptr = (*png_ptr).io_ptr as *mut FILE;
    fflush(io_ptr);
}

/* This function allows the application to supply new output functions for
 * libpng if standard C streams aren't being used.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_fn(
    png_ptr: png_structrp,
    io_ptr: png_voidp,
    write_data_fn: png_rw_ptr,
    output_flush_fn: png_flush_ptr,
) {
    if png_ptr.is_null() {
        return;
    }

    (*png_ptr).io_ptr = io_ptr;

    /* PNG_STDIO_SUPPORTED */
    if write_data_fn.is_some() {
        (*png_ptr).write_data_fn = write_data_fn;
    } else {
        (*png_ptr).write_data_fn = Some(png_default_write_data);
    }

    /* PNG_WRITE_FLUSH_SUPPORTED && PNG_STDIO_SUPPORTED */
    if output_flush_fn.is_some() {
        (*png_ptr).output_flush_fn = output_flush_fn;
    } else {
        (*png_ptr).output_flush_fn = Some(png_default_flush);
    }

    /* PNG_READ_SUPPORTED */
    /* It is an error to read while writing a png file */
    if (*png_ptr).read_data_fn.is_some() {
        (*png_ptr).read_data_fn = None;

        png_warning(
            png_ptr,
            cstr(b"Can't set both read_data_fn and write_data_fn in the same structure\0"),
        );
    }
}
