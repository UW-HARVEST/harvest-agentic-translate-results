//! Translation of c_src/src/pngrio.c lines 1..119
use crate::prelude::*;

/* Read the data from whatever input you are using.  The default routine
 * reads from a file pointer.  Note that this routine sometimes gets called
 * with very small lengths, so you should implement some kind of simple
 * buffering if you are using unbuffered reads.  This should never be asked
 * to read more than 64K on a 16-bit machine.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_data(png_ptr: png_structrp, data: png_bytep, length: usize) {
    if (*png_ptr).read_data_fn.is_some() {
        if let Some(f) = (*png_ptr).read_data_fn {
            f(png_ptr as png_structp, data, length);
        }
    } else {
        png_error(png_ptr, cstr(b"Call to NULL read function\0"));
    }
}

/* This is the function that does the actual reading of data.  If you are
 * not reading from a standard C stream, you should create a replacement
 * read_data function and use it at run time with png_set_read_fn(), rather
 * than changing the library.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_read_data(
    png_ptr: png_structp,
    data: png_bytep,
    length: usize,
) {
    let check: usize;

    if png_ptr.is_null() {
        return;
    }

    /* fread() returns 0 on error, so it is OK to store this in a size_t
     * instead of an int, which is what fread() actually returns.
     */
    check = fread(
        data as *mut c_void,
        1,
        length,
        (*png_ptr).io_ptr as *mut FILE,
    );

    if check != length {
        png_error(png_ptr, cstr(b"Read Error\0"));
    }
}

/* This function allows the application to supply a new input function
 * for libpng if standard C streams aren't being used.
 */
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

    /* PNG_STDIO_SUPPORTED */
    if read_data_fn.is_some() {
        (*png_ptr).read_data_fn = read_data_fn;
    } else {
        (*png_ptr).read_data_fn = Some(png_default_read_data);
    }

    /* PNG_WRITE_SUPPORTED */
    /* It is an error to write to a read device */
    if (*png_ptr).write_data_fn.is_some() {
        (*png_ptr).write_data_fn = None;
        png_warning(
            png_ptr,
            cstr(b"Can't set both read_data_fn and write_data_fn in the same structure\0"),
        );
    }

    /* PNG_WRITE_FLUSH_SUPPORTED */
    (*png_ptr).output_flush_fn = None;
}
