extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn perror(__s: *const ::core::ffi::c_char);
    fn atoi(__nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strcat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strdup(__s: *const ::core::ffi::c_char) -> *mut ::core::ffi::c_char;
    fn strtok_r(
        __s: *mut ::core::ffi::c_char,
        __delim: *const ::core::ffi::c_char,
        __save_ptr: *mut *mut ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut ::core::ffi::c_void,
    pub __pad2: *mut ::core::ffi::c_void,
    pub __pad3: *mut ::core::ffi::c_void,
    pub __pad4: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct matrix_t {
    pub matrix: *mut *mut ::core::ffi::c_int,
    pub width: ::core::ffi::c_int,
    pub height: ::core::ffi::c_int,
}
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn allocate_matrix(
    mut width: ::core::ffi::c_int,
    mut height: ::core::ffi::c_int,
) -> *mut matrix_t {
    let mut mat: *mut matrix_t =
        malloc(::core::mem::size_of::<matrix_t>() as size_t) as *mut matrix_t;
    if mat.is_null() {
        perror(
            b"Failed to allocate memory for matrix struct\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return ::core::ptr::null_mut::<matrix_t>();
    }
    (*mat).width = width;
    (*mat).height = height;
    (*mat).matrix = malloc(
        (height as size_t)
            .wrapping_mul(::core::mem::size_of::<*mut ::core::ffi::c_int>() as size_t),
    ) as *mut *mut ::core::ffi::c_int;
    if (*mat).matrix.is_null() {
        perror(
            b"Failed to allocate memory for matrix rows\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        free(mat as *mut ::core::ffi::c_void);
        return ::core::ptr::null_mut::<matrix_t>();
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < height {
        let ref mut fresh0 = *(*mat).matrix.offset(i as isize);
        *fresh0 = malloc(
            (width as size_t).wrapping_mul(::core::mem::size_of::<::core::ffi::c_int>() as size_t),
        ) as *mut ::core::ffi::c_int;
        if (*(*mat).matrix.offset(i as isize)).is_null() {
            perror(
                b"Failed to allocate memory for matrix columns\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while j <= i {
                free(*(*mat).matrix.offset(j as isize) as *mut ::core::ffi::c_void);
                j += 1;
            }
            free((*mat).matrix as *mut ::core::ffi::c_void);
            free(mat as *mut ::core::ffi::c_void);
            return ::core::ptr::null_mut::<matrix_t>();
        }
        i += 1;
    }
    return mat;
}
#[no_mangle]
pub unsafe extern "C" fn free_matrix(mut mat: *mut matrix_t) {
    if mat.is_null() {
        return;
    }
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*mat).height {
        free(*(*mat).matrix.offset(i as isize) as *mut ::core::ffi::c_void);
        i += 1;
    }
    free((*mat).matrix as *mut ::core::ffi::c_void);
    free(mat as *mut ::core::ffi::c_void);
}
#[no_mangle]
pub unsafe extern "C" fn initialize_matrix_from_string(
    mut input: *const ::core::ffi::c_char,
    mut width: ::core::ffi::c_int,
    mut height: ::core::ffi::c_int,
) -> *mut matrix_t {
    let mut mat: *mut matrix_t = allocate_matrix(width, height);
    let mut input_copy: *mut ::core::ffi::c_char = strdup(input);
    if input_copy.is_null() {
        perror(b"Failed to duplicate input string\0" as *const u8 as *const ::core::ffi::c_char);
        free_matrix(mat);
        return ::core::ptr::null_mut::<matrix_t>();
    }
    let mut saveptr_row: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut row_token: *mut ::core::ffi::c_char = strtok_r(
        input_copy,
        b"\n\0" as *const u8 as *const ::core::ffi::c_char,
        &raw mut saveptr_row,
    );
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < height {
        if row_token.is_null() {
            fprintf(
                stderr as *mut FILE,
                b"Insufficient rows in input string.\n\0" as *const u8
                    as *const ::core::ffi::c_char,
            );
            free(input_copy as *mut ::core::ffi::c_void);
            free_matrix(mat);
            return ::core::ptr::null_mut::<matrix_t>();
        }
        let mut saveptr_col: *mut ::core::ffi::c_char =
            ::core::ptr::null_mut::<::core::ffi::c_char>();
        let mut col_token: *mut ::core::ffi::c_char = strtok_r(
            row_token,
            b" \0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut saveptr_col,
        );
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < width {
            if col_token.is_null() {
                fprintf(
                    stderr as *mut FILE,
                    b"Insufficient columns in row %d.\n\0" as *const u8
                        as *const ::core::ffi::c_char,
                    i + 1 as ::core::ffi::c_int,
                );
                free(input_copy as *mut ::core::ffi::c_void);
                free_matrix(mat);
                return ::core::ptr::null_mut::<matrix_t>();
            }
            *(*(*mat).matrix.offset(i as isize)).offset(j as isize) = atoi(col_token);
            col_token = strtok_r(
                ::core::ptr::null_mut::<::core::ffi::c_char>(),
                b" \0" as *const u8 as *const ::core::ffi::c_char,
                &raw mut saveptr_col,
            );
            j += 1;
        }
        row_token = strtok_r(
            ::core::ptr::null_mut::<::core::ffi::c_char>(),
            b"\n\0" as *const u8 as *const ::core::ffi::c_char,
            &raw mut saveptr_row,
        );
        i += 1;
    }
    free(input_copy as *mut ::core::ffi::c_void);
    return mat;
}
#[no_mangle]
pub unsafe extern "C" fn multiply_matrices(
    mut mat_a: *mut matrix_t,
    mut mat_b: *mut matrix_t,
) -> *mut matrix_t {
    if (*mat_a).width != (*mat_b).height {
        fprintf(
            stderr as *mut FILE,
            b"Matrix dimensions do not allow multiplication.\n\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return ::core::ptr::null_mut::<matrix_t>();
    }
    let mut result: *mut matrix_t = allocate_matrix((*mat_b).width, (*mat_a).height);
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*mat_a).height {
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < (*mat_b).width {
            *(*(*result).matrix.offset(i as isize)).offset(j as isize) = 0 as ::core::ffi::c_int;
            let mut k: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while k < (*mat_a).width {
                *(*(*result).matrix.offset(i as isize)).offset(j as isize) +=
                    *(*(*mat_a).matrix.offset(i as isize)).offset(k as isize)
                        * *(*(*mat_b).matrix.offset(k as isize)).offset(j as isize);
                k += 1;
            }
            j += 1;
        }
        i += 1;
    }
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn matrix_to_string(mut mat: *mut matrix_t) -> *mut ::core::ffi::c_char {
    if mat.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Matrix is NULL.\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    let mut buffer_size: ::core::ffi::c_int = (*mat).height
        * ((*mat).width * 10 as ::core::ffi::c_int + (*mat).width)
        + (*mat).height
        + 1 as ::core::ffi::c_int;
    let mut result: *mut ::core::ffi::c_char =
        malloc(buffer_size as size_t) as *mut ::core::ffi::c_char;
    if result.is_null() {
        perror(
            b"Failed to allocate memory for matrix string\0" as *const u8
                as *const ::core::ffi::c_char,
        );
        return ::core::ptr::null_mut::<::core::ffi::c_char>();
    }
    *result.offset(0 as ::core::ffi::c_int as isize) = '\0' as i32 as ::core::ffi::c_char;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < (*mat).height {
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < (*mat).width {
            let mut buffer: [::core::ffi::c_char; 12] = [0; 12];
            snprintf(
                &raw mut buffer as *mut ::core::ffi::c_char,
                ::core::mem::size_of::<[::core::ffi::c_char; 12]>() as size_t,
                b"%d\0" as *const u8 as *const ::core::ffi::c_char,
                *(*(*mat).matrix.offset(i as isize)).offset(j as isize),
            );
            strcat(result, &raw mut buffer as *mut ::core::ffi::c_char);
            if j < (*mat).width - 1 as ::core::ffi::c_int {
                strcat(result, b" \0" as *const u8 as *const ::core::ffi::c_char);
            }
            j += 1;
        }
        strcat(result, b"\n\0" as *const u8 as *const ::core::ffi::c_char);
        i += 1;
    }
    return result;
}
