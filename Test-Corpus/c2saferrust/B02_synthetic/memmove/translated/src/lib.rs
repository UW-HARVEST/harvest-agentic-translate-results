extern "C" {
    fn memmove(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
}
pub type size_t = usize;
pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn process_buffer(
    mut buffer: *mut uint8_t,
    mut length: size_t,
    mut flags: uint32_t,
    mut param1: ::core::ffi::c_int,
    mut param2: ::core::ffi::c_int,
) -> size_t {
    let mut new_len: size_t = length;
    if buffer.is_null() || length == 0 as size_t {
        return 0 as size_t;
    }
    if flags & 0x1 as uint32_t != 0 {
        let mut offset: ::core::ffi::c_int = param1 % length as ::core::ffi::c_int;
        if offset != 0 as ::core::ffi::c_int {
            rotate_buffer(buffer, length, offset);
        }
    }
    if flags & 0x2 as uint32_t != 0 {
        let mut threshold: uint8_t =
            (if param1 > 0 as ::core::ffi::c_int && param1 <= 255 as ::core::ffi::c_int {
                param1 as uint8_t as ::core::ffi::c_int
            } else {
                3 as ::core::ffi::c_int
            }) as uint8_t;
        new_len = compact_runs(buffer, new_len, threshold);
    }
    if flags & 0x4 as uint32_t != 0 {
        let mut preserve: ::core::ffi::c_int =
            (param2 != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
        new_len = remove_duplicates(buffer, new_len, preserve);
    }
    if flags & 0x8 as uint32_t != 0 && new_len >= 2 as size_t {
        interleave_halves(buffer, new_len);
    }
    if flags & 0x10 as uint32_t != 0 && new_len >= 4 as size_t {
        let mut seg_size: size_t = if param1 > 0 as ::core::ffi::c_int {
            param1 as size_t
        } else {
            4 as size_t
        };
        if seg_size <= new_len {
            reverse_segments(buffer, new_len, seg_size);
        }
    }
    return new_len;
}
unsafe extern "C" fn rotate_buffer(
    mut buf: *mut uint8_t,
    mut len: size_t,
    mut offset: ::core::ffi::c_int,
) {
    if len <= 1 as size_t {
        return;
    }
    offset = offset % len as ::core::ffi::c_int;
    if offset < 0 as ::core::ffi::c_int {
        offset = (offset as ::core::ffi::c_ulong).wrapping_add(len as ::core::ffi::c_ulong)
            as ::core::ffi::c_int as ::core::ffi::c_int;
    }
    if offset == 0 as ::core::ffi::c_int {
        return;
    }
    let mut temp: [uint8_t; 256] = [0; 256];
    let mut chunk: size_t = (if offset < 256 as ::core::ffi::c_int {
        offset
    } else {
        256 as ::core::ffi::c_int
    }) as size_t;
    if (offset as size_t) < len.wrapping_div(2 as size_t) {
        let mut i: size_t = 0;
        i = 0 as size_t;
        while i < offset as size_t {
            let mut copy_len: size_t = if (offset as size_t).wrapping_sub(i) < chunk {
                (offset as size_t).wrapping_sub(i)
            } else {
                chunk
            };
            memmove(
                &raw mut temp as *mut uint8_t as *mut ::core::ffi::c_void,
                buf.offset(i as isize) as *const ::core::ffi::c_void,
                copy_len,
            );
            memmove(
                buf.offset(i as isize) as *mut ::core::ffi::c_void,
                buf.offset(offset as isize) as *const ::core::ffi::c_void,
                len.wrapping_sub(offset as size_t),
            );
            memmove(
                buf.offset(len as isize).offset(-(offset as isize)) as *mut ::core::ffi::c_void,
                &raw mut temp as *mut uint8_t as *const ::core::ffi::c_void,
                copy_len,
            );
            i = (i as ::core::ffi::c_ulong).wrapping_add(chunk as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    } else {
        let mut shift: size_t = len.wrapping_sub(offset as size_t);
        memmove(
            &raw mut temp as *mut uint8_t as *mut ::core::ffi::c_void,
            buf as *const ::core::ffi::c_void,
            shift,
        );
        memmove(
            buf as *mut ::core::ffi::c_void,
            buf.offset(shift as isize) as *const ::core::ffi::c_void,
            offset as size_t,
        );
        memmove(
            buf.offset(offset as isize) as *mut ::core::ffi::c_void,
            &raw mut temp as *mut uint8_t as *const ::core::ffi::c_void,
            shift,
        );
    };
}
unsafe extern "C" fn compact_runs(
    mut buf: *mut uint8_t,
    mut len: size_t,
    mut threshold: uint8_t,
) -> size_t {
    let mut read: size_t = 0 as size_t;
    let mut write: size_t = 0 as size_t;
    while read < len {
        let mut current: uint8_t = *buf.offset(read as isize);
        let mut run_len: size_t = 1 as size_t;
        while read.wrapping_add(run_len) < len
            && *buf.offset(read.wrapping_add(run_len) as isize) as ::core::ffi::c_int
                == current as ::core::ffi::c_int
        {
            run_len = run_len.wrapping_add(1);
        }
        if run_len >= threshold as size_t {
            if run_len > 255 as size_t {
                run_len = 255 as size_t;
            }
            let fresh0 = write;
            write = write.wrapping_add(1);
            *buf.offset(fresh0 as isize) = current;
            let fresh1 = write;
            write = write.wrapping_add(1);
            *buf.offset(fresh1 as isize) = run_len as uint8_t;
            if read.wrapping_add(run_len) < len {
                let mut remaining: size_t = len.wrapping_sub(read.wrapping_add(run_len));
                memmove(
                    buf.offset(write as isize) as *mut ::core::ffi::c_void,
                    buf.offset(read as isize).offset(run_len as isize)
                        as *const ::core::ffi::c_void,
                    remaining,
                );
            }
            len = write.wrapping_add(len.wrapping_sub(read.wrapping_add(run_len)));
            read = write;
        } else {
            if write != read {
                memmove(
                    buf.offset(write as isize) as *mut ::core::ffi::c_void,
                    buf.offset(read as isize) as *const ::core::ffi::c_void,
                    run_len,
                );
            }
            write = (write as ::core::ffi::c_ulong).wrapping_add(run_len as ::core::ffi::c_ulong)
                as size_t as size_t;
            read = (read as ::core::ffi::c_ulong).wrapping_add(run_len as ::core::ffi::c_ulong)
                as size_t as size_t;
        }
    }
    return len;
}
unsafe extern "C" fn remove_duplicates(
    mut buf: *mut uint8_t,
    mut len: size_t,
    mut preserve_order: ::core::ffi::c_int,
) -> size_t {
    if len <= 1 as size_t {
        return len;
    }
    if preserve_order != 0 {
        let mut write: size_t = 1 as size_t;
        let mut i: size_t = 1 as size_t;
        while i < len {
            let mut j: size_t = 0;
            j = 0 as size_t;
            while j < write {
                if *buf.offset(i as isize) as ::core::ffi::c_int
                    == *buf.offset(j as isize) as ::core::ffi::c_int
                {
                    break;
                }
                j = j.wrapping_add(1);
            }
            if j == write {
                if write != i {
                    *buf.offset(write as isize) = *buf.offset(i as isize);
                }
                write = write.wrapping_add(1);
            }
            i = i.wrapping_add(1);
        }
        return write;
    } else {
        let mut seen: [uint8_t; 256] = [
            0 as ::core::ffi::c_int as uint8_t,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        let mut write_0: size_t = 0 as size_t;
        let mut i_0: size_t = 0 as size_t;
        while i_0 < len {
            if seen[*buf.offset(i_0 as isize) as usize] == 0 {
                seen[*buf.offset(i_0 as isize) as usize] = 1 as uint8_t;
                if write_0 != i_0 {
                    let mut temp: uint8_t = *buf.offset(write_0 as isize);
                    *buf.offset(write_0 as isize) = *buf.offset(i_0 as isize);
                    *buf.offset(i_0 as isize) = temp;
                }
                write_0 = write_0.wrapping_add(1);
            }
            i_0 = i_0.wrapping_add(1);
        }
        return write_0;
    };
}
unsafe extern "C" fn interleave_halves(mut buf: *mut uint8_t, mut len: size_t) {
    if len < 2 as size_t {
        return;
    }
    let mut half: size_t = len.wrapping_div(2 as size_t);
    let mut odd: size_t = len.wrapping_rem(2 as size_t);
    let mut temp: [uint8_t; 512] = [0; 512];
    if half <= 256 as size_t {
        memmove(
            &raw mut temp as *mut uint8_t as *mut ::core::ffi::c_void,
            buf as *const ::core::ffi::c_void,
            half,
        );
        let mut i: size_t = 0 as size_t;
        while i < half {
            memmove(
                buf.offset(i.wrapping_mul(2 as size_t) as isize)
                    .offset(1 as ::core::ffi::c_int as isize)
                    as *mut ::core::ffi::c_void,
                buf.offset(half as isize).offset(i as isize) as *const ::core::ffi::c_void,
                1 as size_t,
            );
            *buf.offset(i.wrapping_mul(2 as size_t) as isize) = temp[i as usize];
            i = i.wrapping_add(1);
        }
        if odd != 0 {
            *buf.offset(len.wrapping_sub(1 as size_t) as isize) = *buf.offset(half as isize);
        }
    } else {
        let mut i_0: size_t = 0 as size_t;
        while i_0 < half {
            let mut src: size_t = half.wrapping_add(i_0);
            let mut dst: size_t = i_0.wrapping_mul(2 as size_t).wrapping_add(1 as size_t);
            if dst < src {
                let mut val: uint8_t = *buf.offset(src as isize);
                memmove(
                    buf.offset(dst as isize)
                        .offset(1 as ::core::ffi::c_int as isize)
                        as *mut ::core::ffi::c_void,
                    buf.offset(dst as isize) as *const ::core::ffi::c_void,
                    src.wrapping_sub(dst),
                );
                *buf.offset(dst as isize) = val;
            }
            i_0 = i_0.wrapping_add(1);
        }
    };
}
unsafe extern "C" fn reverse_segments(
    mut buf: *mut uint8_t,
    mut len: size_t,
    mut seg_size: size_t,
) {
    if seg_size <= 1 as size_t || len < seg_size {
        return;
    }
    let mut num_segments: size_t = len.wrapping_div(seg_size);
    let mut remainder: size_t = len.wrapping_rem(seg_size);
    let mut seg: size_t = 0 as size_t;
    while seg < num_segments {
        let mut base: size_t = seg.wrapping_mul(seg_size);
        let mut i: size_t = 0 as size_t;
        while i < seg_size.wrapping_div(2 as size_t) {
            let mut temp: uint8_t = 0;
            let mut left: size_t = base.wrapping_add(i);
            let mut right: size_t = base
                .wrapping_add(seg_size)
                .wrapping_sub(1 as size_t)
                .wrapping_sub(i);
            temp = *buf.offset(left as isize);
            memmove(
                buf.offset(left as isize) as *mut ::core::ffi::c_void,
                buf.offset(right as isize) as *const ::core::ffi::c_void,
                1 as size_t,
            );
            memmove(
                buf.offset(right as isize) as *mut ::core::ffi::c_void,
                &raw mut temp as *const ::core::ffi::c_void,
                1 as size_t,
            );
            i = i.wrapping_add(1);
        }
        seg = seg.wrapping_add(1);
    }
    if remainder > 1 as size_t {
        let mut base_0: size_t = num_segments.wrapping_mul(seg_size);
        let mut i_0: size_t = 0 as size_t;
        while i_0 < remainder.wrapping_div(2 as size_t) {
            let mut temp_0: uint8_t = *buf.offset(base_0.wrapping_add(i_0) as isize);
            *buf.offset(base_0.wrapping_add(i_0) as isize) = *buf.offset(
                base_0
                    .wrapping_add(remainder)
                    .wrapping_sub(1 as size_t)
                    .wrapping_sub(i_0) as isize,
            );
            *buf.offset(
                base_0
                    .wrapping_add(remainder)
                    .wrapping_sub(1 as size_t)
                    .wrapping_sub(i_0) as isize,
            ) = temp_0;
            i_0 = i_0.wrapping_add(1);
        }
    }
}
