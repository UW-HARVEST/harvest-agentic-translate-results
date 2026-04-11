extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const libc::c_char,
        ...
    ) -> libc::c_int;
    fn strncpy(
        __dest: *mut libc::c_char,
        __src: *const libc::c_char,
        __n: size_t,
    ) -> *mut libc::c_char;
    fn strcmp(
        __s1: *const libc::c_char,
        __s2: *const libc::c_char,
    ) -> libc::c_int;
    fn strchr(__s: *const libc::c_char, __c: libc::c_int)
        -> *mut libc::c_char;
    fn strlen(__s: *const libc::c_char) -> size_t;
    fn __ctype_b_loc() -> *mut *const libc::c_ushort;
}
pub use crate::src::analyzer::size_t;
pub use crate::src::analyzer::token_type_t;
pub const TOKEN_ERROR: token_type_t = 11;
pub const TOKEN_COMMENT: token_type_t = 10;
pub const TOKEN_STRING: token_type_t = 9;
pub const TOKEN_OPERATOR: token_type_t = 8;
pub const TOKEN_KEYWORD: token_type_t = 7;
pub const TOKEN_IDENTIFIER: token_type_t = 6;
pub const TOKEN_NEWLINE: token_type_t = 5;
pub const TOKEN_WHITESPACE: token_type_t = 4;
pub const TOKEN_PUNCTUATION: token_type_t = 3;
pub const TOKEN_NUMBER: token_type_t = 2;
pub const TOKEN_WORD: token_type_t = 1;
pub const TOKEN_EOF: token_type_t = 0;
// #[derive(Copy, Clone)]

pub use crate::src::analyzer::token_t;
pub use crate::src::analyzer::tokenizer_next_fn;
pub use crate::src::analyzer::tokenizer_peek_fn;
pub use crate::src::analyzer::tokenizer_reset_fn;
pub use crate::src::analyzer::tokenizer_load_fn;
pub use crate::src::analyzer::tokenizer_get_stats_fn;
// #[derive(Copy, Clone)]

pub use crate::src::analyzer::tokenizer_ops_t;
// #[derive(Copy, Clone)]

pub use crate::src::analyzer::_IO_FILE;
pub use crate::src::analyzer::__off64_t;
pub use crate::src::analyzer::_IO_lock_t;
pub use crate::src::analyzer::__off_t;
// #[derive(Copy, Clone)]

pub use crate::src::analyzer::_IO_marker;
pub use crate::src::analyzer::FILE;
pub const _ISdigit: C2RustUnnamed = 2048;
pub const _ISalnum: C2RustUnnamed = 8;
pub const _ISalpha: C2RustUnnamed = 1024;
pub const _ISspace: C2RustUnnamed = 8192;
pub type C2RustUnnamed = libc::c_uint;
pub const _ISpunct: C2RustUnnamed = 4;
pub const _IScntrl: C2RustUnnamed = 2;
pub const _ISblank: C2RustUnnamed = 1;
pub const _ISgraph: C2RustUnnamed = 32768;
pub const _ISprint: C2RustUnnamed = 16384;
pub const _ISxdigit: C2RustUnnamed = 4096;
pub const _ISlower: C2RustUnnamed = 512;
pub const _ISupper: C2RustUnnamed = 256;
pub const NULL: *mut libc::c_void = std::ptr::null_mut::<libc::c_void>();
pub const MAX_TOKEN_LENGTH: libc::c_int = 256 as libc::c_int;
pub const MAX_BUFFER_SIZE: libc::c_int = 8192 as libc::c_int;
static mut input_buffer: [libc::c_char; 8192] = [0; 8192];
static mut buffer_length: size_t = 0 as size_t;
static mut current_position: size_t = 0 as size_t;
static mut current_line: libc::c_int = 1 as libc::c_int;
static mut current_column: libc::c_int = 1 as libc::c_int;
static mut total_tokens_processed: size_t = 0 as size_t;
static mut total_lines_processed: size_t = 0 as size_t;
static mut total_chars_processed: size_t = 0 as size_t;
static mut lookahead_token: token_t = token_t {
    type_0: TOKEN_EOF,
    value: [0; 256],
    length: 0,
    line: 0,
    column: 0,
};
static mut lookahead_valid: libc::c_int = 0 as libc::c_int;
static mut keywords: [*const libc::c_char; 31] = [
    b"if\0" as *const u8 as *const libc::c_char,
    b"else\0" as *const u8 as *const libc::c_char,
    b"while\0" as *const u8 as *const libc::c_char,
    b"for\0" as *const u8 as *const libc::c_char,
    b"return\0" as *const u8 as *const libc::c_char,
    b"int\0" as *const u8 as *const libc::c_char,
    b"char\0" as *const u8 as *const libc::c_char,
    b"float\0" as *const u8 as *const libc::c_char,
    b"double\0" as *const u8 as *const libc::c_char,
    b"void\0" as *const u8 as *const libc::c_char,
    b"struct\0" as *const u8 as *const libc::c_char,
    b"typedef\0" as *const u8 as *const libc::c_char,
    b"const\0" as *const u8 as *const libc::c_char,
    b"static\0" as *const u8 as *const libc::c_char,
    b"extern\0" as *const u8 as *const libc::c_char,
    b"auto\0" as *const u8 as *const libc::c_char,
    b"register\0" as *const u8 as *const libc::c_char,
    b"sizeof\0" as *const u8 as *const libc::c_char,
    b"break\0" as *const u8 as *const libc::c_char,
    b"continue\0" as *const u8 as *const libc::c_char,
    b"switch\0" as *const u8 as *const libc::c_char,
    b"case\0" as *const u8 as *const libc::c_char,
    b"default\0" as *const u8 as *const libc::c_char,
    b"do\0" as *const u8 as *const libc::c_char,
    b"goto\0" as *const u8 as *const libc::c_char,
    b"enum\0" as *const u8 as *const libc::c_char,
    b"union\0" as *const u8 as *const libc::c_char,
    b"signed\0" as *const u8 as *const libc::c_char,
    b"unsigned\0" as *const u8 as *const libc::c_char,
    b"long\0" as *const u8 as *const libc::c_char,
    b"short\0" as *const u8 as *const libc::c_char,
];
static mut num_keywords: libc::c_int = 0;
unsafe extern "C" fn is_keyword(mut str: *const libc::c_char) -> libc::c_int {
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < num_keywords {
        if strcmp(str, keywords[i as usize]) == 0 as libc::c_int {
            return 1 as libc::c_int;
        }
        i += 1;
    }
    return 0 as libc::c_int;
}
unsafe extern "C" fn peek_char() -> libc::c_char {
    if current_position >= buffer_length {
        return '\0' as i32 as libc::c_char;
    }
    return input_buffer[current_position as usize];
}
unsafe extern "C" fn advance_char() -> libc::c_char {
    if current_position >= buffer_length {
        return '\0' as i32 as libc::c_char;
    }
    let fresh0 = current_position;
    current_position = current_position.wrapping_add(1);
    let mut c: libc::c_char = input_buffer[fresh0 as usize];
    total_chars_processed = total_chars_processed.wrapping_add(1);
    if c as libc::c_int == '\n' as i32 {
        current_line += 1;
        current_column = 1 as libc::c_int;
        total_lines_processed = total_lines_processed.wrapping_add(1);
    } else {
        current_column += 1;
    }
    return c;
}
unsafe extern "C" fn skip_whitespace() {
    while peek_char() as libc::c_int != '\0' as i32
        && *(*__ctype_b_loc()).offset(peek_char() as libc::c_int as isize)
            as libc::c_int
            & _ISspace as libc::c_int as libc::c_ushort as libc::c_int
            != 0
        && peek_char() as libc::c_int != '\n' as i32
    {
        advance_char();
    }
}
unsafe extern "C" fn create_token(
    mut type_0: token_type_t,
    mut value: *const libc::c_char,
    mut length: size_t,
) -> token_t {
    let mut token: token_t = token_t {
        type_0: TOKEN_EOF,
        value: [0; 256],
        length: 0,
        line: 0,
        column: 0,
    };
    token.type_0 = type_0;
    token.length = if length < MAX_TOKEN_LENGTH as size_t {
        length
    } else {
        (MAX_TOKEN_LENGTH - 1 as libc::c_int) as size_t
    };
    strncpy(
        &raw mut token.value as *mut libc::c_char,
        value,
        token.length,
    );
    token.value[token.length as usize] = '\0' as i32 as libc::c_char;
    token.line = current_line;
    token.column = (current_column as size_t).wrapping_sub(token.length) as libc::c_int;
    total_tokens_processed = total_tokens_processed.wrapping_add(1);
    return token;
}
unsafe extern "C" fn scan_word() -> token_t {
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut length: size_t = 0 as size_t;
    while peek_char() as libc::c_int != '\0' as i32
        && (*(*__ctype_b_loc()).offset(peek_char() as libc::c_int as isize)
            as libc::c_int
            & _ISalnum as libc::c_int as libc::c_ushort as libc::c_int
            != 0
            || peek_char() as libc::c_int == '_' as i32)
        && length < (MAX_TOKEN_LENGTH - 1 as libc::c_int) as size_t
    {
        let fresh16 = length;
        length = length.wrapping_add(1);
        buffer[fresh16 as usize] = advance_char();
    }
    buffer[length as usize] = '\0' as i32 as libc::c_char;
    if is_keyword(&raw mut buffer as *mut libc::c_char) != 0 {
        return create_token(
            TOKEN_KEYWORD,
            &raw mut buffer as *mut libc::c_char,
            length,
        );
    }
    return create_token(
        TOKEN_IDENTIFIER,
        &raw mut buffer as *mut libc::c_char,
        length,
    );
}
unsafe extern "C" fn scan_number() -> token_t {
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut length: size_t = 0 as size_t;
    let mut has_decimal: libc::c_int = 0 as libc::c_int;
    while peek_char() as libc::c_int != '\0' as i32
        && (*(*__ctype_b_loc()).offset(peek_char() as libc::c_int as isize)
            as libc::c_int
            & _ISdigit as libc::c_int as libc::c_ushort as libc::c_int
            != 0
            || peek_char() as libc::c_int == '.' as i32)
        && length < (MAX_TOKEN_LENGTH - 1 as libc::c_int) as size_t
    {
        if peek_char() as libc::c_int == '.' as i32 {
            if has_decimal != 0 {
                break;
            }
            has_decimal = 1 as libc::c_int;
        }
        let fresh15 = length;
        length = length.wrapping_add(1);
        buffer[fresh15 as usize] = advance_char();
    }
    buffer[length as usize] = '\0' as i32 as libc::c_char;
    return create_token(
        TOKEN_NUMBER,
        &raw mut buffer as *mut libc::c_char,
        length,
    );
}
unsafe extern "C" fn scan_string() -> token_t {
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut length: size_t = 0 as size_t;
    let mut quote: libc::c_char = advance_char();
    let fresh10 = length;
    length = length.wrapping_add(1);
    buffer[fresh10 as usize] = quote;
    while peek_char() as libc::c_int != '\0' as i32
        && peek_char() as libc::c_int != quote as libc::c_int
        && peek_char() as libc::c_int != '\n' as i32
        && length < (MAX_TOKEN_LENGTH - 2 as libc::c_int) as size_t
    {
        if peek_char() as libc::c_int == '\\' as i32 {
            let fresh11 = length;
            length = length.wrapping_add(1);
            buffer[fresh11 as usize] = advance_char();
            if peek_char() as libc::c_int != '\0' as i32 {
                let fresh12 = length;
                length = length.wrapping_add(1);
                buffer[fresh12 as usize] = advance_char();
            }
        } else {
            let fresh13 = length;
            length = length.wrapping_add(1);
            buffer[fresh13 as usize] = advance_char();
        }
    }
    if peek_char() as libc::c_int == quote as libc::c_int {
        let fresh14 = length;
        length = length.wrapping_add(1);
        buffer[fresh14 as usize] = advance_char();
    }
    buffer[length as usize] = '\0' as i32 as libc::c_char;
    return create_token(
        TOKEN_STRING,
        &raw mut buffer as *mut libc::c_char,
        length,
    );
}
unsafe extern "C" fn scan_comment() -> token_t {
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut length: size_t = 0 as size_t;
    let fresh3 = length;
    length = length.wrapping_add(1);
    buffer[fresh3 as usize] = advance_char();
    if peek_char() as libc::c_int == '/' as i32 {
        let fresh4 = length;
        length = length.wrapping_add(1);
        buffer[fresh4 as usize] = advance_char();
        while peek_char() as libc::c_int != '\0' as i32
            && peek_char() as libc::c_int != '\n' as i32
            && length < (MAX_TOKEN_LENGTH - 1 as libc::c_int) as size_t
        {
            let fresh5 = length;
            length = length.wrapping_add(1);
            buffer[fresh5 as usize] = advance_char();
        }
    } else if peek_char() as libc::c_int == '*' as i32 {
        let fresh6 = length;
        length = length.wrapping_add(1);
        buffer[fresh6 as usize] = advance_char();
        while peek_char() as libc::c_int != '\0' as i32
            && length < (MAX_TOKEN_LENGTH - 2 as libc::c_int) as size_t
        {
            if peek_char() as libc::c_int == '*' as i32 {
                let fresh7 = length;
                length = length.wrapping_add(1);
                buffer[fresh7 as usize] = advance_char();
                if !(peek_char() as libc::c_int == '/' as i32) {
                    continue;
                }
                let fresh8 = length;
                length = length.wrapping_add(1);
                buffer[fresh8 as usize] = advance_char();
                break;
            } else {
                let fresh9 = length;
                length = length.wrapping_add(1);
                buffer[fresh9 as usize] = advance_char();
            }
        }
    }
    buffer[length as usize] = '\0' as i32 as libc::c_char;
    return create_token(
        TOKEN_COMMENT,
        &raw mut buffer as *mut libc::c_char,
        length,
    );
}
unsafe extern "C" fn scan_operator() -> token_t {
    let mut buffer: [libc::c_char; 256] = [0; 256];
    let mut length: size_t = 0 as size_t;
    let mut c: libc::c_char = peek_char();
    let fresh1 = length;
    length = length.wrapping_add(1);
    buffer[fresh1 as usize] = advance_char();
    let mut next: libc::c_char = peek_char();
    if c as libc::c_int == '=' as i32 && next as libc::c_int == '=' as i32
        || c as libc::c_int == '!' as i32 && next as libc::c_int == '=' as i32
        || c as libc::c_int == '<' as i32 && next as libc::c_int == '=' as i32
        || c as libc::c_int == '>' as i32 && next as libc::c_int == '=' as i32
        || c as libc::c_int == '&' as i32 && next as libc::c_int == '&' as i32
        || c as libc::c_int == '|' as i32 && next as libc::c_int == '|' as i32
        || c as libc::c_int == '+' as i32 && next as libc::c_int == '+' as i32
        || c as libc::c_int == '-' as i32 && next as libc::c_int == '-' as i32
        || c as libc::c_int == '-' as i32 && next as libc::c_int == '>' as i32
        || c as libc::c_int == '<' as i32 && next as libc::c_int == '<' as i32
        || c as libc::c_int == '>' as i32 && next as libc::c_int == '>' as i32
    {
        let fresh2 = length;
        length = length.wrapping_add(1);
        buffer[fresh2 as usize] = advance_char();
    }
    buffer[length as usize] = '\0' as i32 as libc::c_char;
    return create_token(
        TOKEN_OPERATOR,
        &raw mut buffer as *mut libc::c_char,
        length,
    );
}
#[no_mangle]
pub unsafe extern "C" fn tokenizer_next_token() -> token_t {
    if lookahead_valid != 0 {
        lookahead_valid = 0 as libc::c_int;
        return lookahead_token;
    }
    skip_whitespace();
    if current_position >= buffer_length {
        return create_token(
            TOKEN_EOF,
            b"\0" as *const u8 as *const libc::c_char,
            0 as size_t,
        );
    }
    let mut c: libc::c_char = peek_char();
    if c as libc::c_int == '\n' as i32 {
        let mut newline: [libc::c_char; 2] =
            [advance_char(), '\0' as i32 as libc::c_char];
        return create_token(
            TOKEN_NEWLINE,
            &raw mut newline as *mut libc::c_char,
            1 as size_t,
        );
    }
    if *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
        & _ISalpha as libc::c_int as libc::c_ushort as libc::c_int
        != 0
        || c as libc::c_int == '_' as i32
    {
        return scan_word();
    }
    if *(*__ctype_b_loc()).offset(c as libc::c_int as isize) as libc::c_int
        & _ISdigit as libc::c_int as libc::c_ushort as libc::c_int
        != 0
    {
        return scan_number();
    }
    if c as libc::c_int == '"' as i32 || c as libc::c_int == '\'' as i32 {
        return scan_string();
    }
    if c as libc::c_int == '/' as i32
        && (peek_char() as libc::c_int == '/' as i32
            || peek_char() as libc::c_int == '*' as i32)
    {
        return scan_comment();
    }
    if !strchr(
        b"+-*/%=<>!&|^~?:\0" as *const u8 as *const libc::c_char,
        c as libc::c_int,
    )
    .is_null()
    {
        return scan_operator();
    }
    if !strchr(
        b"(){}[];,.\0" as *const u8 as *const libc::c_char,
        c as libc::c_int,
    )
    .is_null()
    {
        let mut punct: [libc::c_char; 2] =
            [advance_char(), '\0' as i32 as libc::c_char];
        return create_token(
            TOKEN_PUNCTUATION,
            &raw mut punct as *mut libc::c_char,
            1 as size_t,
        );
    }
    let mut unknown: [libc::c_char; 2] =
        [advance_char(), '\0' as i32 as libc::c_char];
    return create_token(
        TOKEN_ERROR,
        &raw mut unknown as *mut libc::c_char,
        1 as size_t,
    );
}
#[no_mangle]
pub unsafe extern "C" fn tokenizer_peek_token() -> token_t {
    if lookahead_valid == 0 {
        lookahead_token = tokenizer_next_token();
        lookahead_valid = 1 as libc::c_int;
    }
    return lookahead_token;
}
#[no_mangle]
pub unsafe extern "C" fn tokenizer_reset() {
    current_position = 0 as size_t;
    current_line = 1 as libc::c_int;
    current_column = 1 as libc::c_int;
    lookahead_valid = 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tokenizer_load_text(
    mut text: *const libc::c_char,
) -> libc::c_int {
    if text.is_null() {
        return -(1 as libc::c_int);
    }
    let mut length: size_t = strlen(text);
    if length >= MAX_BUFFER_SIZE as size_t {
        fprintf(
            stderr as *mut FILE,
            b"Error: Input text too large\n\0" as *const u8 as *const libc::c_char,
        );
        return -(1 as libc::c_int);
    }
    strncpy(
        &raw mut input_buffer as *mut libc::c_char,
        text,
        (MAX_BUFFER_SIZE - 1 as libc::c_int) as size_t,
    );
    input_buffer[(MAX_BUFFER_SIZE - 1 as libc::c_int) as usize] =
        '\0' as i32 as libc::c_char;
    buffer_length = length;
    tokenizer_reset();
    return 0 as libc::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn tokenizer_get_stats(
    mut lines: *mut size_t,
    mut tokens: *mut size_t,
    mut chars: *mut size_t,
) {
    if !lines.is_null() {
        *lines = total_lines_processed;
    }
    if !tokens.is_null() {
        *tokens = total_tokens_processed;
    }
    if !chars.is_null() {
        *chars = total_chars_processed;
    }
}
#[no_mangle]
pub extern "C" fn get_tokenizer_ops() -> tokenizer_ops_t {
    let mut ops: tokenizer_ops_t = tokenizer_ops_t {
        next_token: None,
        peek_token: None,
        reset: None,
        load_text: None,
        get_stats: None,
    };
    ops.next_token =
        Some(tokenizer_next_token as unsafe extern "C" fn() -> token_t) as tokenizer_next_fn;
    ops.peek_token =
        Some(tokenizer_peek_token as unsafe extern "C" fn() -> token_t) as tokenizer_peek_fn;
    ops.reset = Some(tokenizer_reset as unsafe extern "C" fn() -> ()) as tokenizer_reset_fn;
    ops.load_text = Some(
        tokenizer_load_text
            as unsafe extern "C" fn(*const libc::c_char) -> libc::c_int,
    ) as tokenizer_load_fn;
    ops.get_stats = Some(
        tokenizer_get_stats as unsafe extern "C" fn(*mut size_t, *mut size_t, *mut size_t) -> (),
    ) as tokenizer_get_stats_fn;
    return ops;
}
unsafe extern "C" fn run_static_initializers() {
    num_keywords = (std::mem::size_of::<[*const libc::c_char; 31]>() as usize)
        .wrapping_div(std::mem::size_of::<*const libc::c_char>() as usize)
        as libc::c_int;
}
#[used]
#[cfg_attr(target_os = "linux", link_section = ".init_array")]
#[cfg_attr(target_os = "windows", link_section = ".CRT$XIB")]
#[cfg_attr(target_os = "macos", link_section = "__DATA,__mod_init_func")]
static INIT_ARRAY: [unsafe extern "C" fn(); 1] = [run_static_initializers];
pub fn borrow<'a, 'b: 'a, T>(p: &'a Option<&'b mut T>) -> Option<&'a T> {
    p.as_ref().map(|x| &**x)
}

pub fn borrow_mut<'a, 'b : 'a, T>(p: &'a mut Option<&'b mut T>) -> Option<&'a mut T> {
    p.as_mut().map(|x| &mut **x)
}

pub fn owned_as_ref<'a, T>(p: &'a Option<Box<T>>) -> Option<&'a T> {
    p.as_ref().map(|x| x.as_ref())
}

pub fn owned_as_mut<'a, T>(p: &'a mut Option<Box<T>>) -> Option<&'a mut T> {
    p.as_mut().map(|x| x.as_mut())
}

pub fn option_to_raw<T>(p: Option<&T>) -> * const T {
    p.map_or(core::ptr::null(), |p| p as * const T)
}

pub fn _ref_eq<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) == option_to_raw(q)
}

pub fn _ref_ne<T>(p: Option<&T>, q: Option<&T>) -> bool {
    option_to_raw(p) != option_to_raw(q)
}

