//! Translation of `c_src/src/matrix.c` (public surface in `c_src/include/matrix.h`).
//!
//! Note that `allocate_matrix` is *not* declared in the header, but it is not
//! `static` either, so the C shared object exports it. It is reproduced here as
//! a public `extern "C"` symbol for that reason.

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_void};
use core::ptr;

use crate::cffi::{
    atoi, fprintf, free, malloc, perror, snprintf, stderr_stream, strcat, strdup, strtok_r,
};

/// ```c
/// typedef struct {
///     int** matrix;
///     int width;
///     int height;
/// } matrix_t;
/// ```
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut c_int,
    pub width: c_int,
    pub height: c_int,
}

/// Read `mat->matrix[i][j]`.
#[inline]
unsafe fn cell(mat: *const matrix_t, i: c_int, j: c_int) -> *mut c_int {
    (*(*mat).matrix.offset(i as isize)).offset(j as isize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn allocate_matrix(width: c_int, height: c_int) -> *mut matrix_t {
    // matrix_t* mat = malloc(sizeof(matrix_t));
    let mat = malloc(core::mem::size_of::<matrix_t>()) as *mut matrix_t;
    if mat.is_null() {
        perror(c"Failed to allocate memory for matrix struct".as_ptr());
        return ptr::null_mut();
    }

    (*mat).width = width;
    (*mat).height = height;

    // mat->matrix = malloc(height * sizeof(int*));
    //
    // `height` is an `int`; in C it is converted to `size_t` (sign extended)
    // before being multiplied, and the product wraps. A negative `height`
    // therefore turns into an enormous request that `malloc` rejects.
    let row_bytes =
        (height as isize as usize).wrapping_mul(core::mem::size_of::<*mut c_int>());
    (*mat).matrix = malloc(row_bytes) as *mut *mut c_int;
    if (*mat).matrix.is_null() {
        perror(c"Failed to allocate memory for matrix rows".as_ptr());
        free(mat as *mut c_void);
        return ptr::null_mut();
    }

    let mut i: c_int = 0;
    while i < height {
        let col_bytes = (width as isize as usize).wrapping_mul(core::mem::size_of::<c_int>());
        let row = malloc(col_bytes) as *mut c_int;
        *(*mat).matrix.offset(i as isize) = row;
        if row.is_null() {
            perror(c"Failed to allocate memory for matrix columns".as_ptr());
            // The C loop runs `j <= i`, i.e. it also frees the row whose
            // allocation just failed (a no-op `free(NULL)`).
            let mut j: c_int = 0;
            while j <= i {
                free(*(*mat).matrix.offset(j as isize) as *mut c_void);
                j = j.wrapping_add(1);
            }
            free((*mat).matrix as *mut c_void);
            free(mat as *mut c_void);
            return ptr::null_mut();
        }
        i = i.wrapping_add(1);
    }

    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_matrix(mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }

    let mut i: c_int = 0;
    while i < (*mat).height {
        free(*(*mat).matrix.offset(i as isize) as *mut c_void);
        i = i.wrapping_add(1);
    }
    free((*mat).matrix as *mut c_void);
    free(mat as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn initialize_matrix_from_string(
    input: *const c_char,
    width: c_int,
    height: c_int,
) -> *mut matrix_t {
    // The C code deliberately does not check `mat` for NULL here; preserved.
    let mat = allocate_matrix(width, height);

    let input_copy = strdup(input);
    if input_copy.is_null() {
        perror(c"Failed to duplicate input string".as_ptr());
        free_matrix(mat);
        return ptr::null_mut();
    }

    let mut saveptr_row: *mut c_char = ptr::null_mut();
    let mut row_token = strtok_r(input_copy, c"\n".as_ptr(), &mut saveptr_row);

    let mut i: c_int = 0;
    while i < height {
        if row_token.is_null() {
            fprintf(
                stderr_stream(),
                c"Insufficient rows in input string.\n".as_ptr(),
            );
            free(input_copy as *mut c_void);
            free_matrix(mat);
            return ptr::null_mut();
        }

        let mut saveptr_col: *mut c_char = ptr::null_mut();
        let mut col_token = strtok_r(row_token, c" ".as_ptr(), &mut saveptr_col);

        let mut j: c_int = 0;
        while j < width {
            if col_token.is_null() {
                fprintf(
                    stderr_stream(),
                    c"Insufficient columns in row %d.\n".as_ptr(),
                    i.wrapping_add(1),
                );
                free(input_copy as *mut c_void);
                free_matrix(mat);
                return ptr::null_mut();
            }
            *cell(mat, i, j) = atoi(col_token);
            col_token = strtok_r(ptr::null_mut(), c" ".as_ptr(), &mut saveptr_col);
            j = j.wrapping_add(1);
        }

        row_token = strtok_r(ptr::null_mut(), c"\n".as_ptr(), &mut saveptr_row);
        i = i.wrapping_add(1);
    }

    free(input_copy as *mut c_void);
    mat
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn multiply_matrices(
    mat_a: *mut matrix_t,
    mat_b: *mut matrix_t,
) -> *mut matrix_t {
    if (*mat_a).width != (*mat_b).height {
        fprintf(
            stderr_stream(),
            c"Matrix dimensions do not allow multiplication.\n".as_ptr(),
        );
        return ptr::null_mut();
    }

    // As in the C, the result of `allocate_matrix` is not NULL-checked.
    let result = allocate_matrix((*mat_b).width, (*mat_a).height);

    let mut i: c_int = 0;
    while i < (*mat_a).height {
        let mut j: c_int = 0;
        while j < (*mat_b).width {
            let out = cell(result, i, j);
            *out = 0;
            let mut k: c_int = 0;
            while k < (*mat_a).width {
                let a = *cell(mat_a, i, k);
                let b = *cell(mat_b, k, j);
                *out = (*out).wrapping_add(a.wrapping_mul(b));
                k = k.wrapping_add(1);
            }
            j = j.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn matrix_to_string(mat: *mut matrix_t) -> *mut c_char {
    if mat.is_null() {
        fprintf(stderr_stream(), c"Error: Matrix is NULL.\n".as_ptr());
        return ptr::null_mut();
    }

    let width = (*mat).width;
    let height = (*mat).height;

    // int buffer_size = mat->height * (mat->width * 10 + mat->width)
    //                   + mat->height + 1;
    let buffer_size: c_int = height
        .wrapping_mul(width.wrapping_mul(10).wrapping_add(width))
        .wrapping_add(height)
        .wrapping_add(1);

    // `buffer_size` is an `int` widened to `size_t`; a negative value becomes a
    // huge request that fails, exactly as in C.
    let result = malloc(buffer_size as isize as usize) as *mut c_char;
    if result.is_null() {
        perror(c"Failed to allocate memory for matrix string".as_ptr());
        return ptr::null_mut();
    }

    *result.add(0) = 0;

    // NOTE: the C sizing formula reserves `width * 11` bytes per row for the
    // digits but only ONE extra byte per row for the `width - 1` separating
    // spaces and the trailing newline. Any row wide enough to hold long numbers
    // therefore runs off the end of the allocation and corrupts the heap.
    //
    // That is a genuine bug in the original and it is reproduced verbatim
    // rather than papered over: this translation calls the very same
    // `snprintf`/`strcat` against a block from the very same `malloc`, in the
    // same allocation order, so an overrun scribbles over exactly the same
    // bytes and glibc reacts exactly as it does for the C build. No
    // intermediate Rust allocations are made here, precisely so the heap layout
    // is not perturbed.
    let mut i: c_int = 0;
    while i < height {
        let mut j: c_int = 0;
        while j < width {
            let mut buffer = [0 as c_char; 12];
            snprintf(
                buffer.as_mut_ptr(),
                buffer.len(),
                c"%d".as_ptr(),
                *cell(mat, i, j),
            );
            strcat(result, buffer.as_ptr());

            if j < width.wrapping_sub(1) {
                strcat(result, c" ".as_ptr());
            }
            j = j.wrapping_add(1);
        }
        strcat(result, c"\n".as_ptr());
        i = i.wrapping_add(1);
    }

    result
}
