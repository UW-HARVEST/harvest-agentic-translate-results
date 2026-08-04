// Translation of c_src/src/tokenizer.c
//
// This module reproduces the exact behavior of the C tokenizer, including its
// static (file-local) global state. We use `static mut` to replicate the C
// program's single-threaded, file-local state. Public functions mirror the C
// API and use `extern "C"` and `#[unsafe(no_mangle)]` so the cdylib exports the
// same linker symbols.

use core::ffi::{c_char, c_int};
use libc::{c_void, fprintf, isalnum, isalpha, isdigit, isspace, size_t, strchr, strcmp, strlen, strncpy};

pub const MAX_TOKEN_LENGTH: usize = 256;
pub const MAX_BUFFER_SIZE: usize = 8192;

// `token_type_t` from tokenizer.h.
// In C this is a plain `enum`, which on x86_64 Linux is `int`-sized (4 bytes).
#[repr(C)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct TokenType(pub c_int);

pub const TOKEN_EOF: TokenType = TokenType(0);
pub const TOKEN_WORD: TokenType = TokenType(1);
pub const TOKEN_NUMBER: TokenType = TokenType(2);
pub const TOKEN_PUNCTUATION: TokenType = TokenType(3);
pub const TOKEN_WHITESPACE: TokenType = TokenType(4);
pub const TOKEN_NEWLINE: TokenType = TokenType(5);
pub const TOKEN_IDENTIFIER: TokenType = TokenType(6);
pub const TOKEN_KEYWORD: TokenType = TokenType(7);
pub const TOKEN_OPERATOR: TokenType = TokenType(8);
pub const TOKEN_STRING: TokenType = TokenType(9);
pub const TOKEN_COMMENT: TokenType = TokenType(10);
pub const TOKEN_ERROR: TokenType = TokenType(11);

// `token_t` from tokenizer.h. Layout must match the C struct exactly because
// instances are passed and returned by value across the FFI boundary.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Token {
    pub r#type: TokenType,
    pub value: [c_char; MAX_TOKEN_LENGTH],
    pub length: size_t,
    pub line: c_int,
    pub column: c_int,
}

impl Token {
    const fn zero() -> Self {
        Token {
            r#type: TokenType(0),
            value: [0; MAX_TOKEN_LENGTH],
            length: 0,
            line: 0,
            column: 0,
        }
    }
}

// Function pointer types, matching the typedefs in tokenizer.h.
pub type TokenizerNextFn = extern "C" fn() -> Token;
pub type TokenizerPeekFn = extern "C" fn() -> Token;
pub type TokenizerResetFn = extern "C" fn();
pub type TokenizerLoadFn = extern "C" fn(*const c_char) -> c_int;
pub type TokenizerGetStatsFn = extern "C" fn(*mut size_t, *mut size_t, *mut size_t);

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TokenizerOps {
    pub next_token: TokenizerNextFn,
    pub peek_token: TokenizerPeekFn,
    pub reset: TokenizerResetFn,
    pub load_text: TokenizerLoadFn,
    pub get_stats: TokenizerGetStatsFn,
}

// Static (file-local) state from tokenizer.c.
static mut INPUT_BUFFER: [c_char; MAX_BUFFER_SIZE] = [0; MAX_BUFFER_SIZE];
static mut BUFFER_LENGTH: size_t = 0;
static mut CURRENT_POSITION: size_t = 0;
static mut CURRENT_LINE: c_int = 1;
static mut CURRENT_COLUMN: c_int = 1;
static mut TOTAL_TOKENS_PROCESSED: size_t = 0;
static mut TOTAL_LINES_PROCESSED: size_t = 0;
static mut TOTAL_CHARS_PROCESSED: size_t = 0;
static mut LOOKAHEAD_TOKEN: Token = Token::zero();
static mut LOOKAHEAD_VALID: c_int = 0;

// Keywords list, matching `keywords[]` in tokenizer.c.
static KEYWORDS: &[&[u8]] = &[
    b"if\0",
    b"else\0",
    b"while\0",
    b"for\0",
    b"return\0",
    b"int\0",
    b"char\0",
    b"float\0",
    b"double\0",
    b"void\0",
    b"struct\0",
    b"typedef\0",
    b"const\0",
    b"static\0",
    b"extern\0",
    b"auto\0",
    b"register\0",
    b"sizeof\0",
    b"break\0",
    b"continue\0",
    b"switch\0",
    b"case\0",
    b"default\0",
    b"do\0",
    b"goto\0",
    b"enum\0",
    b"union\0",
    b"signed\0",
    b"unsigned\0",
    b"long\0",
    b"short\0",
];

unsafe fn is_keyword(s: *const c_char) -> c_int {
    for kw in KEYWORDS.iter() {
        if strcmp(s, kw.as_ptr() as *const c_char) == 0 {
            return 1;
        }
    }
    0
}

unsafe fn peek_char() -> c_char {
    if CURRENT_POSITION >= BUFFER_LENGTH {
        return 0;
    }
    INPUT_BUFFER[CURRENT_POSITION as usize]
}

unsafe fn advance_char() -> c_char {
    if CURRENT_POSITION >= BUFFER_LENGTH {
        return 0;
    }
    let c = INPUT_BUFFER[CURRENT_POSITION as usize];
    CURRENT_POSITION += 1;
    TOTAL_CHARS_PROCESSED += 1;

    if c == b'\n' as c_char {
        CURRENT_LINE += 1;
        CURRENT_COLUMN = 1;
        TOTAL_LINES_PROCESSED += 1;
    } else {
        CURRENT_COLUMN += 1;
    }

    c
}

unsafe fn skip_whitespace() {
    while peek_char() != 0
        && isspace(peek_char() as c_int) != 0
        && peek_char() != b'\n' as c_char
    {
        advance_char();
    }
}

unsafe fn create_token(t: TokenType, value: *const c_char, length: size_t) -> Token {
    let mut token = Token::zero();
    token.r#type = t;
    token.length = if (length as usize) < MAX_TOKEN_LENGTH {
        length
    } else {
        (MAX_TOKEN_LENGTH - 1) as size_t
    };
    strncpy(token.value.as_mut_ptr(), value, token.length as usize);
    token.value[token.length as usize] = 0;
    token.line = CURRENT_LINE;
    token.column = CURRENT_COLUMN - token.length as c_int;
    TOTAL_TOKENS_PROCESSED += 1;
    token
}

unsafe fn scan_word() -> Token {
    let mut buffer: [c_char; MAX_TOKEN_LENGTH] = [0; MAX_TOKEN_LENGTH];
    let mut length: size_t = 0;

    while peek_char() != 0
        && (isalnum(peek_char() as c_int) != 0 || peek_char() == b'_' as c_char)
        && (length as usize) < MAX_TOKEN_LENGTH - 1
    {
        buffer[length as usize] = advance_char();
        length += 1;
    }
    buffer[length as usize] = 0;

    if is_keyword(buffer.as_ptr()) != 0 {
        return create_token(TOKEN_KEYWORD, buffer.as_ptr(), length);
    }
    create_token(TOKEN_IDENTIFIER, buffer.as_ptr(), length)
}

unsafe fn scan_number() -> Token {
    let mut buffer: [c_char; MAX_TOKEN_LENGTH] = [0; MAX_TOKEN_LENGTH];
    let mut length: size_t = 0;
    let mut has_decimal = 0;

    while peek_char() != 0
        && (isdigit(peek_char() as c_int) != 0 || peek_char() == b'.' as c_char)
        && (length as usize) < MAX_TOKEN_LENGTH - 1
    {
        if peek_char() == b'.' as c_char {
            if has_decimal != 0 {
                break;
            }
            has_decimal = 1;
        }
        buffer[length as usize] = advance_char();
        length += 1;
    }
    buffer[length as usize] = 0;
    create_token(TOKEN_NUMBER, buffer.as_ptr(), length)
}

unsafe fn scan_string() -> Token {
    let mut buffer: [c_char; MAX_TOKEN_LENGTH] = [0; MAX_TOKEN_LENGTH];
    let mut length: size_t = 0;
    let quote = advance_char();

    buffer[length as usize] = quote;
    length += 1;

    while peek_char() != 0
        && peek_char() != quote
        && peek_char() != b'\n' as c_char
        && (length as usize) < MAX_TOKEN_LENGTH - 2
    {
        if peek_char() == b'\\' as c_char {
            buffer[length as usize] = advance_char();
            length += 1;
            if peek_char() != 0 {
                buffer[length as usize] = advance_char();
                length += 1;
            }
        } else {
            buffer[length as usize] = advance_char();
            length += 1;
        }
    }

    if peek_char() == quote {
        buffer[length as usize] = advance_char();
        length += 1;
    }

    buffer[length as usize] = 0;
    create_token(TOKEN_STRING, buffer.as_ptr(), length)
}

unsafe fn scan_comment() -> Token {
    let mut buffer: [c_char; MAX_TOKEN_LENGTH] = [0; MAX_TOKEN_LENGTH];
    let mut length: size_t = 0;

    // Assume we've seen '/'
    buffer[length as usize] = advance_char();
    length += 1;

    if peek_char() == b'/' as c_char {
        // Single-line comment
        buffer[length as usize] = advance_char();
        length += 1;

        while peek_char() != 0
            && peek_char() != b'\n' as c_char
            && (length as usize) < MAX_TOKEN_LENGTH - 1
        {
            buffer[length as usize] = advance_char();
            length += 1;
        }
    } else if peek_char() == b'*' as c_char {
        // Multi-line comment
        buffer[length as usize] = advance_char();
        length += 1;

        while peek_char() != 0 && (length as usize) < MAX_TOKEN_LENGTH - 2 {
            if peek_char() == b'*' as c_char {
                buffer[length as usize] = advance_char();
                length += 1;
                if peek_char() == b'/' as c_char {
                    buffer[length as usize] = advance_char();
                    length += 1;
                    break;
                }
            } else {
                buffer[length as usize] = advance_char();
                length += 1;
            }
        }
    }

    buffer[length as usize] = 0;
    create_token(TOKEN_COMMENT, buffer.as_ptr(), length)
}

unsafe fn scan_operator() -> Token {
    let mut buffer: [c_char; MAX_TOKEN_LENGTH] = [0; MAX_TOKEN_LENGTH];
    let mut length: size_t = 0;
    let c = peek_char();

    buffer[length as usize] = advance_char();
    length += 1;

    let next = peek_char();
    let two_char = (c == b'=' as c_char && next == b'=' as c_char)
        || (c == b'!' as c_char && next == b'=' as c_char)
        || (c == b'<' as c_char && next == b'=' as c_char)
        || (c == b'>' as c_char && next == b'=' as c_char)
        || (c == b'&' as c_char && next == b'&' as c_char)
        || (c == b'|' as c_char && next == b'|' as c_char)
        || (c == b'+' as c_char && next == b'+' as c_char)
        || (c == b'-' as c_char && next == b'-' as c_char)
        || (c == b'-' as c_char && next == b'>' as c_char)
        || (c == b'<' as c_char && next == b'<' as c_char)
        || (c == b'>' as c_char && next == b'>' as c_char);
    if two_char {
        buffer[length as usize] = advance_char();
        length += 1;
    }

    buffer[length as usize] = 0;
    create_token(TOKEN_OPERATOR, buffer.as_ptr(), length)
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_next_token() -> Token {
    unsafe {
        if LOOKAHEAD_VALID != 0 {
            LOOKAHEAD_VALID = 0;
            return LOOKAHEAD_TOKEN;
        }

        skip_whitespace();

        if CURRENT_POSITION >= BUFFER_LENGTH {
            let empty: [c_char; 1] = [0];
            return create_token(TOKEN_EOF, empty.as_ptr(), 0);
        }

        let c = peek_char();

        // Newline
        if c == b'\n' as c_char {
            let newline: [c_char; 2] = [advance_char(), 0];
            return create_token(TOKEN_NEWLINE, newline.as_ptr(), 1);
        }

        // Identifier or keyword
        if isalpha(c as c_int) != 0 || c == b'_' as c_char {
            return scan_word();
        }

        // Number
        if isdigit(c as c_int) != 0 {
            return scan_number();
        }

        // String
        if c == b'"' as c_char || c == b'\'' as c_char {
            return scan_string();
        }

        // Comment
        if c == b'/' as c_char
            && (peek_char() == b'/' as c_char || peek_char() == b'*' as c_char)
        {
            return scan_comment();
        }

        // Operator
        let op_chars = b"+-*/%=<>!&|^~?:\0";
        if !strchr(op_chars.as_ptr() as *const c_char, c as c_int).is_null() {
            return scan_operator();
        }

        // Punctuation
        let punct_chars = b"(){}[];,.\0";
        if !strchr(punct_chars.as_ptr() as *const c_char, c as c_int).is_null() {
            let punct: [c_char; 2] = [advance_char(), 0];
            return create_token(TOKEN_PUNCTUATION, punct.as_ptr(), 1);
        }

        // Unknown character
        let unknown: [c_char; 2] = [advance_char(), 0];
        create_token(TOKEN_ERROR, unknown.as_ptr(), 1)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_peek_token() -> Token {
    unsafe {
        if LOOKAHEAD_VALID == 0 {
            LOOKAHEAD_TOKEN = tokenizer_next_token();
            LOOKAHEAD_VALID = 1;
        }
        LOOKAHEAD_TOKEN
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_reset() {
    unsafe {
        CURRENT_POSITION = 0;
        CURRENT_LINE = 1;
        CURRENT_COLUMN = 1;
        LOOKAHEAD_VALID = 0;
        // Note: We don't reset total statistics
    }
}

extern "C" {
    static stderr: *mut libc::FILE;
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_load_text(text: *const c_char) -> c_int {
    unsafe {
        if text.is_null() {
            return -1;
        }

        let length = strlen(text);
        if length >= MAX_BUFFER_SIZE as size_t {
            let msg = b"Error: Input text too large\n\0";
            fprintf(stderr, msg.as_ptr() as *const c_char);
            return -1;
        }

        strncpy(
            INPUT_BUFFER.as_mut_ptr(),
            text,
            (MAX_BUFFER_SIZE - 1) as usize,
        );
        INPUT_BUFFER[MAX_BUFFER_SIZE - 1] = 0;
        BUFFER_LENGTH = length;

        tokenizer_reset();

        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tokenizer_get_stats(
    lines: *mut size_t,
    tokens: *mut size_t,
    chars: *mut size_t,
) {
    unsafe {
        if !lines.is_null() {
            *lines = TOTAL_LINES_PROCESSED;
        }
        if !tokens.is_null() {
            *tokens = TOTAL_TOKENS_PROCESSED;
        }
        if !chars.is_null() {
            *chars = TOTAL_CHARS_PROCESSED;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn get_tokenizer_ops() -> TokenizerOps {
    TokenizerOps {
        next_token: tokenizer_next_token,
        peek_token: tokenizer_peek_token,
        reset: tokenizer_reset,
        load_text: tokenizer_load_text,
        get_stats: tokenizer_get_stats,
    }
}

// Helper used by the analyzer module to silence the unused-import warning
// for `c_void` in some configurations.
#[allow(dead_code)]
fn _force_link() -> *const c_void {
    core::ptr::null()
}
