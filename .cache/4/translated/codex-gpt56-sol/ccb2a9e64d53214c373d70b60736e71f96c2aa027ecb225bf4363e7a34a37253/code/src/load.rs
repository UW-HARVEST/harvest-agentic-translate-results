use crate::error::*;
use crate::types::*;
use crate::value::{decref, json_array_append_new};
use crate::*;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;

struct Parser<'a> {
    input: &'a [u8],
    index: usize,
    line: c_int,
    column: c_int,
    flags: usize,
    error: *mut json_error_t,
    depth: usize,
}

impl<'a> Parser<'a> {
    fn fail<T>(&mut self, code: u8, message: &str) -> Result<T, ()> {
        unsafe {
            set_error(
                self.error,
                self.line,
                self.column,
                self.index,
                code,
                message,
            );
        }
        Err(())
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = *self.input.get(self.index)?;
        self.index += 1;
        if byte == b'\n' {
            self.line += 1;
            self.column = 0;
        } else {
            self.column += 1;
        }
        Some(byte)
    }

    fn whitespace(&mut self) {
        while matches!(
            self.input.get(self.index),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.bump();
        }
    }

    fn literal(&mut self, text: &[u8], value: *mut json_t) -> Result<*mut json_t, ()> {
        if self.input.get(self.index..self.index + text.len()) == Some(text) {
            for _ in text {
                self.bump();
            }
            Ok(value)
        } else {
            self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid token")
        }
    }

    fn hex4(&mut self) -> Result<u16, ()> {
        let mut value = 0u16;
        for _ in 0..4 {
            let Some(byte) = self.bump() else {
                return self.fail(JSON_ERROR_PREMATURE_END, "premature end of input");
            };
            let Some(digit) = (byte as char).to_digit(16) else {
                return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid Unicode escape");
            };
            value = (value << 4) | digit as u16;
        }
        Ok(value)
    }

    fn string_bytes(&mut self) -> Result<Vec<u8>, ()> {
        if self.bump() != Some(b'"') {
            return self.fail(JSON_ERROR_INVALID_SYNTAX, "expected string");
        }
        let mut output = Vec::new();
        loop {
            let Some(byte) = self.bump() else {
                return self.fail(JSON_ERROR_PREMATURE_END, "premature end of input");
            };
            match byte {
                b'"' => break,
                0..=0x1f => {
                    return self.fail(JSON_ERROR_INVALID_SYNTAX, "control character 0x00-0x1F");
                }
                b'\\' => {
                    let Some(escape) = self.bump() else {
                        return self.fail(JSON_ERROR_PREMATURE_END, "premature end of input");
                    };
                    match escape {
                        b'"' | b'\\' | b'/' => output.push(escape),
                        b'b' => output.push(8),
                        b'f' => output.push(12),
                        b'n' => output.push(b'\n'),
                        b'r' => output.push(b'\r'),
                        b't' => output.push(b'\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let codepoint = if (0xd800..=0xdbff).contains(&first) {
                                if self.bump() != Some(b'\\') || self.bump() != Some(b'u') {
                                    return self.fail(
                                        JSON_ERROR_INVALID_SYNTAX,
                                        &format!(
                                            "invalid Unicode '\\u{first:04X}' near '\"\\u{first:04X}\"'"
                                        ),
                                    );
                                }
                                let second = self.hex4()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return self.fail(
                                        JSON_ERROR_INVALID_SYNTAX,
                                        &format!(
                                            "invalid Unicode '\\u{first:04X}' near '\"\\u{first:04X}\"'"
                                        ),
                                    );
                                }
                                0x10000
                                    + (((first as u32 - 0xd800) << 10) | (second as u32 - 0xdc00))
                            } else if (0xdc00..=0xdfff).contains(&first) {
                                return self.fail(
                                    JSON_ERROR_INVALID_SYNTAX,
                                    &format!(
                                        "invalid Unicode '\\u{first:04X}' near '\"\\u{first:04X}\"'"
                                    ),
                                );
                            } else {
                                first as u32
                            };
                            let Some(character) = char::from_u32(codepoint) else {
                                return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid Unicode");
                            };
                            let mut buffer = [0; 4];
                            output.extend_from_slice(character.encode_utf8(&mut buffer).as_bytes());
                        }
                        _ => {
                            return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid escape");
                        }
                    }
                }
                _ => output.push(byte),
            }
        }
        if !crate::private::utf8_valid(&output) {
            return self.fail(JSON_ERROR_INVALID_UTF8, "unable to decode byte 0x00");
        }
        Ok(output)
    }

    fn number(&mut self) -> Result<*mut json_t, ()> {
        let start = self.index;
        if self.input.get(self.index) == Some(&b'-') {
            self.bump();
        }
        match self.input.get(self.index) {
            Some(b'0') => {
                self.bump();
                if matches!(self.input.get(self.index), Some(b'0'..=b'9')) {
                    return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid number");
                }
            }
            Some(b'1'..=b'9') => {
                while matches!(self.input.get(self.index), Some(b'0'..=b'9')) {
                    self.bump();
                }
            }
            _ => return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid number"),
        }
        let mut real = false;
        if self.input.get(self.index) == Some(&b'.') {
            real = true;
            self.bump();
            if !matches!(self.input.get(self.index), Some(b'0'..=b'9')) {
                return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid number");
            }
            while matches!(self.input.get(self.index), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        if matches!(self.input.get(self.index), Some(b'e' | b'E')) {
            real = true;
            self.bump();
            if matches!(self.input.get(self.index), Some(b'+' | b'-')) {
                self.bump();
            }
            if !matches!(self.input.get(self.index), Some(b'0'..=b'9')) {
                return self.fail(JSON_ERROR_INVALID_SYNTAX, "invalid number");
            }
            while matches!(self.input.get(self.index), Some(b'0'..=b'9')) {
                self.bump();
            }
        }
        let text = unsafe { std::str::from_utf8_unchecked(&self.input[start..self.index]) };
        if real || self.flags & JSON_DECODE_INT_AS_REAL != 0 {
            let Ok(value) = text.parse::<f64>() else {
                return self.fail(JSON_ERROR_NUMERIC_OVERFLOW, "real number overflow");
            };
            if !value.is_finite() {
                return self.fail(JSON_ERROR_NUMERIC_OVERFLOW, "real number overflow");
            }
            Ok(unsafe { json_real(value) })
        } else {
            let Ok(value) = text.parse::<i64>() else {
                return self.fail(JSON_ERROR_NUMERIC_OVERFLOW, "too big integer");
            };
            Ok(unsafe { json_integer(value) })
        }
    }

    fn array(&mut self) -> Result<*mut json_t, ()> {
        self.bump();
        let result = unsafe { json_array() };
        self.whitespace();
        if self.input.get(self.index) == Some(&b']') {
            self.bump();
            return Ok(result);
        }
        loop {
            self.whitespace();
            let value = match self.value() {
                Ok(value) => value,
                Err(()) => {
                    unsafe { decref(result) };
                    return Err(());
                }
            };
            unsafe { json_array_append_new(result, value) };
            self.whitespace();
            match self.bump() {
                Some(b']') => return Ok(result),
                Some(b',') => {}
                Some(_) => {
                    unsafe { decref(result) };
                    return self.fail(JSON_ERROR_INVALID_SYNTAX, "expected ',' or ']'");
                }
                None => {
                    unsafe { decref(result) };
                    return self.fail(JSON_ERROR_PREMATURE_END, "premature end of input");
                }
            }
        }
    }

    fn object(&mut self) -> Result<*mut json_t, ()> {
        self.bump();
        let result = unsafe { json_object() };
        self.whitespace();
        if self.input.get(self.index) == Some(&b'}') {
            self.bump();
            return Ok(result);
        }
        loop {
            self.whitespace();
            if self.input.get(self.index) != Some(&b'"') {
                unsafe { decref(result) };
                return self.fail(JSON_ERROR_INVALID_SYNTAX, "string or '}' expected");
            }
            let key = self.string_bytes()?;
            if key.contains(&0) {
                unsafe { decref(result) };
                return self.fail(
                    JSON_ERROR_NULL_BYTE_IN_KEY,
                    "\\u0000 is not allowed in object keys",
                );
            }
            self.whitespace();
            if self.bump() != Some(b':') {
                unsafe { decref(result) };
                return self.fail(JSON_ERROR_INVALID_SYNTAX, "':' expected");
            }
            self.whitespace();
            let value = match self.value() {
                Ok(value) => value,
                Err(()) => {
                    unsafe { decref(result) };
                    return Err(());
                }
            };
            unsafe {
                if self.flags & JSON_REJECT_DUPLICATES != 0
                    && !json_object_getn(result, key.as_ptr().cast(), key.len()).is_null()
                {
                    decref(value);
                    decref(result);
                    let key_text = String::from_utf8_lossy(&key);
                    let message = format!("duplicate object key near '\"{key_text}\"'");
                    set_error(
                        self.error,
                        self.line,
                        self.column - 2,
                        self.index.saturating_sub(2),
                        JSON_ERROR_DUPLICATE_KEY,
                        &message,
                    );
                    return Err(());
                }
                json_object_setn_new_nocheck(result, key.as_ptr().cast(), key.len(), value);
            }
            self.whitespace();
            match self.bump() {
                Some(b'}') => return Ok(result),
                Some(b',') => {}
                Some(_) => {
                    unsafe { decref(result) };
                    return self.fail(JSON_ERROR_INVALID_SYNTAX, "expected ',' or '}'");
                }
                None => {
                    unsafe { decref(result) };
                    return self.fail(JSON_ERROR_PREMATURE_END, "premature end of input");
                }
            }
        }
    }

    fn value(&mut self) -> Result<*mut json_t, ()> {
        self.whitespace();
        if self.depth >= 2048 {
            return self.fail(2, "maximum parsing depth reached");
        }
        self.depth += 1;
        let result = match self.input.get(self.index) {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => {
                let bytes = self.string_bytes()?;
                if bytes.contains(&0) && self.flags & JSON_ALLOW_NUL == 0 {
                    self.fail(
                        JSON_ERROR_NULL_CHARACTER,
                        "\\u0000 is not allowed without JSON_ALLOW_NUL near '\"\\u0000\"'",
                    )
                } else {
                    Ok(unsafe { json_stringn_nocheck(bytes.as_ptr().cast(), bytes.len()) })
                }
            }
            Some(b't') => self.literal(b"true", json_true()),
            Some(b'f') => self.literal(b"false", json_false()),
            Some(b'n') => self.literal(b"null", json_null()),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(&byte) => {
                self.bump();
                self.fail(
                    JSON_ERROR_INVALID_SYNTAX,
                    &format!("unexpected token near '{}'", byte as char),
                )
            }
            None => self.fail(JSON_ERROR_PREMATURE_END, "premature end of input"),
        };
        self.depth -= 1;
        result
    }
}

unsafe fn load_bytes(
    input: &[u8],
    flags: usize,
    error: *mut json_error_t,
    source: *const c_char,
) -> *mut json_t {
    jsonp_error_init(error, source);
    if input.contains(&0) && flags & JSON_ALLOW_NUL == 0 {
        set_error(
            error,
            1,
            1,
            0,
            JSON_ERROR_NULL_CHARACTER,
            "NUL byte in input",
        );
        return ptr::null_mut();
    }
    let mut parser = Parser {
        input,
        index: 0,
        line: 1,
        column: 0,
        flags,
        error,
        depth: 0,
    };
    let Ok(value) = parser.value() else {
        return ptr::null_mut();
    };
    if flags & JSON_DECODE_ANY == 0 && !matches!(type_of(value), Some(JSON_OBJECT | JSON_ARRAY)) {
        decref(value);
        let near = String::from_utf8_lossy(&input[..parser.index]);
        set_error(
            error,
            parser.line,
            parser.column,
            parser.index,
            JSON_ERROR_INVALID_SYNTAX,
            &format!("'[' or '{{' expected near '{near}'"),
        );
        return ptr::null_mut();
    }
    parser.whitespace();
    if flags & JSON_DISABLE_EOF_CHECK == 0 && parser.index != input.len() {
        decref(value);
        let remaining = String::from_utf8_lossy(&input[parser.index..]);
        set_error(
            error,
            parser.line,
            input.len() as c_int,
            input.len(),
            JSON_ERROR_END_EXPECTED,
            &format!("end of file expected near '{remaining}'"),
        );
        return ptr::null_mut();
    }
    value
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loads(
    input: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    if input.is_null() {
        jsonp_error_init(error, c"<string>".as_ptr());
        set_error(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            "wrong arguments",
        );
        return ptr::null_mut();
    }
    let bytes = CStr::from_ptr(input).to_bytes();
    load_bytes(bytes, flags, error, c"<string>".as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadb(
    buffer: *const c_char,
    length: usize,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    if buffer.is_null() {
        jsonp_error_init(error, c"<buffer>".as_ptr());
        set_error(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            "wrong arguments",
        );
        return ptr::null_mut();
    }
    load_bytes(
        std::slice::from_raw_parts(buffer.cast(), length),
        flags,
        error,
        c"<buffer>".as_ptr(),
    )
}

unsafe fn read_file(file: *mut libc::FILE) -> Option<Vec<u8>> {
    if file.is_null() {
        return None;
    }
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = libc::fread(buffer.as_mut_ptr().cast(), 1, buffer.len(), file);
        output.extend_from_slice(&buffer[..count]);
        if count < buffer.len() {
            break;
        }
    }
    Some(output)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadf(
    input: *mut libc::FILE,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let Some(bytes) = read_file(input) else {
        jsonp_error_init(error, c"<stream>".as_ptr());
        set_error(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            "wrong arguments",
        );
        return ptr::null_mut();
    };
    load_bytes(&bytes, flags, error, c"<stream>".as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_loadfd(
    input: c_int,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = libc::read(input, buffer.as_mut_ptr().cast(), buffer.len());
        if count < 0 {
            jsonp_error_init(error, c"<stream>".as_ptr());
            set_error(
                error,
                -1,
                -1,
                0,
                JSON_ERROR_INVALID_ARGUMENT,
                "unable to read",
            );
            return ptr::null_mut();
        }
        if count == 0 {
            break;
        }
        output.extend_from_slice(&buffer[..count as usize]);
    }
    load_bytes(&output, flags, error, c"<stream>".as_ptr())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_file(
    path: *const c_char,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    if path.is_null() {
        jsonp_error_init(error, ptr::null());
        set_error(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            "wrong arguments",
        );
        return ptr::null_mut();
    }
    let file = libc::fopen(path, c"rb".as_ptr());
    if file.is_null() {
        jsonp_error_init(error, path);
        set_error(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_CANNOT_OPEN_FILE,
            "unable to open file",
        );
        return ptr::null_mut();
    }
    let result = json_loadf(file, flags, error);
    jsonp_error_set_source(error, path);
    libc::fclose(file);
    result
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn json_load_callback(
    callback: json_load_callback_t,
    data: *mut c_void,
    flags: usize,
    error: *mut json_error_t,
) -> *mut json_t {
    let Some(callback) = callback else {
        jsonp_error_init(error, c"<callback>".as_ptr());
        set_error(
            error,
            -1,
            -1,
            0,
            JSON_ERROR_INVALID_ARGUMENT,
            "wrong arguments",
        );
        return ptr::null_mut();
    };
    let mut output = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let count = callback(buffer.as_mut_ptr().cast(), buffer.len(), data);
        if count > buffer.len() {
            jsonp_error_init(error, c"<callback>".as_ptr());
            set_error(
                error,
                -1,
                -1,
                0,
                JSON_ERROR_INVALID_ARGUMENT,
                "callback failed",
            );
            return ptr::null_mut();
        }
        output.extend_from_slice(&buffer[..count]);
        if count == 0 {
            break;
        }
    }
    load_bytes(&output, flags, error, c"<callback>".as_ptr())
}
