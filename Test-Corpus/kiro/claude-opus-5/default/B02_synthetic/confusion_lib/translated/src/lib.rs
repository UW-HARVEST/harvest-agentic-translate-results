// Rust translation of c_src/src/lib.c
//
// Original C code: Copyright 2025 MIT Lincoln Laboratory (MIT license, see
// c_src/src/lib.c for the full notice).
//
// The translation is deliberately faithful: it reproduces the original
// behaviour byte-for-byte, including quirks such as C's truncating `%`
// operator (so `param4 % 4` can be negative and fall through the `switch`
// without matching any `case`), signed `char`, x86-64 float-to-int
// conversion semantics, and the exact ordering of the `printf` calls.
//
// All formatted output goes through the C library's `printf`/`snprintf` so
// that the emitted bytes and the stdout buffering behaviour are identical to
// the original.

use std::ffi::{c_char, c_float, c_int, c_uint, c_void};

// ---------------------------------------------------------------------------
// libc bindings
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn snprintf(buf: *mut c_char, size: usize, fmt: *const c_char, ...) -> c_int;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
    fn memchr(s: *const c_void, c: c_int, n: usize) -> *mut c_void;
}

// Format strings, expanded exactly as the C preprocessor would expand
// DEBUG_VAR / LOG_OPERATION.
const FMT_DEBUG_PARAM1: &[u8] = b"Debug: param1 = %d\n\0";
const FMT_DEBUG_PARAM2: &[u8] = b"Debug: param2 = %d\n\0";
const FMT_DEBUG_PARAM3: &[u8] = b"Debug: param3 = %d\n\0";
const FMT_DEBUG_PARAM4: &[u8] = b"Debug: param4 = %d\n\0";
const FMT_DEBUG_COUNTER: &[u8] = b"Debug: state->flags.counter = %d\n\0";
const FMT_LOG_MEMCHR: &[u8] = b"Operation: memchr_found with value %d\n\0";
const FMT_ERR_STATE_ALLOC: &[u8] = b"Error: Failed to allocate memory for state\n\0";
const FMT_ERR_BUFFER_ALLOC: &[u8] = b"Error: Failed to allocate buffer\n\0";
const FMT_ERR_NULL_PROCESS: &[u8] = b"Error: Null pointer in process_buffer\n\0";
const FMT_STATE_BUFFER: &[u8] = b"State:%d:Mode:%d\0";
const FMT_BIT_FIELDS: &[u8] = b"Bit fields - flag1:%d flag2:%d flag3:%d mode:%d\n\0";
const FMT_SET_AS_INT: &[u8] = b"Set as int: %d\n\0";
const FMT_READ_AS_FLOAT: &[u8] = b"Read as float: %f\n\0";
const FMT_READ_AS_UINT: &[u8] = b"Read as uint: %u\n\0";
const FMT_READ_AS_BYTES: &[u8] = b"Read as bytes: [%d, %d, %d, %d]\n\0";
const FMT_FINAL_RESULT: &[u8] = b"Final result: %d\n\0";

#[inline]
fn cstr(bytes: &'static [u8]) -> *const c_char {
    bytes.as_ptr() as *const c_char
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Mirrors the C bit-field struct:
///
/// ```c
/// typedef struct {
///     unsigned int flag1 : 1;
///     unsigned int flag2 : 1;
///     unsigned int flag3 : 1;
///     unsigned int counter : 5;
///     unsigned int mode : 3;
///     unsigned int status : 5;
///     unsigned int reserved : 16;
/// } PackedFlags;
/// ```
///
/// GCC/Clang on little-endian targets pack these least-significant-bit first
/// into a single 4-byte storage unit, which is what the bit offsets below
/// encode.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct PackedFlags {
    bits: u32,
}

impl PackedFlags {
    const OFF_FLAG1: u32 = 0;
    const OFF_FLAG2: u32 = 1;
    const OFF_FLAG3: u32 = 2;
    const OFF_COUNTER: u32 = 3;
    const OFF_MODE: u32 = 8;
    const OFF_STATUS: u32 = 11;
    const OFF_RESERVED: u32 = 16;

    #[inline]
    fn get(&self, offset: u32, width: u32) -> c_uint {
        let mask: u32 = if width >= 32 { u32::MAX } else { (1u32 << width) - 1 };
        (self.bits >> offset) & mask
    }

    #[inline]
    fn set(&mut self, offset: u32, width: u32, value: c_uint) {
        let mask: u32 = if width >= 32 { u32::MAX } else { (1u32 << width) - 1 };
        self.bits = (self.bits & !(mask << offset)) | ((value & mask) << offset);
    }

    #[inline]
    fn flag1(&self) -> c_uint {
        self.get(Self::OFF_FLAG1, 1)
    }
    #[inline]
    fn set_flag1(&mut self, v: c_uint) {
        self.set(Self::OFF_FLAG1, 1, v)
    }
    #[inline]
    fn flag2(&self) -> c_uint {
        self.get(Self::OFF_FLAG2, 1)
    }
    #[inline]
    fn set_flag2(&mut self, v: c_uint) {
        self.set(Self::OFF_FLAG2, 1, v)
    }
    #[inline]
    fn flag3(&self) -> c_uint {
        self.get(Self::OFF_FLAG3, 1)
    }
    #[inline]
    fn set_flag3(&mut self, v: c_uint) {
        self.set(Self::OFF_FLAG3, 1, v)
    }
    #[inline]
    fn counter(&self) -> c_uint {
        self.get(Self::OFF_COUNTER, 5)
    }
    #[inline]
    fn set_counter(&mut self, v: c_uint) {
        self.set(Self::OFF_COUNTER, 5, v)
    }
    #[inline]
    fn mode(&self) -> c_uint {
        self.get(Self::OFF_MODE, 3)
    }
    #[inline]
    fn set_mode(&mut self, v: c_uint) {
        self.set(Self::OFF_MODE, 3, v)
    }
    #[inline]
    fn set_status(&mut self, v: c_uint) {
        self.set(Self::OFF_STATUS, 5, v)
    }
    #[inline]
    fn set_reserved(&mut self, v: c_uint) {
        self.set(Self::OFF_RESERVED, 16, v)
    }
}

/// Mirrors the C union; all members are 4 bytes wide, so a single `u32`
/// storage unit reproduces both the layout and the aliasing behaviour.
#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct TypeConfusion {
    storage: u32,
}

impl TypeConfusion {
    #[inline]
    fn int_val(&self) -> c_int {
        self.storage as c_int
    }
    #[inline]
    fn set_int_val(&mut self, v: c_int) {
        self.storage = v as u32;
    }
    #[inline]
    fn float_val(&self) -> c_float {
        f32::from_bits(self.storage)
    }
    #[inline]
    fn uint_val(&self) -> c_uint {
        self.storage
    }
    /// `char` is signed on the x86-64 SysV ABI, and the bytes are stored
    /// little-endian.
    #[inline]
    fn bytes(&self) -> [i8; 4] {
        let b = self.storage.to_le_bytes();
        [b[0] as i8, b[1] as i8, b[2] as i8, b[3] as i8]
    }
}

#[repr(C)]
pub struct ProcessState {
    flags: PackedFlags,
    data: TypeConfusion,
    buffer: *mut c_char,
    capacity: c_int,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Reproduces the x86-64 `cvttss2si` behaviour used by GCC/Clang for a
/// `(int)` cast of a `float`: truncate towards zero, and yield `INT_MIN`
/// ("integer indefinite") for NaN or out-of-range values. Rust's `as i32`
/// saturates instead, so it cannot be used directly here.
#[inline]
fn float_to_int_trunc(v: c_float) -> c_int {
    if v.is_nan() {
        return c_int::MIN;
    }
    let t = v.trunc();
    // 2147483648.0 is exactly representable as f32; INT_MAX is not.
    if t >= -2147483648.0f32 && t < 2147483648.0f32 {
        t as c_int
    } else {
        c_int::MIN
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn create_state(initial_val: c_int, capacity: c_int) -> *mut ProcessState {
    let state = malloc(core::mem::size_of::<ProcessState>()) as *mut ProcessState;

    if state.is_null() {
        printf(cstr(FMT_ERR_STATE_ALLOC));
        return core::ptr::null_mut();
    }

    let flags = &mut (*state).flags;
    // The C code assigns each bit field individually over the freshly
    // malloc'd (indeterminate) storage; `reserved` is the last write and
    // covers the remaining bits, so the whole unit ends up defined.
    flags.bits = 0;
    flags.set_flag1(1);
    flags.set_flag2(0);
    flags.set_flag3(1);
    flags.set_counter(0);
    flags.set_mode(3);
    flags.set_status(15);
    flags.set_reserved(0);

    (*state).data.set_int_val(initial_val);

    (*state).capacity = capacity;
    // `malloc(capacity)` converts the (possibly negative) int to size_t.
    (*state).buffer = malloc(capacity as usize) as *mut c_char;

    if (*state).buffer.is_null() {
        printf(cstr(FMT_ERR_BUFFER_ALLOC));
        free(state as *mut c_void);
        return core::ptr::null_mut();
    }

    snprintf(
        (*state).buffer,
        capacity as usize,
        cstr(FMT_STATE_BUFFER),
        initial_val,
        (*state).flags.mode(),
    );

    state
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn destroy_state(state: *mut ProcessState) {
    if !state.is_null() {
        if !(*state).buffer.is_null() {
            free((*state).buffer as *mut c_void);
        }
        free(state as *mut c_void);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn process_buffer(state: *mut ProcessState, target: c_char) -> c_int {
    if state.is_null() || (*state).buffer.is_null() {
        printf(cstr(FMT_ERR_NULL_PROCESS));
        return -1;
    }

    let mut count: c_int = 0;
    let mut ptr: *const c_char = (*state).buffer;
    let mut remaining: usize = strlen((*state).buffer);

    while remaining > 0 {
        // memchr takes the character as an int and compares it as an
        // unsigned char.
        let found = memchr(ptr as *const c_void, target as c_int, remaining) as *const c_char;

        if found.is_null() {
            break;
        }

        count += 1;
        printf(cstr(FMT_LOG_MEMCHR), count);

        remaining -= (found.offset_from(ptr) + 1) as usize;
        ptr = found.add(1);
    }

    count
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn update_flags(state: *mut ProcessState, param: c_int) {
    if state.is_null() {
        return;
    }

    let flags = &mut (*state).flags;

    // 5-bit counter
    flags.set_counter((flags.counter().wrapping_add(1)) & 0x1F);
    flags.set_flag1((param & 1) as c_uint);
    flags.set_flag2(((param & 2) >> 1) as c_uint);
    flags.set_flag3(((param & 4) >> 2) as c_uint);
    // Arithmetic right shift, matching C's implementation-defined signed
    // shift on GCC/Clang.
    flags.set_mode(((param >> 3) & 0x7) as c_uint);

    printf(cstr(FMT_DEBUG_COUNTER), flags.counter() as c_int);
    printf(
        cstr(FMT_BIT_FIELDS),
        flags.flag1() as c_int,
        flags.flag2() as c_int,
        flags.flag3() as c_int,
        flags.mode() as c_int,
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confuse_types(state: *mut ProcessState, operation: c_int) -> c_int {
    if state.is_null() {
        return 0;
    }

    let mut result: c_int = 0;

    // A plain `match` would need an arm for every other value; the C switch
    // simply falls through when `operation` matches nothing (which happens
    // for the negative values C's `%` can produce).
    match operation {
        0 => {
            (*state).data.set_int_val(1078530011);
            printf(cstr(FMT_SET_AS_INT), (*state).data.int_val());
        }

        1 => {
            let f = (*state).data.float_val();
            // Promoted to double for the variadic call, exactly as in C.
            printf(cstr(FMT_READ_AS_FLOAT), f as f64);
            result = float_to_int_trunc(f * 100.0f32);
        }

        2 => {
            printf(cstr(FMT_READ_AS_UINT), (*state).data.uint_val());
            result = ((*state).data.uint_val() & 0xFF) as c_int;
        }

        3 => {
            let b = (*state).data.bytes();
            printf(
                cstr(FMT_READ_AS_BYTES),
                b[0] as c_int,
                b[1] as c_int,
                b[2] as c_int,
                b[3] as c_int,
            );
            result = (b[0] as c_int).wrapping_add(b[1] as c_int);
        }

        _ => {}
    }

    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn confusion(
    param1: c_int,
    param2: c_int,
    param3: c_int,
    param4: c_int,
) -> c_int {
    printf(cstr(FMT_DEBUG_PARAM1), param1);
    printf(cstr(FMT_DEBUG_PARAM2), param2);
    printf(cstr(FMT_DEBUG_PARAM3), param3);
    printf(cstr(FMT_DEBUG_PARAM4), param4);

    let mut result: c_int = 0;

    let state = create_state(param1, 128);

    if state.is_null() {
        return -1;
    }

    update_flags(state, param2);

    // C's `%` truncates towards zero, so a negative param3 yields a negative
    // remainder and therefore a search character below '0'.
    let search_char = (b'0' as c_int + (param3 % 10)) as c_char;
    let found_count = process_buffer(state, search_char);
    result = result.wrapping_add(found_count.wrapping_mul(10));

    let confusion_result = confuse_types(state, param4 % 4);
    result = result.wrapping_add(confusion_result);

    result = result.wrapping_add(((*state).flags.counter() as c_int).wrapping_mul(5));
    result = result.wrapping_add(((*state).flags.mode() as c_int).wrapping_mul(3));

    printf(cstr(FMT_FINAL_RESULT), result);

    destroy_state(state);

    result
}
