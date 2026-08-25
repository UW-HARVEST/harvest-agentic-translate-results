use crate::error::{VaList, set_error};
use crate::memory::{jsonp_free, jsonp_malloc};
use crate::types::*;
use crate::value::*;
use crate::*;
use std::arch::{global_asm, naked_asm};
use std::ffi::{CStr, c_char, c_int};
use std::ptr;

const JSON_VALIDATE_ONLY: usize = 0x1;
const JSON_STRICT: usize = 0x2;
const ERROR_INVALID_ARGUMENT: u8 = 4;
const ERROR_INVALID_FORMAT: u8 = 9;
const ERROR_WRONG_TYPE: u8 = 10;
const ERROR_NULL_VALUE: u8 = 12;
const ERROR_ITEM_NOT_FOUND: u8 = 16;
const ERROR_INDEX_OUT_OF_RANGE: u8 = 17;

struct Args {
    list: VaList,
}

impl Args {
    unsafe fn new(list: *mut VaList) -> Self {
        Self {
            list: ptr::read(list),
        }
    }

    unsafe fn gp_u64(&mut self) -> u64 {
        if self.list.gp_offset <= 40 {
            let value = ptr::read_unaligned(
                self.list
                    .reg_save_area
                    .add(self.list.gp_offset as usize)
                    .cast::<u64>(),
            );
            self.list.gp_offset += 8;
            value
        } else {
            let address = (self.list.overflow_arg_area as usize + 7) & !7;
            self.list.overflow_arg_area = (address + 8) as *mut u8;
            ptr::read_unaligned(address as *const u64)
        }
    }

    unsafe fn pointer<T>(&mut self) -> *mut T {
        self.gp_u64() as usize as *mut T
    }

    unsafe fn integer(&mut self) -> c_int {
        self.gp_u64() as c_int
    }

    unsafe fn int64(&mut self) -> i64 {
        self.gp_u64() as i64
    }

    unsafe fn size(&mut self) -> usize {
        self.gp_u64() as usize
    }

    unsafe fn double(&mut self) -> f64 {
        if self.list.fp_offset <= 160 {
            let value = ptr::read_unaligned(
                self.list
                    .reg_save_area
                    .add(self.list.fp_offset as usize)
                    .cast::<f64>(),
            );
            self.list.fp_offset += 16;
            value
        } else {
            let address = (self.list.overflow_arg_area as usize + 7) & !7;
            self.list.overflow_arg_area = (address + 8) as *mut u8;
            ptr::read_unaligned(address as *const f64)
        }
    }
}

struct Format<'a> {
    bytes: &'a [u8],
    index: usize,
    error: *mut json_error_t,
    flags: usize,
}

impl<'a> Format<'a> {
    unsafe fn has_error(&self) -> bool {
        !self.error.is_null() && (*self.error).text[0] != 0
    }

    fn skip(&mut self) {
        while matches!(
            self.bytes.get(self.index),
            Some(b' ' | b'\t' | b'\n' | b',' | b':')
        ) {
            self.index += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.bytes.get(self.index).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let value = self.peek()?;
        self.index += 1;
        Some(value)
    }

    unsafe fn fail(&mut self, source: *const c_char, code: u8, message: &str) {
        set_error(
            self.error,
            1,
            self.index as c_int,
            self.index,
            code,
            message,
        );
        crate::error::jsonp_error_set_source(self.error, source);
    }
}

unsafe fn read_pack_string(
    format: &mut Format<'_>,
    args: &mut Args,
    optional: bool,
) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    loop {
        let string = args.pointer::<c_char>();
        if string.is_null() {
            if !optional {
                format.fail(c"<args>".as_ptr(), ERROR_NULL_VALUE, "NULL string");
            }
            return None;
        }
        let length = match format.peek() {
            Some(b'#') => {
                format.next();
                args.integer() as u32 as usize
            }
            Some(b'%') => {
                format.next();
                args.size()
            }
            _ => CStr::from_ptr(string).to_bytes().len(),
        };
        output.extend_from_slice(std::slice::from_raw_parts(string.cast(), length));
        if format.peek() != Some(b'+') {
            break;
        }
        format.next();
    }
    if !crate::private::utf8_valid(&output) {
        format.fail(c"<args>".as_ptr(), 5, "Invalid UTF-8 string");
        None
    } else {
        Some(output)
    }
}

unsafe fn pack_value(format: &mut Format<'_>, args: &mut Args) -> *mut json_t {
    let Some(token) = format.next() else {
        format.fail(
            c"<format>".as_ptr(),
            ERROR_INVALID_FORMAT,
            "Unexpected end of format string",
        );
        return ptr::null_mut();
    };
    match token {
        b'{' => {
            let object = json_object();
            while format.peek() != Some(b'}') {
                if format.next() != Some(b's') {
                    format.fail(
                        c"<format>".as_ptr(),
                        ERROR_INVALID_FORMAT,
                        "Expected format 's'",
                    );
                    decref(object);
                    return ptr::null_mut();
                }
                let Some(key) = read_pack_string(format, args, false) else {
                    decref(object);
                    return ptr::null_mut();
                };
                let optional = {
                    let saved = format.index;
                    let _ = format.next();
                    let optional = format.peek() == Some(b'*');
                    format.index = saved;
                    optional
                };
                let value = pack_value(format, args);
                if value.is_null() {
                    if optional {
                        continue;
                    }
                    if !format.has_error() {
                        format.fail(c"<args>".as_ptr(), ERROR_NULL_VALUE, "NULL object value");
                    }
                    decref(object);
                    return ptr::null_mut();
                }
                json_object_setn_new_nocheck(object, key.as_ptr().cast(), key.len(), value);
            }
            format.next();
            object
        }
        b'[' => {
            let array = json_array();
            while format.peek() != Some(b']') {
                let optional = {
                    let saved = format.index;
                    let _ = format.next();
                    let optional = format.peek() == Some(b'*');
                    format.index = saved;
                    optional
                };
                let value = pack_value(format, args);
                if value.is_null() {
                    if optional {
                        continue;
                    }
                    decref(array);
                    return ptr::null_mut();
                }
                json_array_append_new(array, value);
            }
            format.next();
            array
        }
        b's' => {
            let modifier = format.peek();
            let optional = matches!(modifier, Some(b'?' | b'*'));
            if optional {
                format.next();
            }
            match read_pack_string(format, args, optional) {
                Some(bytes) => json_stringn_nocheck(bytes.as_ptr().cast(), bytes.len()),
                None if modifier == Some(b'?') && !format.has_error() => json_null(),
                None => ptr::null_mut(),
            }
        }
        b'n' => json_null(),
        b'b' => {
            if args.integer() != 0 {
                json_true()
            } else {
                json_false()
            }
        }
        b'i' => json_integer(args.integer() as i64),
        b'I' => json_integer(args.int64()),
        b'f' => json_real(args.double()),
        b'O' | b'o' => {
            let modifier = format.peek();
            if matches!(modifier, Some(b'?' | b'*')) {
                format.next();
            }
            let value = args.pointer::<json_t>();
            if !value.is_null() {
                if token == b'O' { incref(value) } else { value }
            } else if modifier == Some(b'?') {
                json_null()
            } else if modifier == Some(b'*') {
                ptr::null_mut()
            } else {
                format.fail(c"<args>".as_ptr(), ERROR_NULL_VALUE, "NULL object");
                ptr::null_mut()
            }
        }
        _ => {
            format.fail(
                c"<format>".as_ptr(),
                ERROR_INVALID_FORMAT,
                &format!("Unexpected format character '{}'", token as char),
            );
            ptr::null_mut()
        }
    }
}

unsafe fn type_matches(value: *mut json_t, token: u8) -> bool {
    match token {
        b's' => is_type(value, JSON_STRING),
        b'i' | b'I' => is_type(value, JSON_INTEGER),
        b'b' => matches!(type_of(value), Some(JSON_TRUE | JSON_FALSE)),
        b'f' => is_type(value, JSON_REAL),
        b'F' => matches!(type_of(value), Some(JSON_INTEGER | JSON_REAL)),
        b'n' => is_type(value, JSON_NULL),
        _ => true,
    }
}

unsafe fn unpack_value(format: &mut Format<'_>, root: *mut json_t, args: &mut Args) -> c_int {
    let Some(token) = format.next() else {
        format.fail(
            c"<format>".as_ptr(),
            ERROR_INVALID_FORMAT,
            "Unexpected end of format string",
        );
        return -1;
    };
    match token {
        b'{' => {
            if !root.is_null() && !is_type(root, JSON_OBJECT) {
                format.fail(
                    c"<validation>".as_ptr(),
                    ERROR_WRONG_TYPE,
                    "Expected object",
                );
                return -1;
            }
            let mut used = 0usize;
            let mut strict = format.flags & JSON_STRICT != 0;
            while format.peek() != Some(b'}') {
                if matches!(format.peek(), Some(b'!' | b'*')) {
                    strict = format.next() == Some(b'!');
                    continue;
                }
                if format.next() != Some(b's') {
                    format.fail(
                        c"<format>".as_ptr(),
                        ERROR_INVALID_FORMAT,
                        "Expected format 's'",
                    );
                    return -1;
                }
                let key = args.pointer::<c_char>();
                if key.is_null() {
                    format.fail(c"<args>".as_ptr(), ERROR_NULL_VALUE, "NULL object key");
                    return -1;
                }
                let optional = if format.peek() == Some(b'?') {
                    format.next();
                    true
                } else {
                    false
                };
                let value = if root.is_null() {
                    ptr::null_mut()
                } else {
                    json_object_get(root, key)
                };
                if value.is_null() && !optional && !root.is_null() {
                    format.fail(
                        c"<validation>".as_ptr(),
                        ERROR_ITEM_NOT_FOUND,
                        "Object item not found",
                    );
                    return -1;
                }
                if unpack_value(format, value, args) != 0 {
                    return -1;
                }
                if !value.is_null() {
                    used += 1;
                }
            }
            format.next();
            if strict && !root.is_null() && used != json_object_size(root) {
                let object = object_ref(root);
                let unpacked = json_object_size(root) - used;
                let remaining = object
                    .entries
                    .iter()
                    .rev()
                    .take(unpacked)
                    .map(|entry| {
                        String::from_utf8_lossy(
                            &entry.key[std::mem::size_of::<usize>()..][..entry.key_len],
                        )
                    })
                    .collect::<Vec<_>>();
                format.fail(
                    c"<validation>".as_ptr(),
                    9,
                    &format!(
                        "{unpacked} object item(s) left unpacked: {}",
                        remaining.into_iter().rev().collect::<Vec<_>>().join(", ")
                    ),
                );
                -1
            } else {
                0
            }
        }
        b'[' => {
            if !root.is_null() && !is_type(root, JSON_ARRAY) {
                format.fail(c"<validation>".as_ptr(), ERROR_WRONG_TYPE, "Expected array");
                return -1;
            }
            let mut index = 0;
            let mut strict = format.flags & JSON_STRICT != 0;
            while format.peek() != Some(b']') {
                if matches!(format.peek(), Some(b'!' | b'*')) {
                    strict = format.next() == Some(b'!');
                    continue;
                }
                let value = if root.is_null() {
                    ptr::null_mut()
                } else {
                    json_array_get(root, index)
                };
                if value.is_null() && !root.is_null() {
                    format.fail(
                        c"<validation>".as_ptr(),
                        ERROR_INDEX_OUT_OF_RANGE,
                        "Array index out of range",
                    );
                    return -1;
                }
                if unpack_value(format, value, args) != 0 {
                    return -1;
                }
                index += 1;
            }
            format.next();
            if strict && !root.is_null() && index != json_array_size(root) {
                format.fail(c"<validation>".as_ptr(), 7, "array item(s) left unpacked");
                -1
            } else {
                0
            }
        }
        b's' | b'i' | b'I' | b'b' | b'f' | b'F' | b'O' | b'o' | b'n' => {
            if !root.is_null() && !type_matches(root, token) {
                format.fail(
                    c"<validation>".as_ptr(),
                    ERROR_WRONG_TYPE,
                    "Wrong JSON type",
                );
                return -1;
            }
            if token == b'n' || format.flags & JSON_VALIDATE_ONLY != 0 {
                return 0;
            }
            match token {
                b's' => {
                    let target = args.pointer::<*const c_char>();
                    if target.is_null() {
                        format.fail(c"<args>".as_ptr(), ERROR_NULL_VALUE, "NULL string argument");
                        return -1;
                    }
                    let length_target = if format.peek() == Some(b'%') {
                        format.next();
                        args.pointer::<usize>()
                    } else {
                        ptr::null_mut()
                    };
                    if !root.is_null() {
                        *target = json_string_value(root);
                        if !length_target.is_null() {
                            *length_target = json_string_length(root);
                        }
                    }
                }
                b'i' => {
                    let target = args.pointer::<c_int>();
                    if !root.is_null() {
                        *target = json_integer_value(root) as c_int;
                    }
                }
                b'I' => {
                    let target = args.pointer::<i64>();
                    if !root.is_null() {
                        *target = json_integer_value(root);
                    }
                }
                b'b' => {
                    let target = args.pointer::<c_int>();
                    if !root.is_null() {
                        *target = is_type(root, JSON_TRUE) as c_int;
                    }
                }
                b'f' => {
                    let target = args.pointer::<f64>();
                    if !root.is_null() {
                        *target = json_real_value(root);
                    }
                }
                b'F' => {
                    let target = args.pointer::<f64>();
                    if !root.is_null() {
                        *target = json_number_value(root);
                    }
                }
                b'O' | b'o' => {
                    let target = args.pointer::<*mut json_t>();
                    if !root.is_null() {
                        if token == b'O' {
                            incref(root);
                        }
                        *target = root;
                    }
                }
                _ => {}
            }
            0
        }
        _ => {
            format.fail(
                c"<format>".as_ptr(),
                ERROR_INVALID_FORMAT,
                &format!("Unexpected format character '{}'", token as char),
            );
            -1
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vpack_ex(
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: *mut VaList,
) -> *mut json_t {
    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, c"<format>".as_ptr());
        set_error(
            error,
            -1,
            -1,
            0,
            ERROR_INVALID_ARGUMENT,
            "NULL or empty format string",
        );
        return ptr::null_mut();
    }
    jsonp_error_init(error, ptr::null());
    let mut format = Format {
        bytes: CStr::from_ptr(fmt).to_bytes(),
        index: 0,
        error,
        flags,
    };
    let mut args = Args::new(ap);
    let value = pack_value(&mut format, &mut args);
    if value.is_null() {
        return value;
    }
    if format.peek().is_some() {
        decref(value);
        format.fail(
            c"<format>".as_ptr(),
            ERROR_INVALID_FORMAT,
            "Garbage after format string",
        );
        ptr::null_mut()
    } else {
        value
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vunpack_ex(
    root: *mut json_t,
    error: *mut json_error_t,
    flags: usize,
    fmt: *const c_char,
    ap: *mut VaList,
) -> c_int {
    if root.is_null() {
        jsonp_error_init(error, c"<root>".as_ptr());
        set_error(error, -1, -1, 0, ERROR_NULL_VALUE, "NULL root value");
        return -1;
    }
    if fmt.is_null() || *fmt == 0 {
        jsonp_error_init(error, c"<format>".as_ptr());
        set_error(
            error,
            -1,
            -1,
            0,
            ERROR_INVALID_ARGUMENT,
            "NULL or empty format string",
        );
        return -1;
    }
    jsonp_error_init(error, ptr::null());
    let mut format = Format {
        bytes: CStr::from_ptr(fmt).to_bytes(),
        index: 0,
        error,
        flags,
    };
    let mut args = Args::new(ap);
    if unpack_value(&mut format, root, &mut args) != 0 {
        return -1;
    }
    if format.peek().is_some() {
        format.fail(
            c"<format>".as_ptr(),
            ERROR_INVALID_FORMAT,
            "Garbage after format string",
        );
        -1
    } else {
        0
    }
}

unsafe extern "C" {
    fn vsnprintf(buffer: *mut c_char, size: usize, format: *const c_char, ap: *mut VaList)
    -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_vsprintf(fmt: *const c_char, ap: *mut VaList) -> *mut json_t {
    if fmt.is_null() {
        return ptr::null_mut();
    }
    let mut first = ptr::read(ap);
    let length = vsnprintf(ptr::null_mut(), 0, fmt, &mut first);
    if length < 0 {
        return ptr::null_mut();
    }
    let buffer = jsonp_malloc(length as usize + 1).cast::<c_char>();
    if buffer.is_null() {
        return ptr::null_mut();
    }
    let mut second = ptr::read(ap);
    vsnprintf(buffer, length as usize + 1, fmt, &mut second);
    if !crate::private::utf8_valid(std::slice::from_raw_parts(buffer.cast(), length as usize)) {
        jsonp_free(buffer.cast());
        return ptr::null_mut();
    }
    jsonp_stringn_nocheck_own(buffer, length as usize)
}

global_asm!(
    r#"
    .macro SAVE_VARARGS gp
        subq $232, %rsp
        movl $\gp, 0(%rsp)
        movl $48, 4(%rsp)
        leaq 240(%rsp), %rax
        movq %rax, 8(%rsp)
        leaq 32(%rsp), %rax
        movq %rax, 16(%rsp)
        movq %rdi, 32(%rsp)
        movq %rsi, 40(%rsp)
        movq %rdx, 48(%rsp)
        movq %rcx, 56(%rsp)
        movq %r8, 64(%rsp)
        movq %r9, 72(%rsp)
        movaps %xmm0, 80(%rsp)
        movaps %xmm1, 96(%rsp)
        movaps %xmm2, 112(%rsp)
        movaps %xmm3, 128(%rsp)
        movaps %xmm4, 144(%rsp)
        movaps %xmm5, 160(%rsp)
        movaps %xmm6, 176(%rsp)
        movaps %xmm7, 192(%rsp)
    .endm

    .text
    .type __json_pack_asm,@function
__json_pack_asm:
    SAVE_VARARGS 8
    movq 32(%rsp), %rdx
    xorl %edi, %edi
    xorl %esi, %esi
    movq %rsp, %rcx
    call json_vpack_ex
    addq $232, %rsp
    ret
    .size __json_pack_asm, .-__json_pack_asm

    .type __json_pack_ex_asm,@function
__json_pack_ex_asm:
    SAVE_VARARGS 24
    movq %rsp, %rcx
    call json_vpack_ex
    addq $232, %rsp
    ret
    .size __json_pack_ex_asm, .-__json_pack_ex_asm

    .type __json_unpack_asm,@function
__json_unpack_asm:
    SAVE_VARARGS 16
    movq 40(%rsp), %rcx
    xorl %esi, %esi
    xorl %edx, %edx
    movq %rsp, %r8
    call json_vunpack_ex
    addq $232, %rsp
    ret
    .size __json_unpack_asm, .-__json_unpack_asm

    .type __json_unpack_ex_asm,@function
__json_unpack_ex_asm:
    SAVE_VARARGS 32
    movq %rsp, %r8
    call json_vunpack_ex
    addq $232, %rsp
    ret
    .size __json_unpack_ex_asm, .-__json_unpack_ex_asm

    .type __json_sprintf_asm,@function
__json_sprintf_asm:
    SAVE_VARARGS 8
    movq %rsp, %rsi
    call json_vsprintf
    addq $232, %rsp
    ret
    .size __json_sprintf_asm, .-__json_sprintf_asm
"#,
    options(att_syntax)
);

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack(_fmt: *const c_char) -> *mut json_t {
    naked_asm!("jmp __json_pack_asm");
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_pack_ex(
    _error: *mut json_error_t,
    _flags: usize,
    _fmt: *const c_char,
) -> *mut json_t {
    naked_asm!("jmp __json_pack_ex_asm");
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack(_root: *mut json_t, _fmt: *const c_char) -> c_int {
    naked_asm!("jmp __json_unpack_asm");
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_unpack_ex(
    _root: *mut json_t,
    _error: *mut json_error_t,
    _flags: usize,
    _fmt: *const c_char,
) -> c_int {
    naked_asm!("jmp __json_unpack_ex_asm");
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_sprintf(_fmt: *const c_char) -> *mut json_t {
    naked_asm!("jmp __json_sprintf_asm");
}
