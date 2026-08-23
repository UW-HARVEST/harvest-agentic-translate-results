// pngrio.c - functions for data input
//
// This file provides a location for all input.  Users who need
// special handling are expected to write a function that has the same
// arguments as this and performs a similar function, but that possibly
// has a different input method.  Note that you shouldn't change this
// function, but rather write a replacement function and then make
// libpng use it at run time with png_set_read_fn(...).

use crate::*;

/* Read the data from whatever input you are using.  The default routine
 * reads from a file pointer.  Note that this routine sometimes gets called
 * with very small lengths, so you should implement some kind of simple
 * buffering if you are using unbuffered reads.  This should never be asked
 * to read more than 64K on a 16-bit machine.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_data(png_ptr: png_structrp, data: png_bytep, length: usize) {
    if (*png_ptr).read_data_fn.is_some() {
        ((*png_ptr).read_data_fn.unwrap())(png_ptr as png_structp, data, length);
    } else {
        png_error(png_ptr, cstr!("Call to NULL read function"));
    }
}

/* This is the function that does the actual reading of data.  If you are
 * not reading from a standard C stream, you should create a replacement
 * read_data function and use it at run time with png_set_read_fn(), rather
 * than changing the library.
 */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_default_read_data(png_ptr: png_structp, data: png_bytep, length: usize) {
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
        png_error(png_ptr, cstr!("Read Error"));
    }
}

/* This function allows the application to supply a new input function
 * for libpng if standard C streams aren't being used.
 *
 * This function takes as its arguments:
 *
 * png_ptr      - pointer to a png input data structure
 *
 * io_ptr       - pointer to user supplied structure containing info about
 *                the input functions.  May be NULL.
 *
 * read_data_fn - pointer to a new input function that takes as its
 *                arguments a pointer to a png_struct, a pointer to
 *                a location where input data can be stored, and a 32-bit
 *                unsigned int that is the number of bytes to be read.
 *                To exit and output any fatal error messages the new write
 *                function should call png_error(png_ptr, "Error msg").
 *                May be NULL, in which case libpng's default function will
 *                be used.
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

    if read_data_fn.is_some() {
        (*png_ptr).read_data_fn = read_data_fn;
    } else {
        (*png_ptr).read_data_fn =
            Some(png_default_read_data as unsafe extern "C" fn(png_structp, png_bytep, usize));
    }

    /* It is an error to write to a read device */
    if (*png_ptr).write_data_fn.is_some() {
        (*png_ptr).write_data_fn = None;
        png_warning(
            png_ptr,
            cstr!("Can't set both read_data_fn and write_data_fn in the same structure"),
        );
    }

    (*png_ptr).output_flush_fn = None;
}
