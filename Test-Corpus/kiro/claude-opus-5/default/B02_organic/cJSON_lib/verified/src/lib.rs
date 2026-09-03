//! Rust translation of cJSON 1.7.19 (c_src/cJSON.c + c_src/test.c).
//!
//! This crate reproduces the complete public ABI of the C shared library and is
//! intended to be byte-for-byte behaviour compatible, including quirks and bugs
//! of the original implementation.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
/* Several assignments are dead in Rust's view but are kept so that the control
 * flow matches the C original statement-for-statement. */
#![allow(unused_assignments)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_double, c_float, c_int, c_void};
use core::ptr;

mod driver;

/* ------------------------------------------------------------------------- */
/* libc bindings                                                             */
/* ------------------------------------------------------------------------- */

pub(crate) mod libc {
    use core::ffi::{c_char, c_double, c_int, c_void};

    #[repr(C)]
    pub struct lconv {
        pub decimal_point: *const c_char,
        pub thousands_sep: *const c_char,
        pub grouping: *const c_char,
        pub int_curr_symbol: *const c_char,
        pub currency_symbol: *const c_char,
        pub mon_decimal_point: *const c_char,
        pub mon_thousands_sep: *const c_char,
        pub mon_grouping: *const c_char,
        pub positive_sign: *const c_char,
        pub negative_sign: *const c_char,
        pub int_frac_digits: c_char,
        pub frac_digits: c_char,
        pub p_cs_precedes: c_char,
        pub p_sep_by_space: c_char,
        pub n_cs_precedes: c_char,
        pub n_sep_by_space: c_char,
        pub p_sign_posn: c_char,
        pub n_sign_posn: c_char,
        pub int_p_cs_precedes: c_char,
        pub int_p_sep_by_space: c_char,
        pub int_n_cs_precedes: c_char,
        pub int_n_sep_by_space: c_char,
        pub int_p_sign_posn: c_char,
        pub int_n_sign_posn: c_char,
    }

    extern "C" {
        pub fn malloc(size: usize) -> *mut c_void;
        pub fn free(ptr: *mut c_void);
        pub fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
        pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
        pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
        pub fn strlen(s: *const c_char) -> usize;
        pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
        pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
        pub fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
        pub fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> c_double;
        pub fn tolower(c: c_int) -> c_int;
        pub fn localeconv() -> *mut lconv;
        pub fn exit(status: c_int) -> !;
        pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
        pub fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
        pub fn printf(fmt: *const c_char, ...) -> c_int;
    }
}

/* ------------------------------------------------------------------------- */
/* public types                                                              */
/* ------------------------------------------------------------------------- */

pub type cJSON_bool = c_int;

pub const cJSON_Invalid: c_int = 0;
pub const cJSON_False: c_int = 1 << 0;
pub const cJSON_True: c_int = 1 << 1;
pub const cJSON_NULL: c_int = 1 << 2;
pub const cJSON_Number: c_int = 1 << 3;
pub const cJSON_String: c_int = 1 << 4;
pub const cJSON_Array: c_int = 1 << 5;
pub const cJSON_Object: c_int = 1 << 6;
pub const cJSON_Raw: c_int = 1 << 7;

pub const cJSON_IsReference: c_int = 256;
pub const cJSON_StringIsConst: c_int = 512;

pub const CJSON_VERSION_MAJOR: c_int = 1;
pub const CJSON_VERSION_MINOR: c_int = 7;
pub const CJSON_VERSION_PATCH: c_int = 19;

pub const CJSON_NESTING_LIMIT: usize = 1000;
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

pub type cJSON_malloc_fn = unsafe extern "C" fn(usize) -> *mut c_void;
pub type cJSON_free_fn = unsafe extern "C" fn(*mut c_void);
pub type cJSON_realloc_fn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<cJSON_malloc_fn>,
    pub free_fn: Option<cJSON_free_fn>,
}

/* ------------------------------------------------------------------------- */
/* internal types / globals                                                  */
/* ------------------------------------------------------------------------- */

const TRUE: cJSON_bool = 1;
const FALSE: cJSON_bool = 0;

#[derive(Clone, Copy)]
struct Error {
    json: *const u8,
    position: usize,
}

static mut GLOBAL_ERROR: Error = Error {
    json: ptr::null(),
    position: 0,
};

#[derive(Clone, Copy)]
pub(crate) struct InternalHooks {
    allocate: Option<cJSON_malloc_fn>,
    deallocate: Option<cJSON_free_fn>,
    reallocate: Option<cJSON_realloc_fn>,
}

const NULL_HOOKS: InternalHooks = InternalHooks {
    allocate: None,
    deallocate: None,
    reallocate: None,
};

static mut GLOBAL_HOOKS: InternalHooks = InternalHooks {
    allocate: Some(libc::malloc),
    deallocate: Some(libc::free),
    reallocate: Some(libc::realloc),
};

#[inline]
unsafe fn global_hooks() -> InternalHooks {
    GLOBAL_HOOKS
}

impl InternalHooks {
    #[inline]
    unsafe fn allocate(&self, size: usize) -> *mut c_void {
        (self.allocate.expect("cJSON: NULL allocate hook"))(size)
    }
    #[inline]
    unsafe fn deallocate(&self, pointer: *mut c_void) {
        (self.deallocate.expect("cJSON: NULL deallocate hook"))(pointer)
    }
    #[inline]
    unsafe fn reallocate(&self, pointer: *mut c_void, size: usize) -> *mut c_void {
        (self.reallocate.expect("cJSON: NULL reallocate hook"))(pointer, size)
    }
}

/* ------------------------------------------------------------------------- */
/* error pointer / value accessors                                           */
/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    GLOBAL_ERROR.json.wrapping_add(GLOBAL_ERROR.position) as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if cJSON_IsString(item) == 0 {
        return ptr::null_mut();
    }

    (*item).valuestring
}

/* `cJSON.c` is compiled with `-std=c89`, so glibc's C99 `NAN` from <math.h> is
 * *not* visible and cJSON's own fallback is used:
 *
 *     #ifndef NAN
 *     #define NAN 0.0/0.0
 *     #endif
 *
 * GCC constant-folds `0.0/0.0` with x86 SSE semantics, which yields the
 * "QNaN indefinite" value -- i.e. the sign bit is *set*. `f64::NAN` in Rust is
 * the positive quiet NaN, so the raw bits would differ. */
const C_NAN: c_double = f64::from_bits(0xFFF8_0000_0000_0000u64);

/* Truncating double -> int conversion with the semantics the C compiler
 * actually emits on x86-64 (`cvttsd2si`): NaN and out-of-range values become
 * the "integer indefinite" value `INT_MIN`. Rust's `as` cast saturates
 * instead, and maps NaN to 0, so it cannot be used directly.
 *
 * cJSON guards `>= INT_MAX` and `<= (double)INT_MIN` before converting, so in
 * practice only NaN reaches the out-of-range path -- but the helper is exact
 * for every input. */
#[inline]
fn c_double_to_int(d: c_double) -> c_int {
    if c_isnan(d) || !(d > -2147483649.0 && d < 2147483648.0) {
        c_int::MIN
    } else {
        d as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    if cJSON_IsNumber(item) == 0 {
        return C_NAN;
    }

    (*item).valuedouble
}

/* `static char version[15]` filled by sprintf("%i.%i.%i", 1, 7, 19). */
static VERSION: [u8; 15] = [
    b'1', b'.', b'7', b'.', b'1', b'9', 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/* Case insensitive string comparison, doesn't consider two NULL pointers equal though */
unsafe fn case_insensitive_strcmp(mut string1: *const u8, mut string2: *const u8) -> c_int {
    if string1.is_null() || string2.is_null() {
        return 1;
    }

    if string1 == string2 {
        return 0;
    }

    while libc::tolower(*string1 as c_int) == libc::tolower(*string2 as c_int) {
        if *string1 == b'\0' {
            return 0;
        }
        string1 = string1.add(1);
        string2 = string2.add(1);
    }

    libc::tolower(*string1 as c_int) - libc::tolower(*string2 as c_int)
}

unsafe fn cJSON_strdup(string: *const u8, hooks: &InternalHooks) -> *mut u8 {
    if string.is_null() {
        return ptr::null_mut();
    }

    let length = libc::strlen(string as *const c_char) + 1;
    let copy = hooks.allocate(length) as *mut u8;
    if copy.is_null() {
        return ptr::null_mut();
    }
    libc::memcpy(copy as *mut c_void, string as *const c_void, length);

    copy
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        /* Reset hooks */
        GLOBAL_HOOKS.allocate = Some(libc::malloc);
        GLOBAL_HOOKS.deallocate = Some(libc::free);
        GLOBAL_HOOKS.reallocate = Some(libc::realloc);
        return;
    }

    GLOBAL_HOOKS.allocate = Some(libc::malloc);
    if (*hooks).malloc_fn.is_some() {
        GLOBAL_HOOKS.allocate = (*hooks).malloc_fn;
    }

    GLOBAL_HOOKS.deallocate = Some(libc::free);
    if (*hooks).free_fn.is_some() {
        GLOBAL_HOOKS.deallocate = (*hooks).free_fn;
    }

    /* use realloc only if both free and malloc are used */
    GLOBAL_HOOKS.reallocate = None;
    if fn_eq_alloc(GLOBAL_HOOKS.allocate, libc::malloc)
        && fn_eq_free(GLOBAL_HOOKS.deallocate, libc::free)
    {
        GLOBAL_HOOKS.reallocate = Some(libc::realloc);
    }
}

#[inline]
fn fn_eq_alloc(a: Option<cJSON_malloc_fn>, b: cJSON_malloc_fn) -> bool {
    match a {
        Some(f) => f as usize == b as usize,
        None => false,
    }
}

#[inline]
fn fn_eq_free(a: Option<cJSON_free_fn>, b: cJSON_free_fn) -> bool {
    match a {
        Some(f) => f as usize == b as usize,
        None => false,
    }
}

/* Internal constructor. */
unsafe fn cJSON_New_Item(hooks: &InternalHooks) -> *mut cJSON {
    let node = hooks.allocate(core::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        libc::memset(node as *mut c_void, b'\0' as c_int, core::mem::size_of::<cJSON>());
    }

    node
}

/* Delete a cJSON structure. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    let mut next: *mut cJSON;
    while !item.is_null() {
        next = (*item).next;
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).valuestring.is_null() {
            global_hooks().deallocate((*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
            global_hooks().deallocate((*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        global_hooks().deallocate(item as *mut c_void);
        item = next;
    }
}

/* get the decimal point character of the current locale */
unsafe fn get_decimal_point() -> u8 {
    /* ENABLE_LOCALES is enabled by default in the upstream CMake build. */
    let lconv = libc::localeconv();
    *(*lconv).decimal_point as u8
}

/* ------------------------------------------------------------------------- */
/* parse buffer                                                              */
/* ------------------------------------------------------------------------- */

#[derive(Clone, Copy)]
struct ParseBuffer {
    content: *const u8,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: InternalHooks,
}

/* check if the given size is left to read in a given parse buffer (starting with 1) */
#[inline]
unsafe fn can_read(buffer: *const ParseBuffer, size: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset.wrapping_add(size) <= (*buffer).length)
}

/* check if the buffer can be accessed at the given index (starting with 0) */
#[inline]
unsafe fn can_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset.wrapping_add(index) < (*buffer).length)
}

#[inline]
unsafe fn cannot_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    !can_access_at_index(buffer, index)
}

/* get a pointer to the buffer at the position */
#[inline]
unsafe fn buffer_at_offset(buffer: *const ParseBuffer) -> *const u8 {
    (*buffer).content.add((*buffer).offset)
}

/* Parse the input text to generate a number, and populate the result into item. */
unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let number: c_double;
    let mut after_end: *mut c_char = ptr::null_mut();
    let number_c_string: *mut u8;
    let decimal_point = get_decimal_point();
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point = FALSE;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return FALSE;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    while can_access_at_index(input_buffer, i) {
        match *buffer_at_offset(input_buffer).add(i) {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'
            | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = TRUE;
            }
            _ => break,
        }
        i += 1;
    }

    /* malloc for temporary buffer, add 1 for '\0' */
    number_c_string = (*input_buffer).hooks.allocate(number_string_length + 1) as *mut u8;
    if number_c_string.is_null() {
        return FALSE; /* allocation failure */
    }

    libc::memcpy(
        number_c_string as *mut c_void,
        buffer_at_offset(input_buffer) as *const c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = b'\0';

    if has_decimal_point != FALSE {
        i = 0;
        while i < number_string_length {
            if *number_c_string.add(i) == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                *number_c_string.add(i) = decimal_point;
            }
            i += 1;
        }
    }

    number = libc::strtod(number_c_string as *const c_char, &mut after_end);
    if number_c_string as *const c_char == after_end as *const c_char {
        /* free the temporary buffer */
        (*input_buffer)
            .hooks
            .deallocate(number_c_string as *mut c_void);
        return FALSE; /* parse_error */
    }

    (*item).valuedouble = number;

    /* use saturation in case of overflow */
    if number >= c_int::MAX as c_double {
        (*item).valueint = c_int::MAX;
    } else if number <= c_int::MIN as c_double {
        (*item).valueint = c_int::MIN;
    } else {
        (*item).valueint = c_double_to_int(number);
    }

    (*item).type_ = cJSON_Number;

    (*input_buffer).offset += (after_end as usize).wrapping_sub(number_c_string as usize);
    /* free the temporary buffer */
    (*input_buffer)
        .hooks
        .deallocate(number_c_string as *mut c_void);
    TRUE
}

/* don't ask me, but the original cJSON_SetNumberValue returns an integer or double */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if number >= c_int::MAX as c_double {
        (*object).valueint = c_int::MAX;
    } else if number <= c_int::MIN as c_double {
        (*object).valueint = c_int::MIN;
    } else {
        (*object).valueint = c_double_to_int(number);
    }

    (*object).valuedouble = number;
    (*object).valuedouble
}

/* Note: when passing a NULL valuestring, cJSON_SetValuestring treats this as an error and return NULL */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    let copy: *mut c_char;
    let v1_len: usize;
    let v2_len: usize;
    /* if object's type is not cJSON_String or is cJSON_IsReference, it should not set valuestring */
    if object.is_null()
        || ((*object).type_ & cJSON_String) == 0
        || ((*object).type_ & cJSON_IsReference) != 0
    {
        return ptr::null_mut();
    }
    /* return NULL if the object is corrupted or valuestring is NULL */
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }

    v1_len = libc::strlen(valuestring);
    v2_len = libc::strlen((*object).valuestring);

    if v1_len <= v2_len {
        /* strcpy does not handle overlapping string: [X1, X2] [Y1, Y2] => X2 < Y1 or Y2 < X1 */
        if !(((valuestring as usize + v1_len) < (*object).valuestring as usize)
            || (((*object).valuestring as usize + v2_len) < valuestring as usize))
        {
            return ptr::null_mut();
        }
        libc::strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    copy = cJSON_strdup(valuestring as *const u8, &global_hooks()) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;

    copy
}

/* ------------------------------------------------------------------------- */
/* print buffer                                                              */
/* ------------------------------------------------------------------------- */

#[derive(Clone, Copy)]
struct PrintBuffer {
    buffer: *mut u8,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: InternalHooks,
}

/* realloc printbuffer if necessary to have at least "needed" bytes more */
unsafe fn ensure(p: *mut PrintBuffer, mut needed: usize) -> *mut u8 {
    let mut newbuffer: *mut u8;
    let newsize: usize;

    if p.is_null() || (*p).buffer.is_null() {
        return ptr::null_mut();
    }

    if ((*p).length > 0) && ((*p).offset >= (*p).length) {
        /* make sure that offset is valid */
        return ptr::null_mut();
    }

    if needed > c_int::MAX as usize {
        /* sizes bigger than INT_MAX are currently not supported */
        return ptr::null_mut();
    }

    needed = needed.wrapping_add((*p).offset).wrapping_add(1);
    if needed <= (*p).length {
        return (*p).buffer.add((*p).offset);
    }

    if (*p).noalloc != FALSE {
        return ptr::null_mut();
    }

    /* calculate new buffer size */
    if needed > ((c_int::MAX as usize) / 2) {
        /* overflow of int, use INT_MAX if possible */
        if needed <= c_int::MAX as usize {
            newsize = c_int::MAX as usize;
        } else {
            return ptr::null_mut();
        }
    } else {
        newsize = needed * 2;
    }

    if (*p).hooks.reallocate.is_some() {
        /* reallocate with realloc if available */
        newbuffer = (*p).hooks.reallocate((*p).buffer as *mut c_void, newsize) as *mut u8;
        if newbuffer.is_null() {
            (*p).hooks.deallocate((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();

            return ptr::null_mut();
        }
    } else {
        /* otherwise reallocate manually */
        newbuffer = (*p).hooks.allocate(newsize) as *mut u8;
        if newbuffer.is_null() {
            (*p).hooks.deallocate((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();

            return ptr::null_mut();
        }

        libc::memcpy(
            newbuffer as *mut c_void,
            (*p).buffer as *const c_void,
            (*p).offset + 1,
        );
        (*p).hooks.deallocate((*p).buffer as *mut c_void);
    }
    (*p).length = newsize;
    (*p).buffer = newbuffer;

    newbuffer = newbuffer.add((*p).offset);
    newbuffer
}

/* calculate the new length of the string in a printbuffer and update the offset */
unsafe fn update_offset(buffer: *mut PrintBuffer) {
    let buffer_pointer: *const u8;
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return;
    }
    buffer_pointer = (*buffer).buffer.add((*buffer).offset);

    (*buffer).offset += libc::strlen(buffer_pointer as *const c_char);
}

/* securely comparison of floating-point variables */
unsafe fn compare_double(a: c_double, b: c_double) -> cJSON_bool {
    let max_val = if fabs(a) > fabs(b) { fabs(a) } else { fabs(b) };
    if fabs(a - b) <= max_val * f64::EPSILON {
        TRUE
    } else {
        FALSE
    }
}

#[inline]
fn fabs(x: c_double) -> c_double {
    f64::from_bits(x.to_bits() & !(1u64 << 63))
}

#[inline]
fn c_isnan(d: c_double) -> bool {
    d != d
}

#[inline]
fn c_isinf(d: c_double) -> bool {
    /* isinf() from math.h in C99 */
    !c_isnan(d) && fabs(d) == f64::INFINITY
}

/* Render the number nicely from the given item into a string. */
unsafe fn print_number(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut output_pointer: *mut u8;
    let d = (*item).valuedouble;
    let mut length: c_int;
    let mut i: usize;
    let mut number_buffer: [u8; 26] = [0; 26]; /* temporary buffer to print the number into */
    let decimal_point = get_decimal_point();
    let mut test: c_double = 0.0;

    if output_buffer.is_null() {
        return FALSE;
    }

    /* This checks for NaN and Infinity */
    if c_isnan(d) || c_isinf(d) {
        length = libc::snprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            26,
            b"null\0".as_ptr() as *const c_char,
        );
    } else if d == (*item).valueint as c_double {
        length = libc::snprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            26,
            b"%d\0".as_ptr() as *const c_char,
            (*item).valueint,
        );
    } else {
        /* Try 15 decimal places of precision to avoid nonsignificant nonzero digits */
        length = libc::snprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            26,
            b"%1.15g\0".as_ptr() as *const c_char,
            d,
        );

        /* Check whether the original double can be recovered */
        if (libc::sscanf(
            number_buffer.as_ptr() as *const c_char,
            b"%lg\0".as_ptr() as *const c_char,
            &mut test as *mut c_double,
        ) != 1)
            || compare_double(test, d) == FALSE
        {
            /* If not, print with 17 decimal places of precision */
            length = libc::snprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                26,
                b"%1.17g\0".as_ptr() as *const c_char,
                d,
            );
        }
    }

    /* sprintf failed or buffer overrun occurred */
    if (length < 0) || (length > (26 - 1) as c_int) {
        return FALSE;
    }

    /* reserve appropriate space in the output */
    output_pointer = ensure(output_buffer, length as usize + 1);
    if output_pointer.is_null() {
        return FALSE;
    }

    /* copy the printed number to the output and replace locale
     * dependent decimal point with '.' */
    i = 0;
    while i < (length as usize) {
        if number_buffer[i] == decimal_point {
            *output_pointer.add(i) = b'.';
            i += 1;
            continue;
        }

        *output_pointer.add(i) = number_buffer[i];
        i += 1;
    }
    *output_pointer.add(i) = b'\0';

    (*output_buffer).offset += length as usize;

    let _ = &mut output_pointer;
    TRUE
}

/* parse 4 digit hexadecimal number */
unsafe fn parse_hex4(input: *const u8) -> u32 {
    let mut h: u32 = 0;
    let mut i: usize = 0;

    while i < 4 {
        let c = *input.add(i);
        /* parse digit */
        if (c >= b'0') && (c <= b'9') {
            h = h.wrapping_add(c as u32 - b'0' as u32);
        } else if (c >= b'A') && (c <= b'F') {
            h = h.wrapping_add(10u32.wrapping_add(c as u32).wrapping_sub(b'A' as u32));
        } else if (c >= b'a') && (c <= b'f') {
            h = h.wrapping_add(10u32.wrapping_add(c as u32).wrapping_sub(b'a' as u32));
        } else
        /* invalid */
        {
            return 0;
        }

        if i < 3 {
            /* shift left to make place for the next nibble */
            h <<= 4;
        }

        i += 1;
    }

    h
}

/* converts a UTF-16 literal to UTF-8
 * A literal can be one or two sequences of the form \uXXXX */
unsafe fn utf16_literal_to_utf8(
    input_pointer: *const u8,
    input_end: *const u8,
    output_pointer: *mut *mut u8,
) -> u8 {
    let mut codepoint: u64;
    let first_code: u32;
    let first_sequence: *const u8 = input_pointer;
    let utf8_length: u8;
    let mut utf8_position: u8;
    let sequence_length: u8;
    let mut first_byte_mark: u8 = 0;

    if (input_end as isize - first_sequence as isize) < 6 {
        /* input ends unexpectedly */
        return 0;
    }

    /* get the first utf16 sequence */
    first_code = parse_hex4(first_sequence.add(2));

    /* check that the code is valid */
    if (first_code >= 0xDC00) && (first_code <= 0xDFFF) {
        return 0;
    }

    /* UTF16 surrogate pair */
    if (first_code >= 0xD800) && (first_code <= 0xDBFF) {
        let second_sequence: *const u8 = first_sequence.add(6);
        let second_code: u32;
        sequence_length = 12; /* \uXXXX\uXXXX */

        if (input_end as isize - second_sequence as isize) < 6 {
            /* input ends unexpectedly */
            return 0;
        }

        if (*second_sequence != b'\\') || (*second_sequence.add(1) != b'u') {
            /* missing second half of the surrogate pair */
            return 0;
        }

        /* get the second utf16 sequence */
        second_code = parse_hex4(second_sequence.add(2));
        /* check that the code is valid */
        if (second_code < 0xDC00) || (second_code > 0xDFFF) {
            /* invalid second half of the surrogate pair */
            return 0;
        }

        /* calculate the unicode codepoint from the surrogate pair */
        codepoint = 0x10000u64 + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as u64);
    } else {
        sequence_length = 6; /* \uXXXX */
        codepoint = first_code as u64;
    }

    /* encode as UTF-8
     * takes at maximum 4 bytes to encode:
     * 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
    if codepoint < 0x80 {
        /* normal ascii, encoding 0xxxxxxx */
        utf8_length = 1;
    } else if codepoint < 0x800 {
        /* two bytes, encoding 110xxxxx 10xxxxxx */
        utf8_length = 2;
        first_byte_mark = 0xC0; /* 11000000 */
    } else if codepoint < 0x10000 {
        /* three bytes, encoding 1110xxxx 10xxxxxx 10xxxxxx */
        utf8_length = 3;
        first_byte_mark = 0xE0; /* 11100000 */
    } else if codepoint <= 0x10FFFF {
        /* four bytes, encoding 1110xxxx 10xxxxxx 10xxxxxx 10xxxxxx */
        utf8_length = 4;
        first_byte_mark = 0xF0; /* 11110000 */
    } else {
        /* invalid unicode codepoint */
        return 0;
    }

    /* encode as utf8 */
    utf8_position = utf8_length - 1;
    while utf8_position > 0 {
        /* 10xxxxxx */
        *(*output_pointer).add(utf8_position as usize) = ((codepoint | 0x80) & 0xBF) as u8;
        codepoint >>= 6;
        utf8_position -= 1;
    }
    /* encode first byte */
    if utf8_length > 1 {
        *(*output_pointer) = ((codepoint | first_byte_mark as u64) & 0xFF) as u8;
    } else {
        *(*output_pointer) = (codepoint & 0x7F) as u8;
    }

    *output_pointer = (*output_pointer).add(utf8_length as usize);

    sequence_length
}

/* Parse the input text into an unescaped cinput, and populate item. */
unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut input_pointer: *const u8 = buffer_at_offset(input_buffer).add(1);
    let mut input_end: *const u8 = buffer_at_offset(input_buffer).add(1);
    let mut output_pointer: *mut u8;
    let mut output: *mut u8 = ptr::null_mut();

    /* not a string */
    if *buffer_at_offset(input_buffer) != b'\"' {
        return parse_string_fail(input_buffer, input_pointer, output);
    }

    {
        /* calculate approximate size of the output (overestimate) */
        let allocation_length: usize;
        let mut skipped_bytes: usize = 0;
        while ((input_end as usize - (*input_buffer).content as usize) < (*input_buffer).length)
            && (*input_end != b'\"')
        {
            /* is escape sequence */
            if *input_end == b'\\' {
                if (input_end.add(1) as usize - (*input_buffer).content as usize)
                    >= (*input_buffer).length
                {
                    /* prevent buffer overflow when last input character is a backslash */
                    return parse_string_fail(input_buffer, input_pointer, output);
                }
                skipped_bytes += 1;
                input_end = input_end.add(1);
            }
            input_end = input_end.add(1);
        }
        if ((input_end as usize - (*input_buffer).content as usize) >= (*input_buffer).length)
            || (*input_end != b'\"')
        {
            /* string ended unexpectedly */
            return parse_string_fail(input_buffer, input_pointer, output);
        }

        /* This is at most how much we need for the output */
        allocation_length =
            (input_end as usize - buffer_at_offset(input_buffer) as usize) - skipped_bytes;
        output = (*input_buffer).hooks.allocate(allocation_length + 1) as *mut u8;
        if output.is_null() {
            /* allocation failure */
            return parse_string_fail(input_buffer, input_pointer, output);
        }
    }

    output_pointer = output;
    /* loop through the string literal */
    while input_pointer < input_end {
        if *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
            output_pointer = output_pointer.add(1);
            input_pointer = input_pointer.add(1);
        }
        /* escape sequence */
        else {
            let mut sequence_length: u8 = 2;
            if (input_end as isize - input_pointer as isize) < 1 {
                return parse_string_fail(input_buffer, input_pointer, output);
            }

            match *input_pointer.add(1) {
                b'b' => {
                    *output_pointer = b'\x08';
                    output_pointer = output_pointer.add(1);
                }
                b'f' => {
                    *output_pointer = b'\x0c';
                    output_pointer = output_pointer.add(1);
                }
                b'n' => {
                    *output_pointer = b'\n';
                    output_pointer = output_pointer.add(1);
                }
                b'r' => {
                    *output_pointer = b'\r';
                    output_pointer = output_pointer.add(1);
                }
                b't' => {
                    *output_pointer = b'\t';
                    output_pointer = output_pointer.add(1);
                }
                b'\"' | b'\\' | b'/' => {
                    *output_pointer = *input_pointer.add(1);
                    output_pointer = output_pointer.add(1);
                }

                /* UTF-16 literal */
                b'u' => {
                    sequence_length =
                        utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer);
                    if sequence_length == 0 {
                        /* failed to convert UTF16-literal to UTF-8 */
                        return parse_string_fail(input_buffer, input_pointer, output);
                    }
                }

                _ => {
                    return parse_string_fail(input_buffer, input_pointer, output);
                }
            }
            input_pointer = input_pointer.add(sequence_length as usize);
        }
    }

    /* zero terminate the output */
    *output_pointer = b'\0';

    (*item).type_ = cJSON_String;
    (*item).valuestring = output as *mut c_char;

    (*input_buffer).offset = input_end as usize - (*input_buffer).content as usize;
    (*input_buffer).offset += 1;

    TRUE
}

unsafe fn parse_string_fail(
    input_buffer: *mut ParseBuffer,
    input_pointer: *const u8,
    output: *mut u8,
) -> cJSON_bool {
    if !output.is_null() {
        (*input_buffer).hooks.deallocate(output as *mut c_void);
    }

    if !input_pointer.is_null() {
        (*input_buffer).offset = input_pointer as usize - (*input_buffer).content as usize;
    }

    FALSE
}

/* Render the cstring provided to an escaped version that can be printed. */
unsafe fn print_string_ptr(input: *const u8, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut input_pointer: *const u8;
    let output: *mut u8;
    let mut output_pointer: *mut u8;
    let output_length: usize;
    /* numbers of additional characters needed for escaping */
    let mut escape_characters: usize = 0;

    if output_buffer.is_null() {
        return FALSE;
    }

    /* empty string */
    if input.is_null() {
        let out = ensure(output_buffer, 3);
        if out.is_null() {
            return FALSE;
        }
        libc::strcpy(out as *mut c_char, b"\"\"\0".as_ptr() as *const c_char);

        return TRUE;
    }

    /* set "flag" to 1 if something needs to be escaped */
    input_pointer = input;
    while *input_pointer != 0 {
        match *input_pointer {
            b'\"' | b'\\' | b'\x08' | b'\x0c' | b'\n' | b'\r' | b'\t' => {
                /* one character escape sequence */
                escape_characters += 1;
            }
            _ => {
                if *input_pointer < 32 {
                    /* UTF-16 escape sequence uXXXX */
                    escape_characters += 5;
                }
            }
        }
        input_pointer = input_pointer.add(1);
    }
    output_length = (input_pointer as usize - input as usize) + escape_characters;

    output = ensure(output_buffer, output_length + 3);
    if output.is_null() {
        return FALSE;
    }

    /* no characters have to be escaped */
    if escape_characters == 0 {
        *output = b'\"';
        libc::memcpy(
            output.add(1) as *mut c_void,
            input as *const c_void,
            output_length,
        );
        *output.add(output_length + 1) = b'\"';
        *output.add(output_length + 2) = b'\0';

        return TRUE;
    }

    *output = b'\"';
    output_pointer = output.add(1);
    /* copy the string */
    input_pointer = input;
    while *input_pointer != b'\0' {
        if (*input_pointer > 31) && (*input_pointer != b'\"') && (*input_pointer != b'\\') {
            /* normal character, copy */
            *output_pointer = *input_pointer;
        } else {
            /* character needs to be escaped */
            *output_pointer = b'\\';
            output_pointer = output_pointer.add(1);
            match *input_pointer {
                b'\\' => {
                    *output_pointer = b'\\';
                }
                b'\"' => {
                    *output_pointer = b'\"';
                }
                b'\x08' => {
                    *output_pointer = b'b';
                }
                b'\x0c' => {
                    *output_pointer = b'f';
                }
                b'\n' => {
                    *output_pointer = b'n';
                }
                b'\r' => {
                    *output_pointer = b'r';
                }
                b'\t' => {
                    *output_pointer = b't';
                }
                _ => {
                    /* escape and print as unicode codepoint */
                    /* sprintf((char*)output_pointer, "u%04x", *input_pointer); */
                    let v = *input_pointer as u32;
                    *output_pointer = b'u';
                    *output_pointer.add(1) = hex_digit((v >> 12) & 0xF);
                    *output_pointer.add(2) = hex_digit((v >> 8) & 0xF);
                    *output_pointer.add(3) = hex_digit((v >> 4) & 0xF);
                    *output_pointer.add(4) = hex_digit(v & 0xF);
                    *output_pointer.add(5) = b'\0';
                    output_pointer = output_pointer.add(4);
                }
            }
        }

        input_pointer = input_pointer.add(1);
        output_pointer = output_pointer.add(1);
    }
    *output.add(output_length + 1) = b'\"';
    *output.add(output_length + 2) = b'\0';

    TRUE
}

#[inline]
fn hex_digit(nibble: u32) -> u8 {
    if nibble < 10 {
        b'0' + nibble as u8
    } else {
        b'a' + (nibble as u8 - 10)
    }
}

/* Invoke print_string_ptr (which is useful) on an item. */
unsafe fn print_string(item: *const cJSON, p: *mut PrintBuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const u8, p)
}

/* Utility to jump whitespace and cr/lf */
unsafe fn buffer_skip_whitespace(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() {
        return ptr::null_mut();
    }

    if cannot_access_at_index(buffer, 0) {
        return buffer;
    }

    while can_access_at_index(buffer, 0) && (*buffer_at_offset(buffer) <= 32) {
        (*buffer).offset += 1;
    }

    if (*buffer).offset == (*buffer).length {
        (*buffer).offset -= 1;
    }

    buffer
}

/* skip the UTF-8 BOM (byte order mark) if it is at the beginning of a buffer */
unsafe fn skip_utf8_bom(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() || ((*buffer).offset != 0) {
        return ptr::null_mut();
    }

    if can_access_at_index(buffer, 4)
        && (libc::strncmp(
            buffer_at_offset(buffer) as *const c_char,
            b"\xEF\xBB\xBF\0".as_ptr() as *const c_char,
            3,
        ) == 0)
    {
        (*buffer).offset += 3;
    }

    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    let buffer_length: usize;

    if value.is_null() {
        return ptr::null_mut();
    }

    /* Adding null character size due to require_null_terminated. */
    buffer_length = libc::strlen(value) + 1;

    cJSON_ParseWithLengthOpts(
        value,
        buffer_length,
        return_parse_end,
        require_null_terminated,
    )
}

/* Parse an object - create a new root, and populate. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    let mut buffer = ParseBuffer {
        content: ptr::null(),
        length: 0,
        offset: 0,
        depth: 0,
        hooks: NULL_HOOKS,
    };
    let mut item: *mut cJSON = ptr::null_mut();

    /* reset error position */
    GLOBAL_ERROR.json = ptr::null();
    GLOBAL_ERROR.position = 0;

    'fail: {
        if value.is_null() || buffer_length == 0 {
            break 'fail;
        }

        buffer.content = value as *const u8;
        buffer.length = buffer_length;
        buffer.offset = 0;
        buffer.hooks = global_hooks();

        item = cJSON_New_Item(&global_hooks());
        if item.is_null()
        /* memory fail */
        {
            break 'fail;
        }

        if parse_value(
            item,
            buffer_skip_whitespace(skip_utf8_bom(&mut buffer as *mut ParseBuffer)),
        ) == FALSE
        {
            /* parse failure. ep is set. */
            break 'fail;
        }

        /* if we require null-terminated JSON without appended garbage, skip and then check for a null terminator */
        if require_null_terminated != FALSE {
            buffer_skip_whitespace(&mut buffer as *mut ParseBuffer);
            if (buffer.offset >= buffer.length)
                || *buffer_at_offset(&buffer as *const ParseBuffer) != b'\0'
            {
                break 'fail;
            }
        }
        if !return_parse_end.is_null() {
            *return_parse_end = buffer_at_offset(&buffer as *const ParseBuffer) as *const c_char;
        }

        return item;
    }

    /* fail: */
    if !item.is_null() {
        cJSON_Delete(item);
    }

    if !value.is_null() {
        let mut local_error = Error {
            json: value as *const u8,
            position: 0,
        };

        if buffer.offset < buffer.length {
            local_error.position = buffer.offset;
        } else if buffer.length > 0 {
            local_error.position = buffer.length - 1;
        }

        if !return_parse_end.is_null() {
            *return_parse_end = local_error.json.add(local_error.position) as *const c_char;
        }

        GLOBAL_ERROR = local_error;
    }

    ptr::null_mut()
}

/* Default options for cJSON_Parse */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLength(
    value: *const c_char,
    buffer_length: usize,
) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

#[inline]
fn cjson_min(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

unsafe fn print_internal(item: *const cJSON, format: cJSON_bool, hooks: &InternalHooks) -> *mut u8 {
    const DEFAULT_BUFFER_SIZE: usize = 256;
    let mut buffer_storage = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: NULL_HOOKS,
    };
    let buffer: *mut PrintBuffer = &mut buffer_storage;
    let mut printed: *mut u8 = ptr::null_mut();

    'fail: {
        /* create buffer */
        (*buffer).buffer = hooks.allocate(DEFAULT_BUFFER_SIZE) as *mut u8;
        (*buffer).length = DEFAULT_BUFFER_SIZE;
        (*buffer).format = format;
        (*buffer).hooks = *hooks;
        if (*buffer).buffer.is_null() {
            break 'fail;
        }

        /* print the value */
        if print_value(item, buffer) == FALSE {
            break 'fail;
        }
        update_offset(buffer);

        /* check if reallocate is available */
        if hooks.reallocate.is_some() {
            printed =
                hooks.reallocate((*buffer).buffer as *mut c_void, (*buffer).offset + 1) as *mut u8;
            if printed.is_null() {
                break 'fail;
            }
            (*buffer).buffer = ptr::null_mut();
        } else
        /* otherwise copy the JSON over to a new buffer */
        {
            printed = hooks.allocate((*buffer).offset + 1) as *mut u8;
            if printed.is_null() {
                break 'fail;
            }
            libc::memcpy(
                printed as *mut c_void,
                (*buffer).buffer as *const c_void,
                cjson_min((*buffer).length, (*buffer).offset + 1),
            );
            *printed.add((*buffer).offset) = b'\0'; /* just to be sure */

            /* free the buffer */
            hooks.deallocate((*buffer).buffer as *mut c_void);
            (*buffer).buffer = ptr::null_mut();
        }

        return printed;
    }

    /* fail: */
    if !(*buffer).buffer.is_null() {
        hooks.deallocate((*buffer).buffer as *mut c_void);
        (*buffer).buffer = ptr::null_mut();
    }

    if !printed.is_null() {
        hooks.deallocate(printed as *mut c_void);
        printed = ptr::null_mut();
    }

    printed
}

/* Render a cJSON item/entity/structure to text. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print_internal(item, TRUE, &global_hooks()) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print_internal(item, FALSE, &global_hooks()) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(
    item: *const cJSON,
    prebuffer: c_int,
    fmt: cJSON_bool,
) -> *mut c_char {
    let mut p = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: NULL_HOOKS,
    };

    if prebuffer < 0 {
        return ptr::null_mut();
    }

    p.buffer = global_hooks().allocate(prebuffer as usize) as *mut u8;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }

    p.length = prebuffer as usize;
    p.offset = 0;
    p.noalloc = FALSE;
    p.format = fmt;
    p.hooks = global_hooks();

    if print_value(item, &mut p as *mut PrintBuffer) == FALSE {
        global_hooks().deallocate(p.buffer as *mut c_void);
        p.buffer = ptr::null_mut();
        return ptr::null_mut();
    }

    p.buffer as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintPreallocated(
    item: *mut cJSON,
    buffer: *mut c_char,
    length: c_int,
    format: cJSON_bool,
) -> cJSON_bool {
    let mut p = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: NULL_HOOKS,
    };

    if (length < 0) || buffer.is_null() {
        return FALSE;
    }

    p.buffer = buffer as *mut u8;
    p.length = length as usize;
    p.offset = 0;
    p.noalloc = TRUE;
    p.format = format;
    p.hooks = global_hooks();

    print_value(item, &mut p as *mut PrintBuffer)
}

/* Parser core - when encountering text, process appropriately. */
unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return FALSE; /* no input */
    }

    /* parse the different types of values */
    /* null */
    if can_read(input_buffer, 4)
        && (libc::strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"null\0".as_ptr() as *const c_char,
            4,
        ) == 0)
    {
        (*item).type_ = cJSON_NULL;
        (*input_buffer).offset += 4;
        return TRUE;
    }
    /* false */
    if can_read(input_buffer, 5)
        && (libc::strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"false\0".as_ptr() as *const c_char,
            5,
        ) == 0)
    {
        (*item).type_ = cJSON_False;
        (*input_buffer).offset += 5;
        return TRUE;
    }
    /* true */
    if can_read(input_buffer, 4)
        && (libc::strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"true\0".as_ptr() as *const c_char,
            4,
        ) == 0)
    {
        (*item).type_ = cJSON_True;
        (*item).valueint = 1;
        (*input_buffer).offset += 4;
        return TRUE;
    }
    /* string */
    if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer) == b'\"') {
        return parse_string(item, input_buffer);
    }
    /* number */
    if can_access_at_index(input_buffer, 0)
        && ((*buffer_at_offset(input_buffer) == b'-')
            || ((*buffer_at_offset(input_buffer) >= b'0')
                && (*buffer_at_offset(input_buffer) <= b'9')))
    {
        return parse_number(item, input_buffer);
    }
    /* array */
    if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer) == b'[') {
        return parse_array(item, input_buffer);
    }
    /* object */
    if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer) == b'{') {
        return parse_object(item, input_buffer);
    }

    FALSE
}

/* Render a value to text. */
unsafe fn print_value(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let output: *mut u8;

    if item.is_null() || output_buffer.is_null() {
        return FALSE;
    }

    match (*item).type_ & 0xFF {
        x if x == cJSON_NULL => {
            output = ensure(output_buffer, 5);
            if output.is_null() {
                return FALSE;
            }
            libc::strcpy(output as *mut c_char, b"null\0".as_ptr() as *const c_char);
            TRUE
        }

        x if x == cJSON_False => {
            output = ensure(output_buffer, 6);
            if output.is_null() {
                return FALSE;
            }
            libc::strcpy(output as *mut c_char, b"false\0".as_ptr() as *const c_char);
            TRUE
        }

        x if x == cJSON_True => {
            output = ensure(output_buffer, 5);
            if output.is_null() {
                return FALSE;
            }
            libc::strcpy(output as *mut c_char, b"true\0".as_ptr() as *const c_char);
            TRUE
        }

        x if x == cJSON_Number => print_number(item, output_buffer),

        x if x == cJSON_Raw => {
            let raw_length: usize;
            if (*item).valuestring.is_null() {
                return FALSE;
            }

            raw_length = libc::strlen((*item).valuestring) + 1;
            output = ensure(output_buffer, raw_length);
            if output.is_null() {
                return FALSE;
            }
            libc::memcpy(
                output as *mut c_void,
                (*item).valuestring as *const c_void,
                raw_length,
            );
            TRUE
        }

        x if x == cJSON_String => print_string(item, output_buffer),

        x if x == cJSON_Array => print_array(item, output_buffer),

        x if x == cJSON_Object => print_object(item, output_buffer),

        _ => FALSE,
    }
}

/* Build an array from input text. */
unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut(); /* head of the linked list */
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return FALSE; /* to deeply nested */
    }
    (*input_buffer).depth += 1;

    'success: {
        'fail: {
            if *buffer_at_offset(input_buffer) != b'[' {
                /* not an array */
                break 'fail;
            }

            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer) == b']') {
                /* empty array */
                break 'success;
            }

            /* check if we skipped to the end of the buffer */
            if cannot_access_at_index(input_buffer, 0) {
                (*input_buffer).offset -= 1;
                break 'fail;
            }

            /* step back to character in front of the first element */
            (*input_buffer).offset -= 1;
            /* loop through the comma separated array elements */
            loop {
                /* allocate next item */
                let new_item = cJSON_New_Item(&(*input_buffer).hooks);
                if new_item.is_null() {
                    break 'fail; /* allocation failure */
                }

                /* attach next item to list */
                if head.is_null() {
                    /* start the linked list */
                    head = new_item;
                    current_item = new_item;
                } else {
                    /* add to the end and advance */
                    (*current_item).next = new_item;
                    (*new_item).prev = current_item;
                    current_item = new_item;
                }

                /* parse next value */
                (*input_buffer).offset += 1;
                buffer_skip_whitespace(input_buffer);
                if parse_value(current_item, input_buffer) == FALSE {
                    break 'fail; /* failed to parse value */
                }
                buffer_skip_whitespace(input_buffer);

                if !(can_access_at_index(input_buffer, 0)
                    && (*buffer_at_offset(input_buffer) == b','))
                {
                    break;
                }
            }

            if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b']' {
                break 'fail; /* expected end of array */
            }

            break 'success;
        }

        /* fail: */
        if !head.is_null() {
            cJSON_Delete(head);
        }

        return FALSE;
    }

    /* success: */
    (*input_buffer).depth -= 1;

    if !head.is_null() {
        (*head).prev = current_item;
    }

    (*item).type_ = cJSON_Array;
    (*item).child = head;

    (*input_buffer).offset += 1;

    TRUE
}

/* Render an array to text */
unsafe fn print_array(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut output_pointer: *mut u8;
    let mut length: usize;
    let mut current_element: *mut cJSON = (*item).child;

    if output_buffer.is_null() {
        return FALSE;
    }

    /* Compose the output array. */
    /* opening square bracket */
    output_pointer = ensure(output_buffer, 1);
    if output_pointer.is_null() {
        return FALSE;
    }

    *output_pointer = b'[';
    (*output_buffer).offset += 1;
    (*output_buffer).depth += 1;

    while !current_element.is_null() {
        if print_value(current_element, output_buffer) == FALSE {
            return FALSE;
        }
        update_offset(output_buffer);
        if !(*current_element).next.is_null() {
            length = if (*output_buffer).format != FALSE { 2 } else { 1 };
            output_pointer = ensure(output_buffer, length + 1);
            if output_pointer.is_null() {
                return FALSE;
            }
            *output_pointer = b',';
            output_pointer = output_pointer.add(1);
            if (*output_buffer).format != FALSE {
                *output_pointer = b' ';
                output_pointer = output_pointer.add(1);
            }
            *output_pointer = b'\0';
            (*output_buffer).offset += length;
        }
        current_element = (*current_element).next;
    }

    output_pointer = ensure(output_buffer, 2);
    if output_pointer.is_null() {
        return FALSE;
    }
    *output_pointer = b']';
    output_pointer = output_pointer.add(1);
    *output_pointer = b'\0';
    (*output_buffer).depth -= 1;

    TRUE
}

/* Build an object from the text. */
unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut(); /* linked list head */
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return FALSE; /* to deeply nested */
    }
    (*input_buffer).depth += 1;

    'success: {
        'fail: {
            if cannot_access_at_index(input_buffer, 0) || (*buffer_at_offset(input_buffer) != b'{')
            {
                break 'fail; /* not an object */
            }

            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer) == b'}') {
                break 'success; /* empty object */
            }

            /* check if we skipped to the end of the buffer */
            if cannot_access_at_index(input_buffer, 0) {
                (*input_buffer).offset -= 1;
                break 'fail;
            }

            /* step back to character in front of the first element */
            (*input_buffer).offset -= 1;
            /* loop through the comma separated array elements */
            loop {
                /* allocate next item */
                let new_item = cJSON_New_Item(&(*input_buffer).hooks);
                if new_item.is_null() {
                    break 'fail; /* allocation failure */
                }

                /* attach next item to list */
                if head.is_null() {
                    /* start the linked list */
                    head = new_item;
                    current_item = new_item;
                } else {
                    /* add to the end and advance */
                    (*current_item).next = new_item;
                    (*new_item).prev = current_item;
                    current_item = new_item;
                }

                if cannot_access_at_index(input_buffer, 1) {
                    break 'fail; /* nothing comes after the comma */
                }

                /* parse the name of the child */
                (*input_buffer).offset += 1;
                buffer_skip_whitespace(input_buffer);
                if parse_string(current_item, input_buffer) == FALSE {
                    break 'fail; /* failed to parse name */
                }
                buffer_skip_whitespace(input_buffer);

                /* swap valuestring and string, because we parsed the name */
                (*current_item).string = (*current_item).valuestring;
                (*current_item).valuestring = ptr::null_mut();

                if cannot_access_at_index(input_buffer, 0)
                    || (*buffer_at_offset(input_buffer) != b':')
                {
                    break 'fail; /* invalid object */
                }

                /* parse the value */
                (*input_buffer).offset += 1;
                buffer_skip_whitespace(input_buffer);
                if parse_value(current_item, input_buffer) == FALSE {
                    break 'fail; /* failed to parse value */
                }
                buffer_skip_whitespace(input_buffer);

                if !(can_access_at_index(input_buffer, 0)
                    && (*buffer_at_offset(input_buffer) == b','))
                {
                    break;
                }
            }

            if cannot_access_at_index(input_buffer, 0) || (*buffer_at_offset(input_buffer) != b'}')
            {
                break 'fail; /* expected end of object */
            }

            break 'success;
        }

        /* fail: */
        if !head.is_null() {
            cJSON_Delete(head);
        }

        return FALSE;
    }

    /* success: */
    (*input_buffer).depth -= 1;

    if !head.is_null() {
        (*head).prev = current_item;
    }

    (*item).type_ = cJSON_Object;
    (*item).child = head;

    (*input_buffer).offset += 1;
    TRUE
}

/* Render an object to text. */
unsafe fn print_object(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut output_pointer: *mut u8;
    let mut length: usize;
    let mut current_item: *mut cJSON = (*item).child;

    if output_buffer.is_null() {
        return FALSE;
    }

    /* Compose the output: */
    length = if (*output_buffer).format != FALSE { 2 } else { 1 }; /* fmt: {\n */
    output_pointer = ensure(output_buffer, length + 1);
    if output_pointer.is_null() {
        return FALSE;
    }

    *output_pointer = b'{';
    output_pointer = output_pointer.add(1);
    (*output_buffer).depth += 1;
    if (*output_buffer).format != FALSE {
        *output_pointer = b'\n';
        output_pointer = output_pointer.add(1);
    }
    (*output_buffer).offset += length;

    while !current_item.is_null() {
        if (*output_buffer).format != FALSE {
            let mut i: usize;
            output_pointer = ensure(output_buffer, (*output_buffer).depth);
            if output_pointer.is_null() {
                return FALSE;
            }
            i = 0;
            while i < (*output_buffer).depth {
                *output_pointer = b'\t';
                output_pointer = output_pointer.add(1);
                i += 1;
            }
            (*output_buffer).offset += (*output_buffer).depth;
        }

        /* print key */
        if print_string_ptr((*current_item).string as *const u8, output_buffer) == FALSE {
            return FALSE;
        }
        update_offset(output_buffer);

        length = if (*output_buffer).format != FALSE { 2 } else { 1 };
        output_pointer = ensure(output_buffer, length);
        if output_pointer.is_null() {
            return FALSE;
        }
        *output_pointer = b':';
        output_pointer = output_pointer.add(1);
        if (*output_buffer).format != FALSE {
            *output_pointer = b'\t';
            output_pointer = output_pointer.add(1);
        }
        (*output_buffer).offset += length;

        /* print value */
        if print_value(current_item, output_buffer) == FALSE {
            return FALSE;
        }
        update_offset(output_buffer);

        /* print comma if not last */
        length = (if (*output_buffer).format != FALSE { 1 } else { 0 })
            + (if !(*current_item).next.is_null() { 1 } else { 0 });
        output_pointer = ensure(output_buffer, length + 1);
        if output_pointer.is_null() {
            return FALSE;
        }
        if !(*current_item).next.is_null() {
            *output_pointer = b',';
            output_pointer = output_pointer.add(1);
        }

        if (*output_buffer).format != FALSE {
            *output_pointer = b'\n';
            output_pointer = output_pointer.add(1);
        }
        *output_pointer = b'\0';
        (*output_buffer).offset += length;

        current_item = (*current_item).next;
    }

    output_pointer = ensure(
        output_buffer,
        if (*output_buffer).format != FALSE {
            (*output_buffer).depth + 1
        } else {
            2
        },
    );
    if output_pointer.is_null() {
        return FALSE;
    }
    if (*output_buffer).format != FALSE {
        let mut i: usize = 0;
        while i < ((*output_buffer).depth - 1) {
            *output_pointer = b'\t';
            output_pointer = output_pointer.add(1);
            i += 1;
        }
    }
    *output_pointer = b'}';
    output_pointer = output_pointer.add(1);
    *output_pointer = b'\0';
    (*output_buffer).depth -= 1;

    TRUE
}

/* ------------------------------------------------------------------------- */
/* Get Array size/item / object item.                                        */
/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    let mut child: *mut cJSON;
    let mut size: usize = 0;

    if array.is_null() {
        return 0;
    }

    child = (*array).child;

    while !child.is_null() {
        size += 1;
        child = (*child).next;
    }

    /* FIXME: Can overflow here. Cannot be fixed without breaking the API */

    size as c_int
}

unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
    let mut current_child: *mut cJSON;

    if array.is_null() {
        return ptr::null_mut();
    }

    current_child = (*array).child;
    while !current_child.is_null() && (index > 0) {
        index -= 1;
        current_child = (*current_child).next;
    }

    current_child
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 {
        return ptr::null_mut();
    }

    get_array_item(array, index as usize)
}

unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    let mut current_element: *mut cJSON;

    if object.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    current_element = (*object).child;
    if case_sensitive != FALSE {
        while !current_element.is_null()
            && !(*current_element).string.is_null()
            && (libc::strcmp(name, (*current_element).string) != 0)
        {
            current_element = (*current_element).next;
        }
    } else {
        while !current_element.is_null()
            && (case_insensitive_strcmp(
                name as *const u8,
                (*current_element).string as *const u8,
            ) != 0)
        {
            current_element = (*current_element).next;
        }
    }

    if current_element.is_null() || (*current_element).string.is_null() {
        return ptr::null_mut();
    }

    current_element
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    if !cJSON_GetObjectItem(object, string).is_null() {
        1
    } else {
        0
    }
}

/* Utility for array list handling. */
unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {
    (*prev).next = item;
    (*item).prev = prev;
}

/* Utility for handling references. */
unsafe fn create_reference(item: *const cJSON, hooks: &InternalHooks) -> *mut cJSON {
    let reference: *mut cJSON;
    if item.is_null() {
        return ptr::null_mut();
    }

    reference = cJSON_New_Item(hooks);
    if reference.is_null() {
        return ptr::null_mut();
    }

    libc::memcpy(
        reference as *mut c_void,
        item as *const c_void,
        core::mem::size_of::<cJSON>(),
    );
    (*reference).string = ptr::null_mut();
    (*reference).type_ |= cJSON_IsReference;
    (*reference).prev = ptr::null_mut();
    (*reference).next = ptr::null_mut();
    reference
}

unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    let child: *mut cJSON;

    if item.is_null() || array.is_null() || (array == item) {
        return FALSE;
    }

    child = (*array).child;
    /*
     * To find the last item in array quickly, we use prev in array
     */
    if child.is_null() {
        /* list is empty, start new one */
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else {
        /* append to the end */
        if !(*child).prev.is_null() {
            suffix_object((*child).prev, item);
            (*(*array).child).prev = item;
        }
    }

    TRUE
}

/* Add item to array/object. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    add_item_to_array(array, item)
}

/* helper function to cast away const */
#[inline]
fn cast_away_const(string: *const c_void) -> *mut c_void {
    string as *mut c_void
}

unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
    hooks: &InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    let new_key: *mut c_char;
    let new_type: c_int;

    if object.is_null() || string.is_null() || item.is_null() || (object == item) {
        return FALSE;
    }

    if constant_key != FALSE {
        new_key = cast_away_const(string as *const c_void) as *mut c_char;
        new_type = (*item).type_ | cJSON_StringIsConst;
    } else {
        new_key = cJSON_strdup(string as *const u8, hooks) as *mut c_char;
        if new_key.is_null() {
            return FALSE;
        }

        new_type = (*item).type_ & !cJSON_StringIsConst;
    }

    if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
        hooks.deallocate((*item).string as *mut c_void);
    }

    (*item).string = new_key;
    (*item).type_ = new_type;

    add_item_to_array(object, item)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &global_hooks(), FALSE)
}

/* Add an item to an object with constant string as key */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &global_hooks(), TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() {
        return FALSE;
    }

    add_item_to_array(array, create_reference(item, &global_hooks()))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    if object.is_null() || string.is_null() {
        return FALSE;
    }

    add_item_to_object(
        object,
        string,
        create_reference(item, &global_hooks()),
        &global_hooks(),
        FALSE,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let null = cJSON_CreateNull();
    if add_item_to_object(object, name, null, &global_hooks(), FALSE) != FALSE {
        return null;
    }

    cJSON_Delete(null);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let true_item = cJSON_CreateTrue();
    if add_item_to_object(object, name, true_item, &global_hooks(), FALSE) != FALSE {
        return true_item;
    }

    cJSON_Delete(true_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let false_item = cJSON_CreateFalse();
    if add_item_to_object(object, name, false_item, &global_hooks(), FALSE) != FALSE {
        return false_item;
    }

    cJSON_Delete(false_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    let bool_item = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, bool_item, &global_hooks(), FALSE) != FALSE {
        return bool_item;
    }

    cJSON_Delete(bool_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    let number_item = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, number_item, &global_hooks(), FALSE) != FALSE {
        return number_item;
    }

    cJSON_Delete(number_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    let string_item = cJSON_CreateString(string);
    if add_item_to_object(object, name, string_item, &global_hooks(), FALSE) != FALSE {
        return string_item;
    }

    cJSON_Delete(string_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    let raw_item = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, raw_item, &global_hooks(), FALSE) != FALSE {
        return raw_item;
    }

    cJSON_Delete(raw_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let object_item = cJSON_CreateObject();
    if add_item_to_object(object, name, object_item, &global_hooks(), FALSE) != FALSE {
        return object_item;
    }

    cJSON_Delete(object_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let array = cJSON_CreateArray();
    if add_item_to_object(object, name, array, &global_hooks(), FALSE) != FALSE {
        return array;
    }

    cJSON_Delete(array);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
    if parent.is_null()
        || item.is_null()
        || (item != (*parent).child && (*item).prev.is_null())
    {
        return ptr::null_mut();
    }

    if item != (*parent).child {
        /* not the first element */
        (*(*item).prev).next = (*item).next;
    }
    if !(*item).next.is_null() {
        /* not the last element */
        (*(*item).next).prev = (*item).prev;
    }

    if item == (*parent).child {
        /* first element */
        (*parent).child = (*item).next;
    } else if (*item).next.is_null() {
        /* last element */
        (*(*parent).child).prev = (*item).prev;
    }

    /* make sure the detached item doesn't point anywhere anymore */
    (*item).prev = ptr::null_mut();
    (*item).next = ptr::null_mut();

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromArray(
    array: *mut cJSON,
    which: c_int,
) -> *mut cJSON {
    if which < 0 {
        return ptr::null_mut();
    }

    cJSON_DetachItemViaPointer(array, get_array_item(array, which as usize))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItem(object, string);

    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItemCaseSensitive(object, string);

    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

/* Replace array/object items with new ones. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    let after_inserted: *mut cJSON;

    if which < 0 || newitem.is_null() {
        return FALSE;
    }

    after_inserted = get_array_item(array, which as usize);
    if after_inserted.is_null() {
        return add_item_to_array(array, newitem);
    }

    if after_inserted != (*array).child && (*after_inserted).prev.is_null() {
        /* return false if after_inserted is a corrupted array item */
        return FALSE;
    }

    (*newitem).next = after_inserted;
    (*newitem).prev = (*after_inserted).prev;
    (*after_inserted).prev = newitem;
    if after_inserted == (*array).child {
        (*array).child = newitem;
    } else {
        (*(*newitem).prev).next = newitem;
    }
    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
    if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
        return FALSE;
    }

    if replacement == item {
        return TRUE;
    }

    (*replacement).next = (*item).next;
    (*replacement).prev = (*item).prev;

    if !(*replacement).next.is_null() {
        (*(*replacement).next).prev = replacement;
    }
    if (*parent).child == item {
        if (*(*parent).child).prev == (*parent).child {
            (*replacement).prev = replacement;
        }
        (*parent).child = replacement;
    } else {
        /*
         * To find the last item in array quickly, we use prev in array.
         * We can't modify the last item's next pointer where this item was the parent's child
         */
        if !(*replacement).prev.is_null() {
            (*(*replacement).prev).next = replacement;
        }
        if (*replacement).next.is_null() {
            (*(*parent).child).prev = replacement;
        }
    }

    (*item).next = ptr::null_mut();
    (*item).prev = ptr::null_mut();
    cJSON_Delete(item);

    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 {
        return FALSE;
    }

    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if replacement.is_null() || string.is_null() {
        return FALSE;
    }

    /* replace the name in the replacement */
    if ((*replacement).type_ & cJSON_StringIsConst) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string = cJSON_strdup(string as *const u8, &global_hooks()) as *mut c_char;
    if (*replacement).string.is_null() {
        return FALSE;
    }

    (*replacement).type_ &= !cJSON_StringIsConst;

    cJSON_ReplaceItemViaPointer(
        object,
        get_object_item(object, string, case_sensitive),
        replacement,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, TRUE)
}

/* ------------------------------------------------------------------------- */
/* Create basic types:                                                       */
/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_NULL;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_True;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_False;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = if boolean != FALSE { cJSON_True } else { cJSON_False };
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_Number;
        (*item).valuedouble = num;

        /* use saturation in case of overflow */
        if num >= c_int::MAX as c_double {
            (*item).valueint = c_int::MAX;
        } else if num <= c_int::MIN as c_double {
            (*item).valueint = c_int::MIN;
        } else {
            (*item).valueint = c_double_to_int(num);
        }
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_String;
        (*item).valuestring = cJSON_strdup(string as *const u8, &global_hooks()) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_String | cJSON_IsReference;
        (*item).valuestring = cast_away_const(string as *const c_void) as *mut c_char;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_Object | cJSON_IsReference;
        (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_Array | cJSON_IsReference;
        (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_Raw;
        (*item).valuestring = cJSON_strdup(raw as *const u8, &global_hooks()) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_Array;
    }

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks());
    if !item.is_null() {
        (*item).type_ = cJSON_Object;
    }

    item
}

/* Create Arrays: */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    let mut i: usize = 0;
    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a: *mut cJSON;

    if (count < 0) || numbers.is_null() {
        return ptr::null_mut();
    }

    a = cJSON_CreateArray();

    while !a.is_null() && (i < count as usize) {
        n = cJSON_CreateNumber(*numbers.add(i) as c_double);
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
        }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
        i += 1;
    }

    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = n;
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(
    numbers: *const c_float,
    count: c_int,
) -> *mut cJSON {
    let mut i: usize = 0;
    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a: *mut cJSON;

    if (count < 0) || numbers.is_null() {
        return ptr::null_mut();
    }

    a = cJSON_CreateArray();

    while !a.is_null() && (i < count as usize) {
        n = cJSON_CreateNumber(*numbers.add(i) as c_double);
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
        }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
        i += 1;
    }

    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = n;
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
    let mut i: usize = 0;
    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a: *mut cJSON;

    if (count < 0) || numbers.is_null() {
        return ptr::null_mut();
    }

    a = cJSON_CreateArray();

    while !a.is_null() && (i < count as usize) {
        n = cJSON_CreateNumber(*numbers.add(i));
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
        }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
        i += 1;
    }

    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = n;
    }

    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    let mut i: usize = 0;
    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a: *mut cJSON;

    if (count < 0) || strings.is_null() {
        return ptr::null_mut();
    }

    a = cJSON_CreateArray();

    while !a.is_null() && (i < count as usize) {
        n = cJSON_CreateString(*strings.add(i));
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
        }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
        i += 1;
    }

    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = n;
    }

    a
}

/* Duplication */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: usize,
    recurse: cJSON_bool,
) -> *mut cJSON {
    let mut newitem: *mut cJSON = ptr::null_mut();
    let mut child: *mut cJSON;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();

    'fail: {
        /* Bail on bad ptr */
        if item.is_null() {
            break 'fail;
        }
        /* Create new item */
        newitem = cJSON_New_Item(&global_hooks());
        if newitem.is_null() {
            break 'fail;
        }
        /* Copy over all vars */
        (*newitem).type_ = (*item).type_ & !cJSON_IsReference;
        (*newitem).valueint = (*item).valueint;
        (*newitem).valuedouble = (*item).valuedouble;
        if !(*item).valuestring.is_null() {
            (*newitem).valuestring =
                cJSON_strdup((*item).valuestring as *const u8, &global_hooks()) as *mut c_char;
            if (*newitem).valuestring.is_null() {
                break 'fail;
            }
        }
        if !(*item).string.is_null() {
            (*newitem).string = if ((*item).type_ & cJSON_StringIsConst) != 0 {
                (*item).string
            } else {
                cJSON_strdup((*item).string as *const u8, &global_hooks()) as *mut c_char
            };
            if (*newitem).string.is_null() {
                break 'fail;
            }
        }
        /* If non-recursive, then we're done! */
        if recurse == FALSE {
            return newitem;
        }
        /* Walk the ->next chain for the child. */
        child = (*item).child;
        while !child.is_null() {
            if depth >= CJSON_CIRCULAR_LIMIT {
                break 'fail;
            }
            /* Duplicate (with recurse) each item in the ->next chain */
            newchild = cJSON_Duplicate_rec(child, depth + 1, TRUE);
            if newchild.is_null() {
                break 'fail;
            }
            if !next.is_null() {
                /* If newitem->child already set, then crosswire ->prev and ->next and move on */
                (*next).next = newchild;
                (*newchild).prev = next;
                next = newchild;
            } else {
                /* Set newitem->child and move to it */
                (*newitem).child = newchild;
                next = newchild;
            }
            child = (*child).next;
        }
        if !newitem.is_null() && !(*newitem).child.is_null() {
            (*(*newitem).child).prev = newchild;
        }

        return newitem;
    }

    /* fail: */
    if !newitem.is_null() {
        cJSON_Delete(newitem);
    }

    ptr::null_mut()
}

/* ------------------------------------------------------------------------- */
/* Minify                                                                    */
/* ------------------------------------------------------------------------- */

unsafe fn skip_oneline_comment(input: *mut *mut c_char) {
    /* static_strlen("//") == 2 */
    *input = (*input).add(2);

    while *(*input) != 0 {
        if *(*input) == b'\n' as c_char {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut c_char) {
    // static_strlen("/" "*") == 2
    *input = (*input).add(2);

    while *(*input) != 0 {
        if (*(*input) == b'*' as c_char) && (*(*input).add(1) == b'/' as c_char) {
            *input = (*input).add(2);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn minify_string(input: *mut *mut c_char, output: *mut *mut c_char) {
    *(*output) = *(*input);
    *input = (*input).add(1);
    *output = (*output).add(1);

    while *(*input) != 0 {
        *(*output) = *(*input);

        if *(*input) == b'\"' as c_char {
            *(*output) = b'\"' as c_char;
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        } else if (*(*input) == b'\\' as c_char) && (*(*input).add(1) == b'\"' as c_char) {
            *(*output).add(1) = *(*input).add(1);
            *input = (*input).add(1);
            *output = (*output).add(1);
        }

        *input = (*input).add(1);
        *output = (*output).add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Minify(json: *mut c_char) {
    let mut into: *mut c_char = json;
    let mut json: *mut c_char = json;

    if json.is_null() {
        return;
    }

    while *json != 0 {
        match *json as u8 {
            b' ' | b'\t' | b'\r' | b'\n' => {
                json = json.add(1);
            }

            b'/' => {
                if *json.add(1) == b'/' as c_char {
                    skip_oneline_comment(&mut json);
                } else if *json.add(1) == b'*' as c_char {
                    skip_multiline_comment(&mut json);
                } else {
                    json = json.add(1);
                }
            }

            b'\"' => {
                minify_string(&mut json, &mut into);
            }

            _ => {
                *into = *json;
                json = json.add(1);
                into = into.add(1);
            }
        }
    }

    /* and null-terminate. */
    *into = 0;
}

/* ------------------------------------------------------------------------- */
/* Type checks                                                               */
/* ------------------------------------------------------------------------- */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_Invalid) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_False) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xff) == cJSON_True) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & (cJSON_True | cJSON_False)) != 0) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_NULL) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_Number) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_String) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_Array) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_Object) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }

    (((*item).type_ & 0xFF) == cJSON_Raw) as cJSON_bool
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null() || b.is_null() || (((*a).type_ & 0xFF) != ((*b).type_ & 0xFF)) {
        return FALSE;
    }

    /* check if type is valid */
    match (*a).type_ & 0xFF {
        x if x == cJSON_False
            || x == cJSON_True
            || x == cJSON_NULL
            || x == cJSON_Number
            || x == cJSON_String
            || x == cJSON_Raw
            || x == cJSON_Array
            || x == cJSON_Object => {}

        _ => return FALSE,
    }

    /* identical objects are equal */
    if a == b {
        return TRUE;
    }

    match (*a).type_ & 0xFF {
        /* in these cases and equal type is enough */
        x if x == cJSON_False || x == cJSON_True || x == cJSON_NULL => TRUE,

        x if x == cJSON_Number => {
            if compare_double((*a).valuedouble, (*b).valuedouble) != FALSE {
                return TRUE;
            }
            FALSE
        }

        x if x == cJSON_String || x == cJSON_Raw => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() {
                return FALSE;
            }
            if libc::strcmp((*a).valuestring, (*b).valuestring) == 0 {
                return TRUE;
            }

            FALSE
        }

        x if x == cJSON_Array => {
            let mut a_element: *mut cJSON = (*a).child;
            let mut b_element: *mut cJSON = (*b).child;

            while !a_element.is_null() && !b_element.is_null() {
                if cJSON_Compare(a_element, b_element, case_sensitive) == FALSE {
                    return FALSE;
                }

                a_element = (*a_element).next;
                b_element = (*b_element).next;
            }

            /* one of the arrays is longer than the other */
            if a_element != b_element {
                return FALSE;
            }

            TRUE
        }

        x if x == cJSON_Object => {
            let mut a_element: *mut cJSON;
            let mut b_element: *mut cJSON;

            a_element = if !a.is_null() {
                (*a).child
            } else {
                ptr::null_mut()
            };
            while !a_element.is_null() {
                /* TODO This has O(n^2) runtime, which is horrible! */
                b_element = get_object_item(b, (*a_element).string, case_sensitive);
                if b_element.is_null() {
                    return FALSE;
                }

                if cJSON_Compare(a_element, b_element, case_sensitive) == FALSE {
                    return FALSE;
                }

                a_element = (*a_element).next;
            }

            /* doing this twice, once on a and b to prevent true comparison if a subset of b
             * TODO: Do this the proper way, this is just a fix for now */
            b_element = if !b.is_null() {
                (*b).child
            } else {
                ptr::null_mut()
            };
            while !b_element.is_null() {
                a_element = get_object_item(a, (*b_element).string, case_sensitive);
                if a_element.is_null() {
                    return FALSE;
                }

                if cJSON_Compare(b_element, a_element, case_sensitive) == FALSE {
                    return FALSE;
                }

                b_element = (*b_element).next;
            }

            TRUE
        }

        _ => FALSE,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    global_hooks().allocate(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    global_hooks().deallocate(object);
}
