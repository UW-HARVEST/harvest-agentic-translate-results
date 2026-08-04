#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(raw_ref_op)]
#[allow(unused_imports)]
use ::driver;
extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn printf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn scanf(__format: *const libc::c_char, ...) -> libc::c_int;
    fn malloc(__size: size_t) -> *mut libc::c_void;
    fn free(__ptr: *mut libc::c_void);
    fn memcpy(
        __dest: *mut libc::c_void,
        __src: *const libc::c_void,
        __n: size_t,
    ) -> *mut libc::c_void;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type __off_t = libc::c_long;
pub type __off64_t = libc::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: libc::c_int,
    pub _IO_read_ptr: *mut libc::c_char,
    pub _IO_read_end: *mut libc::c_char,
    pub _IO_read_base: *mut libc::c_char,
    pub _IO_write_base: *mut libc::c_char,
    pub _IO_write_ptr: *mut libc::c_char,
    pub _IO_write_end: *mut libc::c_char,
    pub _IO_buf_base: *mut libc::c_char,
    pub _IO_buf_end: *mut libc::c_char,
    pub _IO_save_base: *mut libc::c_char,
    pub _IO_backup_base: *mut libc::c_char,
    pub _IO_save_end: *mut libc::c_char,
    pub _markers: *mut _IO_marker,
    pub _chain: *mut _IO_FILE,
    pub _fileno: libc::c_int,
    pub _flags2: libc::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: libc::c_ushort,
    pub _vtable_offset: libc::c_schar,
    pub _shortbuf: [libc::c_char; 1],
    pub _lock: *mut libc::c_void,
    pub _offset: __off64_t,
    pub __pad1: *mut libc::c_void,
    pub __pad2: *mut libc::c_void,
    pub __pad3: *mut libc::c_void,
    pub __pad4: *mut libc::c_void,
    pub __pad5: size_t,
    pub _mode: libc::c_int,
    pub _unused2: [libc::c_char; 20],
}
pub type _IO_lock_t = ();
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: libc::c_int,
}
pub type FILE = _IO_FILE;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct buffer_t {
    pub data: [uint8_t; 256],
    pub length: size_t,
    pub checksum: uint32_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct buffer_array_t {
    pub buffers: *mut buffer_t,
    pub count: libc::c_int,
    pub capacity: libc::c_int,
}
pub type operation_t = libc::c_uint;
pub const OP_CHECKSUM: operation_t = 6;
pub const OP_ROTATE: operation_t = 5;
pub const OP_INTERLEAVE: operation_t = 4;
pub const OP_SPLIT: operation_t = 3;
pub const OP_MERGE: operation_t = 2;
pub const OP_REVERSE: operation_t = 1;
pub const OP_COPY: operation_t = 0;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const true_0: libc::c_int = 1 as libc::c_int;
pub const false_0: libc::c_int = 0 as libc::c_int;
#[no_mangle]
pub unsafe extern "C" fn calculate_checksum(
    mut data: *const uint8_t,
    mut length: size_t,
) -> uint32_t {
    let mut sum: uint32_t = 0 as uint32_t;
    let mut i: size_t = 0 as size_t;
    while i < length {
        sum = sum << 3 as libc::c_int ^ *data.offset(i as isize) as uint32_t;
        i = i.wrapping_add(1);
    }
    return sum;
}
#[no_mangle]
pub unsafe extern "C" fn validate_buffer(mut buf: *const buffer_t) -> bool {
    if buf.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL buffer\n\0" as *const u8 as *const libc::c_char,
        );
        return false_0 != 0;
    }
    if (*buf).length > 256 as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: Buffer length %zu exceeds maximum 256\n\0" as *const u8
                as *const libc::c_char,
            (*buf).length,
        );
        return false_0 != 0;
    }
    let mut expected: uint32_t =
        calculate_checksum(&raw const (*buf).data as *const uint8_t, (*buf).length);
    if (*buf).checksum != expected {
        fprintf(
            stderr as *mut FILE,
            b"Warning: Checksum mismatch. Expected %u, got %u\n\0" as *const u8
                as *const libc::c_char,
            expected,
            (*buf).checksum,
        );
    }
    return true_0 != 0;
}
#[no_mangle]
pub unsafe extern "C" fn init_buffer_array(
    mut initial_capacity: libc::c_int,
) -> *mut buffer_array_t {
    if initial_capacity <= 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: Invalid capacity %d\n\0" as *const u8 as *const libc::c_char,
            initial_capacity,
        );
        return std::ptr::null_mut::<buffer_array_t>();
    }
    let mut arr: *mut buffer_array_t =
        malloc(std::mem::size_of::<buffer_array_t>() as size_t) as *mut buffer_array_t;
    if arr.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to allocate buffer array\n\0" as *const u8
                as *const libc::c_char,
        );
        return std::ptr::null_mut::<buffer_array_t>();
    }
    (*arr).buffers = malloc(
        (std::mem::size_of::<buffer_t>() as size_t).wrapping_mul(initial_capacity as size_t),
    ) as *mut buffer_t;
    if (*arr).buffers.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to allocate buffer storage\n\0" as *const u8
                as *const libc::c_char,
        );
        free(arr as *mut libc::c_void);
        return std::ptr::null_mut::<buffer_array_t>();
    }
    (*arr).count = 0 as libc::c_int;
    (*arr).capacity = initial_capacity;
    return arr;
}
#[no_mangle]
pub unsafe extern "C" fn free_buffer_array(mut arr: *mut buffer_array_t) {
    if !arr.is_null() {
        free((*arr).buffers as *mut libc::c_void);
        free(arr as *mut libc::c_void);
    }
}
#[no_mangle]
pub unsafe extern "C" fn buffer_copy(
    mut src: *const buffer_t,
    mut dst: *mut buffer_t,
) -> libc::c_int {
    if src.is_null() || dst.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL pointer in buffer_copy\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if !validate_buffer(src) {
        return -(1 as libc::c_int);
    }
    memcpy(
        &raw mut (*dst).data as *mut uint8_t as *mut libc::c_void,
        &raw const (*src).data as *const uint8_t as *const libc::c_void,
        (*src).length,
    );
    (*dst).length = (*src).length;
    (*dst).checksum = calculate_checksum(&raw mut (*dst).data as *mut uint8_t, (*dst).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_reverse(mut buf: *mut buffer_t) -> libc::c_int {
    if buf.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL buffer in reverse\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if (*buf).length == 0 as size_t {
        return 0 as libc::c_int;
    }
    let mut temp: [uint8_t; 256] = [0; 256];
    memcpy(
        &raw mut temp as *mut uint8_t as *mut libc::c_void,
        &raw mut (*buf).data as *mut uint8_t as *const libc::c_void,
        (*buf).length,
    );
    let mut i: size_t = 0 as size_t;
    while i < (*buf).length {
        (*buf).data[i as usize] =
            temp[(*buf).length.wrapping_sub(1 as size_t).wrapping_sub(i) as usize];
        i = i.wrapping_add(1);
    }
    (*buf).checksum = calculate_checksum(&raw mut (*buf).data as *mut uint8_t, (*buf).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_merge(
    mut src1: *const buffer_t,
    mut src2: *const buffer_t,
    mut dst: *mut buffer_t,
) -> libc::c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL pointer in buffer_merge\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if (*src1).length.wrapping_add((*src2).length) > 256 as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: Merged length %zu exceeds maximum\n\0" as *const u8
                as *const libc::c_char,
            (*src1).length.wrapping_add((*src2).length),
        );
        return -(1 as libc::c_int);
    }
    memcpy(
        &raw mut (*dst).data as *mut uint8_t as *mut libc::c_void,
        &raw const (*src1).data as *const uint8_t as *const libc::c_void,
        (*src1).length,
    );
    memcpy(
        (&raw mut (*dst).data as *mut uint8_t).offset((*src1).length as isize)
            as *mut libc::c_void,
        &raw const (*src2).data as *const uint8_t as *const libc::c_void,
        (*src2).length,
    );
    (*dst).length = (*src1).length.wrapping_add((*src2).length);
    (*dst).checksum = calculate_checksum(&raw mut (*dst).data as *mut uint8_t, (*dst).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_split(
    mut src: *const buffer_t,
    mut split_pos: size_t,
    mut dst1: *mut buffer_t,
    mut dst2: *mut buffer_t,
) -> libc::c_int {
    if src.is_null() || dst1.is_null() || dst2.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL pointer in buffer_split\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if split_pos > (*src).length {
        fprintf(
            stderr as *mut FILE,
            b"Error: Split position %zu exceeds length %zu\n\0" as *const u8
                as *const libc::c_char,
            split_pos,
            (*src).length,
        );
        return -(1 as libc::c_int);
    }
    if split_pos > 0 as size_t {
        memcpy(
            &raw mut (*dst1).data as *mut uint8_t as *mut libc::c_void,
            &raw const (*src).data as *const uint8_t as *const libc::c_void,
            split_pos,
        );
    }
    (*dst1).length = split_pos;
    (*dst1).checksum = calculate_checksum(&raw mut (*dst1).data as *mut uint8_t, (*dst1).length);
    let mut remaining: size_t = (*src).length.wrapping_sub(split_pos);
    if remaining > 0 as size_t {
        memcpy(
            &raw mut (*dst2).data as *mut uint8_t as *mut libc::c_void,
            (&raw const (*src).data as *const uint8_t).offset(split_pos as isize)
                as *const libc::c_void,
            remaining,
        );
    }
    (*dst2).length = remaining;
    (*dst2).checksum = calculate_checksum(&raw mut (*dst2).data as *mut uint8_t, (*dst2).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_interleave(
    mut src1: *const buffer_t,
    mut src2: *const buffer_t,
    mut dst: *mut buffer_t,
) -> libc::c_int {
    if src1.is_null() || src2.is_null() || dst.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL pointer in buffer_interleave\n\0" as *const u8
                as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    let mut max_len: size_t = if (*src1).length > (*src2).length {
        (*src1).length
    } else {
        (*src2).length
    };
    if (*src1).length.wrapping_add((*src2).length) > 256 as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: Interleaved length exceeds maximum\n\0" as *const u8
                as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    let mut dst_pos: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < max_len {
        if i < (*src1).length {
            let fresh0 = dst_pos;
            dst_pos = dst_pos.wrapping_add(1);
            memcpy(
                (&raw mut (*dst).data as *mut uint8_t).offset(fresh0 as isize) as *mut uint8_t
                    as *mut libc::c_void,
                (&raw const (*src1).data as *const uint8_t).offset(i as isize) as *const uint8_t
                    as *const libc::c_void,
                1 as size_t,
            );
        }
        if i < (*src2).length {
            let fresh1 = dst_pos;
            dst_pos = dst_pos.wrapping_add(1);
            memcpy(
                (&raw mut (*dst).data as *mut uint8_t).offset(fresh1 as isize) as *mut uint8_t
                    as *mut libc::c_void,
                (&raw const (*src2).data as *const uint8_t).offset(i as isize) as *const uint8_t
                    as *const libc::c_void,
                1 as size_t,
            );
        }
        i = i.wrapping_add(1);
    }
    (*dst).length = dst_pos;
    (*dst).checksum = calculate_checksum(&raw mut (*dst).data as *mut uint8_t, (*dst).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_rotate(
    mut buf: *mut buffer_t,
    mut positions: libc::c_int,
) -> libc::c_int {
    if buf.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL buffer in rotate\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if (*buf).length == 0 as size_t || positions == 0 as libc::c_int {
        return 0 as libc::c_int;
    }
    positions = positions % (*buf).length as libc::c_int;
    if positions < 0 as libc::c_int {
        positions = (positions as libc::c_ulong)
            .wrapping_add((*buf).length as libc::c_ulong)
            as libc::c_int as libc::c_int;
    }
    let mut temp: [uint8_t; 256] = [0; 256];
    memcpy(
        &raw mut temp as *mut uint8_t as *mut libc::c_void,
        &raw mut (*buf).data as *mut uint8_t as *const libc::c_void,
        (*buf).length,
    );
    memcpy(
        &raw mut (*buf).data as *mut uint8_t as *mut libc::c_void,
        (&raw mut temp as *mut uint8_t).offset(positions as isize) as *const libc::c_void,
        (*buf).length.wrapping_sub(positions as size_t),
    );
    memcpy(
        (&raw mut (*buf).data as *mut uint8_t)
            .offset((*buf).length.wrapping_sub(positions as size_t) as isize)
            as *mut libc::c_void,
        &raw mut temp as *mut uint8_t as *const libc::c_void,
        positions as size_t,
    );
    (*buf).checksum = calculate_checksum(&raw mut (*buf).data as *mut uint8_t, (*buf).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_conditional_copy(
    mut src: *const buffer_t,
    mut dst: *mut buffer_t,
    mut pattern: uint8_t,
    mut copy_matching: bool,
) -> libc::c_int {
    if src.is_null() || dst.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL pointer in conditional_copy\n\0" as *const u8
                as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    let mut dst_pos: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < (*src).length {
        let mut matches: bool =
            (*src).data[i as usize] as libc::c_int == pattern as libc::c_int;
        if matches as libc::c_int == copy_matching as libc::c_int {
            let fresh2 = dst_pos;
            dst_pos = dst_pos.wrapping_add(1);
            memcpy(
                (&raw mut (*dst).data as *mut uint8_t).offset(fresh2 as isize) as *mut uint8_t
                    as *mut libc::c_void,
                (&raw const (*src).data as *const uint8_t).offset(i as isize) as *const uint8_t
                    as *const libc::c_void,
                1 as size_t,
            );
        }
        i = i.wrapping_add(1);
    }
    (*dst).length = dst_pos;
    (*dst).checksum = calculate_checksum(&raw mut (*dst).data as *mut uint8_t, (*dst).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn buffer_copy_strided(
    mut src: *const buffer_t,
    mut dst: *mut buffer_t,
    mut stride: libc::c_int,
) -> libc::c_int {
    if src.is_null() || dst.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL pointer in copy_strided\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if stride <= 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: Invalid stride %d\n\0" as *const u8 as *const libc::c_char,
            stride,
        );
        return -(1 as libc::c_int);
    }
    let mut dst_pos: size_t = 0 as size_t;
    let mut i: size_t = 0 as size_t;
    while i < (*src).length {
        let fresh3 = dst_pos;
        dst_pos = dst_pos.wrapping_add(1);
        memcpy(
            (&raw mut (*dst).data as *mut uint8_t).offset(fresh3 as isize) as *mut uint8_t
                as *mut libc::c_void,
            (&raw const (*src).data as *const uint8_t).offset(i as isize) as *const uint8_t
                as *const libc::c_void,
            1 as size_t,
        );
        i = (i as libc::c_ulong).wrapping_add(stride as libc::c_ulong) as size_t
            as size_t;
    }
    (*dst).length = dst_pos;
    (*dst).checksum = calculate_checksum(&raw mut (*dst).data as *mut uint8_t, (*dst).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn process_buffer_array(
    mut arr: *mut buffer_array_t,
    mut op: operation_t,
    mut param: libc::c_int,
) -> libc::c_int {
    if arr.is_null() || (*arr).count == 0 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: Invalid buffer array\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    match op as libc::c_uint {
        0 => {
            let mut i: libc::c_int = 1 as libc::c_int;
            while i < (*arr).count {
                if buffer_copy(
                    (*arr).buffers.offset(0 as libc::c_int as isize) as *mut buffer_t,
                    (*arr).buffers.offset(i as isize) as *mut buffer_t,
                ) != 0 as libc::c_int
                {
                    return -(1 as libc::c_int);
                }
                i += 1;
            }
        }
        1 => {
            let mut i_0: libc::c_int = 0 as libc::c_int;
            while i_0 < (*arr).count {
                if buffer_reverse((*arr).buffers.offset(i_0 as isize) as *mut buffer_t)
                    != 0 as libc::c_int
                {
                    return -(1 as libc::c_int);
                }
                i_0 += 1;
            }
        }
        2 => {
            if (*arr).count < 2 as libc::c_int {
                fprintf(
                    stderr as *mut FILE,
                    b"Error: Need at least 2 buffers for merge\n\0" as *const u8
                        as *const libc::c_char,
                );
                return -(1 as libc::c_int);
            }
            let mut i_1: libc::c_int = 0 as libc::c_int;
            while i_1 < (*arr).count - 1 as libc::c_int {
                let mut merged: buffer_t = buffer_t {
                    data: [0; 256],
                    length: 0,
                    checksum: 0,
                };
                if buffer_merge(
                    (*arr).buffers.offset(i_1 as isize) as *mut buffer_t,
                    (*arr)
                        .buffers
                        .offset((i_1 + 1 as libc::c_int) as isize)
                        as *mut buffer_t,
                    &raw mut merged,
                ) != 0 as libc::c_int
                {
                    return -(1 as libc::c_int);
                }
                memcpy(
                    (*arr).buffers.offset(i_1 as isize) as *mut buffer_t
                        as *mut libc::c_void,
                    &raw mut merged as *const libc::c_void,
                    std::mem::size_of::<buffer_t>() as size_t,
                );
                i_1 += 2 as libc::c_int;
            }
        }
        5 => {
            let mut i_2: libc::c_int = 0 as libc::c_int;
            while i_2 < (*arr).count {
                if buffer_rotate((*arr).buffers.offset(i_2 as isize) as *mut buffer_t, param)
                    != 0 as libc::c_int
                {
                    return -(1 as libc::c_int);
                }
                i_2 += 1;
            }
        }
        6 => {
            let mut i_3: libc::c_int = 0 as libc::c_int;
            while i_3 < (*arr).count {
                if !validate_buffer((*arr).buffers.offset(i_3 as isize) as *mut buffer_t) {
                    return -(1 as libc::c_int);
                }
                i_3 += 1;
            }
        }
        _ => {
            fprintf(
                stderr as *mut FILE,
                b"Error: Unknown operation %d\n\0" as *const u8 as *const libc::c_char,
                op as libc::c_uint,
            );
            return -(1 as libc::c_int);
        }
    }
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn read_buffer(mut buf: *mut buffer_t) -> libc::c_int {
    if buf.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL buffer in read_buffer\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    let mut length: libc::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut length,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to read buffer length\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    if length < 0 as libc::c_int || length > 256 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: Invalid buffer length %d\n\0" as *const u8 as *const libc::c_char,
            length,
        );
        return -(1 as libc::c_int);
    }
    (*buf).length = length as size_t;
    let mut i: size_t = 0 as size_t;
    while i < (*buf).length {
        let mut byte: libc::c_int = 0;
        if scanf(
            b"%d\0" as *const u8 as *const libc::c_char,
            &raw mut byte,
        ) != 1 as libc::c_int
        {
            fprintf(
                stderr as *mut FILE,
                b"Error: Failed to read byte %zu\n\0" as *const u8 as *const libc::c_char,
                i,
            );
            return -(1 as libc::c_int);
        }
        (*buf).data[i as usize] = byte as uint8_t;
        i = i.wrapping_add(1);
    }
    (*buf).checksum = calculate_checksum(&raw mut (*buf).data as *mut uint8_t, (*buf).length);
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn write_buffer(mut buf: *const buffer_t) {
    if buf.is_null() {
        fprintf(
            stderr as *mut FILE,
            b"Error: NULL buffer in write_buffer\n\0" as *const u8 as *const libc::c_char,
        );
        return;
    }
    printf(
        b"%zu\0" as *const u8 as *const libc::c_char,
        (*buf).length,
    );
    let mut i: size_t = 0 as size_t;
    while i < (*buf).length {
        printf(
            b" %u\0" as *const u8 as *const libc::c_char,
            (*buf).data[i as usize] as libc::c_int,
        );
        i = i.wrapping_add(1);
    }
    printf(b"\n\0" as *const u8 as *const libc::c_char);
}
unsafe fn main_0(
    mut argc: libc::c_int,
    mut argv: *mut *mut libc::c_char,
) -> libc::c_int {
    let mut operation: libc::c_int = 0;
    let mut buffer_count: libc::c_int = 0;
    if scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut operation,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to read operation\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    if scanf(
        b"%d\0" as *const u8 as *const libc::c_char,
        &raw mut buffer_count,
    ) != 1 as libc::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to read buffer count\n\0" as *const u8 as *const libc::c_char,
        );
        return 1 as libc::c_int;
    }
    if buffer_count <= 0 as libc::c_int || buffer_count > 100 as libc::c_int {
        fprintf(
            stderr as *mut FILE,
            b"Error: Invalid buffer count %d\n\0" as *const u8 as *const libc::c_char,
            buffer_count,
        );
        return 1 as libc::c_int;
    }
    let mut buffers: *mut buffer_array_t = init_buffer_array(buffer_count);
    if buffers.is_null() {
        return 1 as libc::c_int;
    }
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < buffer_count {
        if read_buffer((*buffers).buffers.offset(i as isize) as *mut buffer_t)
            != 0 as libc::c_int
        {
            free_buffer_array(buffers);
            return 1 as libc::c_int;
        }
        (*buffers).count += 1;
        i += 1;
    }
    let mut result: libc::c_int = 0 as libc::c_int;
    match operation {
        0 => {
            if buffer_count >= 2 as libc::c_int {
                let mut temp: buffer_t = buffer_t {
                    data: [0; 256],
                    length: 0,
                    checksum: 0,
                };
                result = buffer_copy(
                    (*buffers).buffers.offset(0 as libc::c_int as isize) as *mut buffer_t,
                    &raw mut temp,
                );
                if result == 0 as libc::c_int {
                    write_buffer(&raw mut temp);
                }
            } else {
                fprintf(
                    stderr as *mut FILE,
                    b"Error: Copy needs at least 2 buffers\n\0" as *const u8
                        as *const libc::c_char,
                );
                result = -(1 as libc::c_int);
            }
        }
        1 => {
            let mut i_0: libc::c_int = 0 as libc::c_int;
            while i_0 < buffer_count {
                result = buffer_reverse((*buffers).buffers.offset(i_0 as isize) as *mut buffer_t);
                if result != 0 as libc::c_int {
                    break;
                }
                write_buffer((*buffers).buffers.offset(i_0 as isize) as *mut buffer_t);
                i_0 += 1;
            }
        }
        2 => {
            if buffer_count >= 2 as libc::c_int {
                let mut merged: buffer_t = buffer_t {
                    data: [0; 256],
                    length: 0,
                    checksum: 0,
                };
                result = buffer_merge(
                    (*buffers).buffers.offset(0 as libc::c_int as isize) as *mut buffer_t,
                    (*buffers).buffers.offset(1 as libc::c_int as isize) as *mut buffer_t,
                    &raw mut merged,
                );
                if result == 0 as libc::c_int {
                    write_buffer(&raw mut merged);
                }
            } else {
                fprintf(
                    stderr as *mut FILE,
                    b"Error: Merge needs at least 2 buffers\n\0" as *const u8
                        as *const libc::c_char,
                );
                result = -(1 as libc::c_int);
            }
        }
        3 => {
            if buffer_count >= 1 as libc::c_int {
                let mut split_pos: libc::c_int = 0;
                if scanf(
                    b"%d\0" as *const u8 as *const libc::c_char,
                    &raw mut split_pos,
                ) != 1 as libc::c_int
                {
                    fprintf(
                        stderr as *mut FILE,
                        b"Error: Failed to read split position\n\0" as *const u8
                            as *const libc::c_char,
                    );
                    result = -(1 as libc::c_int);
                } else {
                    let mut part1: buffer_t = buffer_t {
                        data: [0; 256],
                        length: 0,
                        checksum: 0,
                    };
                    let mut part2: buffer_t = buffer_t {
                        data: [0; 256],
                        length: 0,
                        checksum: 0,
                    };
                    result = buffer_split(
                        (*buffers).buffers.offset(0 as libc::c_int as isize)
                            as *mut buffer_t,
                        split_pos as size_t,
                        &raw mut part1,
                        &raw mut part2,
                    );
                    if result == 0 as libc::c_int {
                        write_buffer(&raw mut part1);
                        write_buffer(&raw mut part2);
                    }
                }
            }
        }
        4 => {
            if buffer_count >= 2 as libc::c_int {
                let mut interleaved: buffer_t = buffer_t {
                    data: [0; 256],
                    length: 0,
                    checksum: 0,
                };
                result = buffer_interleave(
                    (*buffers).buffers.offset(0 as libc::c_int as isize) as *mut buffer_t,
                    (*buffers).buffers.offset(1 as libc::c_int as isize) as *mut buffer_t,
                    &raw mut interleaved,
                );
                if result == 0 as libc::c_int {
                    write_buffer(&raw mut interleaved);
                }
            } else {
                fprintf(
                    stderr as *mut FILE,
                    b"Error: Interleave needs at least 2 buffers\n\0" as *const u8
                        as *const libc::c_char,
                );
                result = -(1 as libc::c_int);
            }
        }
        5 => {
            let mut positions: libc::c_int = 0;
            if scanf(
                b"%d\0" as *const u8 as *const libc::c_char,
                &raw mut positions,
            ) != 1 as libc::c_int
            {
                fprintf(
                    stderr as *mut FILE,
                    b"Error: Failed to read rotation amount\n\0" as *const u8
                        as *const libc::c_char,
                );
                result = -(1 as libc::c_int);
            } else {
                let mut i_1: libc::c_int = 0 as libc::c_int;
                while i_1 < buffer_count {
                    result = buffer_rotate(
                        (*buffers).buffers.offset(i_1 as isize) as *mut buffer_t,
                        positions,
                    );
                    if result != 0 as libc::c_int {
                        break;
                    }
                    write_buffer((*buffers).buffers.offset(i_1 as isize) as *mut buffer_t);
                    i_1 += 1;
                }
            }
        }
        6 => {
            let mut i_2: libc::c_int = 0 as libc::c_int;
            while i_2 < buffer_count {
                printf(
                    b"%u\n\0" as *const u8 as *const libc::c_char,
                    (*(*buffers).buffers.offset(i_2 as isize)).checksum,
                );
                i_2 += 1;
            }
        }
        _ => {
            fprintf(
                stderr as *mut FILE,
                b"Error: Unknown operation %d\n\0" as *const u8 as *const libc::c_char,
                operation,
            );
            result = -(1 as libc::c_int);
        }
    }
    free_buffer_array(buffers);
    return if result != 0 as libc::c_int {
        1 as libc::c_int
    } else {
        0 as libc::c_int
    };
}
pub fn main() {
    let mut args_strings: Vec<Vec<u8>> = ::std::env::args()
        .map(|arg| {
            ::std::ffi::CString::new(arg)
                .expect("Failed to convert argument into CString.")
                .into_bytes_with_nul()
        })
        .collect();
    let mut args_ptrs: Vec<*mut libc::c_char> = args_strings
        .iter_mut()
        .map(|arg| arg.as_mut_ptr() as *mut libc::c_char)
        .chain(::core::iter::once(std::ptr::null_mut()))
        .collect();
    unsafe {
        ::std::process::exit(main_0(
            (args_ptrs.len() - 1) as libc::c_int,
            args_ptrs.as_mut_ptr() as *mut *mut libc::c_char,
        ) as i32)
    }
}
