use std::ffi::{c_char, c_int, c_uint, c_void};
use std::mem::MaybeUninit;
use std::ptr;

const MAX_BUFFER_LENGTH: usize = 256;

const OP_COPY: c_int = 0;
const OP_REVERSE: c_int = 1;
const OP_MERGE: c_int = 2;
const OP_ROTATE: c_int = 5;
const OP_CHECKSUM: c_int = 6;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Buffer {
    pub data: [u8; MAX_BUFFER_LENGTH],
    pub length: usize,
    pub checksum: u32,
}

#[repr(C)]
pub struct BufferArray {
    pub buffers: *mut Buffer,
    pub count: c_int,
    pub capacity: c_int,
}

unsafe extern "C" {
    static mut stderr: *mut c_void;

    #[link_name = "__isoc99_scanf"]
    fn c_scanf(format: *const c_char, ...) -> c_int;
    fn fprintf(stream: *mut c_void, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn putchar(character: c_int) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
}

unsafe fn error(message: &'static [u8]) {
    unsafe {
        fprintf(stderr, message.as_ptr().cast::<c_char>());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calculate_checksum(data: *const u8, length: usize) -> u32 {
    let mut sum = 0_u32;
    for index in 0..length {
        sum = sum.wrapping_shl(3) ^ u32::from(unsafe { *data.add(index) });
    }
    sum
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn validate_buffer(buffer: *const Buffer) -> bool {
    if buffer.is_null() {
        unsafe { error(b"Error: NULL buffer\n\0") };
        return false;
    }

    let buffer = unsafe { &*buffer };
    if buffer.length > MAX_BUFFER_LENGTH {
        unsafe {
            fprintf(
                stderr,
                b"Error: Buffer length %zu exceeds maximum 256\n\0"
                    .as_ptr()
                    .cast::<c_char>(),
                buffer.length,
            );
        }
        return false;
    }

    let expected = unsafe { calculate_checksum(buffer.data.as_ptr(), buffer.length) };
    if buffer.checksum != expected {
        unsafe {
            fprintf(
                stderr,
                b"Warning: Checksum mismatch. Expected %u, got %u\n\0"
                    .as_ptr()
                    .cast::<c_char>(),
                expected as c_uint,
                buffer.checksum as c_uint,
            );
        }
    }
    true
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn init_buffer_array(initial_capacity: c_int) -> *mut BufferArray {
    if initial_capacity <= 0 {
        unsafe {
            fprintf(
                stderr,
                b"Error: Invalid capacity %d\n\0".as_ptr().cast::<c_char>(),
                initial_capacity,
            );
        }
        return ptr::null_mut();
    }

    let array = unsafe { malloc(size_of::<BufferArray>()) }.cast::<BufferArray>();
    if array.is_null() {
        unsafe { error(b"Error: Failed to allocate buffer array\n\0") };
        return ptr::null_mut();
    }

    let storage_size = size_of::<Buffer>().wrapping_mul(initial_capacity as usize);
    let buffers = unsafe { malloc(storage_size) }.cast::<Buffer>();
    if buffers.is_null() {
        unsafe {
            error(b"Error: Failed to allocate buffer storage\n\0");
            free(array.cast::<c_void>());
        }
        return ptr::null_mut();
    }

    unsafe {
        ptr::write(
            array,
            BufferArray {
                buffers,
                count: 0,
                capacity: initial_capacity,
            },
        );
    }
    array
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free_buffer_array(array: *mut BufferArray) {
    if !array.is_null() {
        unsafe {
            free((*array).buffers.cast::<c_void>());
            free(array.cast::<c_void>());
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_copy(source: *const Buffer, destination: *mut Buffer) -> c_int {
    if source.is_null() || destination.is_null() {
        unsafe { error(b"Error: NULL pointer in buffer_copy\n\0") };
        return -1;
    }
    if !unsafe { validate_buffer(source) } {
        return -1;
    }

    let length = unsafe { (*source).length };
    unsafe {
        ptr::copy_nonoverlapping(
            (*source).data.as_ptr(),
            (*destination).data.as_mut_ptr(),
            length,
        );
        (*destination).length = length;
        (*destination).checksum =
            calculate_checksum((*destination).data.as_ptr(), (*destination).length);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_reverse(buffer: *mut Buffer) -> c_int {
    if buffer.is_null() {
        unsafe { error(b"Error: NULL buffer in reverse\n\0") };
        return -1;
    }
    let length = unsafe { (*buffer).length };
    if length == 0 {
        return 0;
    }

    let mut temporary = [0_u8; MAX_BUFFER_LENGTH];
    unsafe {
        ptr::copy_nonoverlapping((*buffer).data.as_ptr(), temporary.as_mut_ptr(), length);
        for index in 0..length {
            (*buffer).data[index] = temporary[length - 1 - index];
        }
        (*buffer).checksum = calculate_checksum((*buffer).data.as_ptr(), length);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_merge(
    source1: *const Buffer,
    source2: *const Buffer,
    destination: *mut Buffer,
) -> c_int {
    if source1.is_null() || source2.is_null() || destination.is_null() {
        unsafe { error(b"Error: NULL pointer in buffer_merge\n\0") };
        return -1;
    }

    let length1 = unsafe { (*source1).length };
    let length2 = unsafe { (*source2).length };
    let merged_length = length1.wrapping_add(length2);
    if merged_length > MAX_BUFFER_LENGTH {
        unsafe {
            fprintf(
                stderr,
                b"Error: Merged length %zu exceeds maximum\n\0"
                    .as_ptr()
                    .cast::<c_char>(),
                merged_length,
            );
        }
        return -1;
    }

    unsafe {
        ptr::copy_nonoverlapping(
            (*source1).data.as_ptr(),
            (*destination).data.as_mut_ptr(),
            length1,
        );
        ptr::copy_nonoverlapping(
            (*source2).data.as_ptr(),
            (*destination).data.as_mut_ptr().add(length1),
            length2,
        );
        (*destination).length = merged_length;
        (*destination).checksum = calculate_checksum((*destination).data.as_ptr(), merged_length);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_split(
    source: *const Buffer,
    split_position: usize,
    destination1: *mut Buffer,
    destination2: *mut Buffer,
) -> c_int {
    if source.is_null() || destination1.is_null() || destination2.is_null() {
        unsafe { error(b"Error: NULL pointer in buffer_split\n\0") };
        return -1;
    }

    let source_length = unsafe { (*source).length };
    if split_position > source_length {
        unsafe {
            fprintf(
                stderr,
                b"Error: Split position %zu exceeds length %zu\n\0"
                    .as_ptr()
                    .cast::<c_char>(),
                split_position,
                source_length,
            );
        }
        return -1;
    }

    unsafe {
        if split_position > 0 {
            ptr::copy_nonoverlapping(
                (*source).data.as_ptr(),
                (*destination1).data.as_mut_ptr(),
                split_position,
            );
        }
        (*destination1).length = split_position;
        (*destination1).checksum =
            calculate_checksum((*destination1).data.as_ptr(), split_position);

        let remaining = source_length - split_position;
        if remaining > 0 {
            ptr::copy_nonoverlapping(
                (*source).data.as_ptr().add(split_position),
                (*destination2).data.as_mut_ptr(),
                remaining,
            );
        }
        (*destination2).length = remaining;
        (*destination2).checksum = calculate_checksum((*destination2).data.as_ptr(), remaining);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_interleave(
    source1: *const Buffer,
    source2: *const Buffer,
    destination: *mut Buffer,
) -> c_int {
    if source1.is_null() || source2.is_null() || destination.is_null() {
        unsafe { error(b"Error: NULL pointer in buffer_interleave\n\0") };
        return -1;
    }

    let length1 = unsafe { (*source1).length };
    let length2 = unsafe { (*source2).length };
    let max_length = length1.max(length2);
    if length1.wrapping_add(length2) > MAX_BUFFER_LENGTH {
        unsafe { error(b"Error: Interleaved length exceeds maximum\n\0") };
        return -1;
    }

    let mut destination_position = 0;
    unsafe {
        for index in 0..max_length {
            if index < length1 {
                ptr::copy_nonoverlapping(
                    (*source1).data.as_ptr().add(index),
                    (*destination).data.as_mut_ptr().add(destination_position),
                    1,
                );
                destination_position += 1;
            }
            if index < length2 {
                ptr::copy_nonoverlapping(
                    (*source2).data.as_ptr().add(index),
                    (*destination).data.as_mut_ptr().add(destination_position),
                    1,
                );
                destination_position += 1;
            }
        }
        (*destination).length = destination_position;
        (*destination).checksum =
            calculate_checksum((*destination).data.as_ptr(), destination_position);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_rotate(buffer: *mut Buffer, mut positions: c_int) -> c_int {
    if buffer.is_null() {
        unsafe { error(b"Error: NULL buffer in rotate\n\0") };
        return -1;
    }

    let length = unsafe { (*buffer).length };
    if length == 0 || positions == 0 {
        return 0;
    }

    positions %= length as c_int;
    if positions < 0 {
        positions = positions.wrapping_add(length as c_int);
    }

    let positions = positions as usize;
    let mut temporary = [0_u8; MAX_BUFFER_LENGTH];
    unsafe {
        ptr::copy_nonoverlapping((*buffer).data.as_ptr(), temporary.as_mut_ptr(), length);
        ptr::copy_nonoverlapping(
            temporary.as_ptr().add(positions),
            (*buffer).data.as_mut_ptr(),
            length - positions,
        );
        ptr::copy_nonoverlapping(
            temporary.as_ptr(),
            (*buffer).data.as_mut_ptr().add(length - positions),
            positions,
        );
        (*buffer).checksum = calculate_checksum((*buffer).data.as_ptr(), length);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_conditional_copy(
    source: *const Buffer,
    destination: *mut Buffer,
    pattern: u8,
    copy_matching: bool,
) -> c_int {
    if source.is_null() || destination.is_null() {
        unsafe { error(b"Error: NULL pointer in conditional_copy\n\0") };
        return -1;
    }

    let mut destination_position = 0;
    unsafe {
        for index in 0..(*source).length {
            let matches = (*source).data[index] == pattern;
            if matches == copy_matching {
                ptr::copy_nonoverlapping(
                    (*source).data.as_ptr().add(index),
                    (*destination).data.as_mut_ptr().add(destination_position),
                    1,
                );
                destination_position += 1;
            }
        }
        (*destination).length = destination_position;
        (*destination).checksum =
            calculate_checksum((*destination).data.as_ptr(), destination_position);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn buffer_copy_strided(
    source: *const Buffer,
    destination: *mut Buffer,
    stride: c_int,
) -> c_int {
    if source.is_null() || destination.is_null() {
        unsafe { error(b"Error: NULL pointer in copy_strided\n\0") };
        return -1;
    }
    if stride <= 0 {
        unsafe {
            fprintf(
                stderr,
                b"Error: Invalid stride %d\n\0".as_ptr().cast::<c_char>(),
                stride,
            );
        }
        return -1;
    }

    let mut destination_position = 0;
    let mut index = 0;
    unsafe {
        while index < (*source).length {
            ptr::copy_nonoverlapping(
                (*source).data.as_ptr().add(index),
                (*destination).data.as_mut_ptr().add(destination_position),
                1,
            );
            destination_position += 1;
            index = index.wrapping_add(stride as usize);
        }
        (*destination).length = destination_position;
        (*destination).checksum =
            calculate_checksum((*destination).data.as_ptr(), destination_position);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer_array(
    array: *mut BufferArray,
    operation: c_int,
    parameter: c_int,
) -> c_int {
    if array.is_null() || unsafe { (*array).count == 0 } {
        unsafe { error(b"Error: Invalid buffer array\n\0") };
        return -1;
    }

    let count = unsafe { (*array).count };
    let buffers = unsafe { (*array).buffers };
    match operation {
        OP_COPY => {
            for index in 1..count {
                if unsafe { buffer_copy(buffers, buffers.add(index as usize)) } != 0 {
                    return -1;
                }
            }
        }
        OP_REVERSE => {
            for index in 0..count {
                if unsafe { buffer_reverse(buffers.add(index as usize)) } != 0 {
                    return -1;
                }
            }
        }
        OP_MERGE => {
            if count < 2 {
                unsafe { error(b"Error: Need at least 2 buffers for merge\n\0") };
                return -1;
            }
            let mut index = 0;
            while index < count - 1 {
                let mut merged = MaybeUninit::<Buffer>::uninit();
                if unsafe {
                    buffer_merge(
                        buffers.add(index as usize),
                        buffers.add((index + 1) as usize),
                        merged.as_mut_ptr(),
                    )
                } != 0
                {
                    return -1;
                }
                unsafe {
                    ptr::copy_nonoverlapping(merged.as_ptr(), buffers.add(index as usize), 1);
                }
                index += 2;
            }
        }
        OP_ROTATE => {
            for index in 0..count {
                if unsafe { buffer_rotate(buffers.add(index as usize), parameter) } != 0 {
                    return -1;
                }
            }
        }
        OP_CHECKSUM => {
            for index in 0..count {
                if !unsafe { validate_buffer(buffers.add(index as usize)) } {
                    return -1;
                }
            }
        }
        _ => {
            unsafe {
                fprintf(
                    stderr,
                    b"Error: Unknown operation %d\n\0".as_ptr().cast::<c_char>(),
                    operation,
                );
            }
            return -1;
        }
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn read_buffer(buffer: *mut Buffer) -> c_int {
    if buffer.is_null() {
        unsafe { error(b"Error: NULL buffer in read_buffer\n\0") };
        return -1;
    }

    let mut length: c_int = 0;
    if unsafe { c_scanf(b"%d\0".as_ptr().cast::<c_char>(), ptr::addr_of_mut!(length)) } != 1 {
        unsafe { error(b"Error: Failed to read buffer length\n\0") };
        return -1;
    }
    if !(0..=MAX_BUFFER_LENGTH as c_int).contains(&length) {
        unsafe {
            fprintf(
                stderr,
                b"Error: Invalid buffer length %d\n\0"
                    .as_ptr()
                    .cast::<c_char>(),
                length,
            );
        }
        return -1;
    }

    unsafe {
        (*buffer).length = length as usize;
        for index in 0..(*buffer).length {
            let mut byte: c_int = 0;
            if c_scanf(b"%d\0".as_ptr().cast::<c_char>(), ptr::addr_of_mut!(byte)) != 1 {
                fprintf(
                    stderr,
                    b"Error: Failed to read byte %zu\n\0"
                        .as_ptr()
                        .cast::<c_char>(),
                    index,
                );
                return -1;
            }
            (*buffer).data[index] = byte as u8;
        }
        (*buffer).checksum = calculate_checksum((*buffer).data.as_ptr(), (*buffer).length);
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn write_buffer(buffer: *const Buffer) {
    if buffer.is_null() {
        unsafe { error(b"Error: NULL buffer in write_buffer\n\0") };
        return;
    }

    unsafe {
        printf(b"%zu\0".as_ptr().cast::<c_char>(), (*buffer).length);
        for index in 0..(*buffer).length {
            printf(
                b" %u\0".as_ptr().cast::<c_char>(),
                (*buffer).data[index] as c_uint,
            );
        }
        putchar(b'\n' as c_int);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    let mut operation: c_int = 0;
    if unsafe {
        c_scanf(
            b"%d\0".as_ptr().cast::<c_char>(),
            ptr::addr_of_mut!(operation),
        )
    } != 1
    {
        unsafe { error(b"Error: Failed to read operation\n\0") };
        return 1;
    }

    let mut buffer_count: c_int = 0;
    if unsafe {
        c_scanf(
            b"%d\0".as_ptr().cast::<c_char>(),
            ptr::addr_of_mut!(buffer_count),
        )
    } != 1
    {
        unsafe { error(b"Error: Failed to read buffer count\n\0") };
        return 1;
    }
    if buffer_count <= 0 || buffer_count > 100 {
        unsafe {
            fprintf(
                stderr,
                b"Error: Invalid buffer count %d\n\0"
                    .as_ptr()
                    .cast::<c_char>(),
                buffer_count,
            );
        }
        return 1;
    }

    let buffers = unsafe { init_buffer_array(buffer_count) };
    if buffers.is_null() {
        return 1;
    }
    for index in 0..buffer_count {
        if unsafe { read_buffer((*buffers).buffers.add(index as usize)) } != 0 {
            unsafe { free_buffer_array(buffers) };
            return 1;
        }
        unsafe { (*buffers).count += 1 };
    }

    let storage = unsafe { (*buffers).buffers };
    let mut result = 0;
    match operation {
        0 => {
            if buffer_count >= 2 {
                let mut temporary = MaybeUninit::<Buffer>::uninit();
                result = unsafe { buffer_copy(storage, temporary.as_mut_ptr()) };
                if result == 0 {
                    unsafe { write_buffer(temporary.as_ptr()) };
                }
            } else {
                unsafe { error(b"Error: Copy needs at least 2 buffers\n\0") };
                result = -1;
            }
        }
        1 => {
            for index in 0..buffer_count {
                result = unsafe { buffer_reverse(storage.add(index as usize)) };
                if result != 0 {
                    break;
                }
                unsafe { write_buffer(storage.add(index as usize)) };
            }
        }
        2 => {
            if buffer_count >= 2 {
                let mut merged = MaybeUninit::<Buffer>::uninit();
                result = unsafe { buffer_merge(storage, storage.add(1), merged.as_mut_ptr()) };
                if result == 0 {
                    unsafe { write_buffer(merged.as_ptr()) };
                }
            } else {
                unsafe { error(b"Error: Merge needs at least 2 buffers\n\0") };
                result = -1;
            }
        }
        3 => {
            let mut split_position: c_int = 0;
            if unsafe {
                c_scanf(
                    b"%d\0".as_ptr().cast::<c_char>(),
                    ptr::addr_of_mut!(split_position),
                )
            } != 1
            {
                unsafe { error(b"Error: Failed to read split position\n\0") };
                result = -1;
            } else {
                let mut part1 = MaybeUninit::<Buffer>::uninit();
                let mut part2 = MaybeUninit::<Buffer>::uninit();
                result = unsafe {
                    buffer_split(
                        storage,
                        split_position as usize,
                        part1.as_mut_ptr(),
                        part2.as_mut_ptr(),
                    )
                };
                if result == 0 {
                    unsafe {
                        write_buffer(part1.as_ptr());
                        write_buffer(part2.as_ptr());
                    }
                }
            }
        }
        4 => {
            if buffer_count >= 2 {
                let mut interleaved = MaybeUninit::<Buffer>::uninit();
                result =
                    unsafe { buffer_interleave(storage, storage.add(1), interleaved.as_mut_ptr()) };
                if result == 0 {
                    unsafe { write_buffer(interleaved.as_ptr()) };
                }
            } else {
                unsafe { error(b"Error: Interleave needs at least 2 buffers\n\0") };
                result = -1;
            }
        }
        5 => {
            let mut positions: c_int = 0;
            if unsafe {
                c_scanf(
                    b"%d\0".as_ptr().cast::<c_char>(),
                    ptr::addr_of_mut!(positions),
                )
            } != 1
            {
                unsafe { error(b"Error: Failed to read rotation amount\n\0") };
                result = -1;
            } else {
                for index in 0..buffer_count {
                    result = unsafe { buffer_rotate(storage.add(index as usize), positions) };
                    if result != 0 {
                        break;
                    }
                    unsafe { write_buffer(storage.add(index as usize)) };
                }
            }
        }
        6 => {
            for index in 0..buffer_count {
                unsafe {
                    printf(
                        b"%u\n\0".as_ptr().cast::<c_char>(),
                        (*storage.add(index as usize)).checksum as c_uint,
                    );
                }
            }
        }
        _ => {
            unsafe {
                fprintf(
                    stderr,
                    b"Error: Unknown operation %d\n\0".as_ptr().cast::<c_char>(),
                    operation,
                );
            }
            result = -1;
        }
    }

    unsafe { free_buffer_array(buffers) };
    if result != 0 {
        1
    } else {
        0
    }
}
