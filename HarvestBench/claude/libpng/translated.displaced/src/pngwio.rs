// pngwio.c - functions for data output
//
// Copyright (c) 2018-2025 Cosmin Truta
// Copyright (c) 1998-2002,2004,2006-2014,2016,2018 Glenn Randers-Pehrson
// Copyright (c) 1996-1997 Andreas Dilger
// Copyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.
//
// This code is released under the libpng license.
// For conditions of distribution and use, see the disclaimer
// and license in png.h
//
// This file provides a location for all output.  Users who need
// special handling are expected to write functions that have the same
// arguments as these and perform similar functions, but that possibly
// use different output methods.  Note that you shouldn't change these
// functions, but rather write replacement functions and then change
// them at run time with png_set_write_fn(...).

use crate::*;

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
        ((*png_ptr).write_data_fn.unwrap())(png_ptr, data as png_bytep, length);
    } else {
        png_error(png_ptr as png_const_structrp, cstr!("Call to NULL write function"));
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
        png_error(png_ptr as png_const_structrp, cstr!("Write Error"));
    }
}

/* This function is called to output any data pending writing (normally
 * to disk).  After png_flush is called, there should be no data pending
 * writing in any buffers.
 */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_flush(png_ptr: png_structrp) {
    if (*png_ptr).output_flush_fn.is_some() {
        ((*png_ptr).output_flush_fn.unwrap())(png_ptr);
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
 *
 * This function takes as its arguments:
 * png_ptr       - pointer to a png output data structure
 * io_ptr        - pointer to user supplied structure containing info about
 *                 the output functions.  May be NULL.
 * write_data_fn - pointer to a new output function that takes as its
 *                 arguments a pointer to a png_struct, a pointer to
 *                 data to be written, and a 32-bit unsigned int that is
 *                 the number of bytes to be written.  The new write
 *                 function should call png_error(png_ptr, "Error msg")
 *                 to exit and output any fatal error messages.  May be
 *                 NULL, in which case libpng's default function will
 *                 be used.
 * flush_data_fn - pointer to a new flush function that takes as its
 *                 arguments a pointer to a png_struct.  After a call to
 *                 the flush function, there should be no data in any buffers
 *                 or pending transmission.  If the output method doesn't do
 *                 any buffering of output, a function prototype must still be
 *                 supplied although it doesn't have to do anything.  If
 *                 PNG_WRITE_FLUSH_SUPPORTED is not defined at libpng compile
 *                 time, output_flush_fn will be ignored, although it must be
 *                 supplied for compatibility.  May be NULL, in which case
 *                 libpng's default function will be used, if
 *                 PNG_WRITE_FLUSH_SUPPORTED is defined.  This is not
 *                 a good idea if io_ptr does not point to a standard
 *                 *FILE structure.
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
            png_ptr as png_const_structrp,
            cstr!("Can't set both read_data_fn and write_data_fn in the same structure"),
        );
    }
}
