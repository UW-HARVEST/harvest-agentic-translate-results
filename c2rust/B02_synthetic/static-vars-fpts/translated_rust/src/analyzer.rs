extern "C" {
    static mut stderr: *mut _IO_FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn printf(__format: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strcpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strstr(
        __haystack: *const ::core::ffi::c_char,
        __needle: *const ::core::ffi::c_char,
    ) -> *mut ::core::ffi::c_char;
}
pub type size_t = usize;
pub type token_type_t = ::core::ffi::c_uint;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct token_t {
    pub type_0: token_type_t,
    pub value: [::core::ffi::c_char; 256],
    pub length: size_t,
    pub line: ::core::ffi::c_int,
    pub column: ::core::ffi::c_int,
}
pub type tokenizer_next_fn = Option<unsafe extern "C" fn() -> token_t>;
pub type tokenizer_peek_fn = Option<unsafe extern "C" fn() -> token_t>;
pub type tokenizer_reset_fn = Option<unsafe extern "C" fn() -> ()>;
pub type tokenizer_load_fn =
    Option<unsafe extern "C" fn(*const ::core::ffi::c_char) -> ::core::ffi::c_int>;
pub type tokenizer_get_stats_fn =
    Option<unsafe extern "C" fn(*mut size_t, *mut size_t, *mut size_t) -> ()>;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tokenizer_ops_t {
    pub next_token: tokenizer_next_fn,
    pub peek_token: tokenizer_peek_fn,
    pub reset: tokenizer_reset_fn,
    pub load_text: tokenizer_load_fn,
    pub get_stats: tokenizer_get_stats_fn,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct analysis_result_t {
    pub word_count: size_t,
    pub number_count: size_t,
    pub keyword_count: size_t,
    pub operator_count: size_t,
    pub comment_count: size_t,
    pub string_count: size_t,
    pub line_count: size_t,
    pub char_count: size_t,
}
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
pub type __off64_t = ::core::ffi::c_long;
pub type _IO_lock_t = ();
pub type __off_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_marker {
    pub _next: *mut _IO_marker,
    pub _sbuf: *mut _IO_FILE,
    pub _pos: ::core::ffi::c_int,
}
pub type FILE = _IO_FILE;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const MAX_TOKEN_LENGTH: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
static mut tokenizer_ops: tokenizer_ops_t = tokenizer_ops_t {
    next_token: None,
    peek_token: None,
    reset: None,
    load_text: None,
    get_stats: None,
};
static mut initialized: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut token_type_counts: [::core::ffi::c_int; 20] = [0; 20];
static mut common_words: [[::core::ffi::c_char; 256]; 100] = [[0; 256]; 100];
static mut common_word_counts: [::core::ffi::c_int; 100] = [0; 100];
static mut num_common_words: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[no_mangle]
pub unsafe extern "C" fn analyzer_init(mut ops: tokenizer_ops_t) {
    tokenizer_ops = ops;
    initialized = 1 as ::core::ffi::c_int;
    memset(
        &raw mut token_type_counts as *mut ::core::ffi::c_int as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[::core::ffi::c_int; 20]>() as size_t,
    );
    memset(
        &raw mut common_word_counts as *mut ::core::ffi::c_int as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[::core::ffi::c_int; 100]>() as size_t,
    );
    num_common_words = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn track_word(mut word: *const ::core::ffi::c_char) {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < num_common_words {
        if strcmp(
            &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256]).offset(i as isize)
                as *mut ::core::ffi::c_char,
            word,
        ) == 0 as ::core::ffi::c_int
        {
            common_word_counts[i as usize] += 1;
            return;
        }
        i += 1;
    }
    if num_common_words < 100 as ::core::ffi::c_int {
        strncpy(
            &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256])
                .offset(num_common_words as isize) as *mut ::core::ffi::c_char,
            word,
            (MAX_TOKEN_LENGTH - 1 as ::core::ffi::c_int) as size_t,
        );
        common_words[num_common_words as usize]
            [(MAX_TOKEN_LENGTH - 1 as ::core::ffi::c_int) as usize] =
            '\0' as i32 as ::core::ffi::c_char;
        common_word_counts[num_common_words as usize] = 1 as ::core::ffi::c_int;
        num_common_words += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn analyze_text(mut text: *const ::core::ffi::c_char) -> analysis_result_t {
    let mut result: analysis_result_t = analysis_result_t {
        word_count: 0 as size_t,
        number_count: 0,
        keyword_count: 0,
        operator_count: 0,
        comment_count: 0,
        string_count: 0,
        line_count: 0,
        char_count: 0,
    };
    if initialized == 0 {
        fprintf(
            stderr as *mut FILE,
            b"Error: Analyzer not initialized\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return result;
    }
    if tokenizer_ops.load_text.expect("non-null function pointer")(text) != 0 as ::core::ffi::c_int
    {
        fprintf(
            stderr as *mut FILE,
            b"Error: Failed to load text\n\0" as *const u8 as *const ::core::ffi::c_char,
        );
        return result;
    }
    let mut token: token_t = token_t {
        type_0: TOKEN_EOF,
        value: [0; 256],
        length: 0,
        line: 0,
        column: 0,
    };
    loop {
        token = tokenizer_ops.next_token.expect("non-null function pointer")();
        if !(token.type_0 as ::core::ffi::c_uint
            != TOKEN_EOF as ::core::ffi::c_int as ::core::ffi::c_uint)
        {
            break;
        }
        token_type_counts[token.type_0 as usize] += 1;
        match token.type_0 as ::core::ffi::c_uint {
            1 | 6 => {
                result.word_count = result.word_count.wrapping_add(1);
                track_word(&raw mut token.value as *mut ::core::ffi::c_char);
            }
            2 => {
                result.number_count = result.number_count.wrapping_add(1);
            }
            7 => {
                result.keyword_count = result.keyword_count.wrapping_add(1);
            }
            8 => {
                result.operator_count = result.operator_count.wrapping_add(1);
            }
            10 => {
                result.comment_count = result.comment_count.wrapping_add(1);
            }
            9 => {
                result.string_count = result.string_count.wrapping_add(1);
            }
            5 => {
                result.line_count = result.line_count.wrapping_add(1);
            }
            _ => {}
        }
    }
    let mut lines: size_t = 0;
    let mut tokens: size_t = 0;
    let mut chars: size_t = 0;
    tokenizer_ops.get_stats.expect("non-null function pointer")(
        &raw mut lines,
        &raw mut tokens,
        &raw mut chars,
    );
    result.line_count = lines;
    result.char_count = chars;
    return result;
}
#[no_mangle]
pub unsafe extern "C" fn print_token_distribution() {
    printf(b"\n=== Token Distribution ===\n\0" as *const u8 as *const ::core::ffi::c_char);
    let mut token_names: [*const ::core::ffi::c_char; 12] = [
        b"EOF\0" as *const u8 as *const ::core::ffi::c_char,
        b"WORD\0" as *const u8 as *const ::core::ffi::c_char,
        b"NUMBER\0" as *const u8 as *const ::core::ffi::c_char,
        b"PUNCTUATION\0" as *const u8 as *const ::core::ffi::c_char,
        b"WHITESPACE\0" as *const u8 as *const ::core::ffi::c_char,
        b"NEWLINE\0" as *const u8 as *const ::core::ffi::c_char,
        b"IDENTIFIER\0" as *const u8 as *const ::core::ffi::c_char,
        b"KEYWORD\0" as *const u8 as *const ::core::ffi::c_char,
        b"OPERATOR\0" as *const u8 as *const ::core::ffi::c_char,
        b"STRING\0" as *const u8 as *const ::core::ffi::c_char,
        b"COMMENT\0" as *const u8 as *const ::core::ffi::c_char,
        b"ERROR\0" as *const u8 as *const ::core::ffi::c_char,
    ];
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < 12 as ::core::ffi::c_int {
        if token_type_counts[i as usize] > 0 as ::core::ffi::c_int {
            printf(
                b"%s: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
                token_names[i as usize],
                token_type_counts[i as usize],
            );
        }
        i += 1;
    }
    printf(b"\n=== Most Common Words ===\n\0" as *const u8 as *const ::core::ffi::c_char);
    let mut i_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_0 < num_common_words - 1 as ::core::ffi::c_int {
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < num_common_words - i_0 - 1 as ::core::ffi::c_int {
            if common_word_counts[j as usize]
                < common_word_counts[(j + 1 as ::core::ffi::c_int) as usize]
            {
                let mut temp_count: ::core::ffi::c_int = common_word_counts[j as usize];
                common_word_counts[j as usize] =
                    common_word_counts[(j + 1 as ::core::ffi::c_int) as usize];
                common_word_counts[(j + 1 as ::core::ffi::c_int) as usize] = temp_count;
                let mut temp_word: [::core::ffi::c_char; 256] = [0; 256];
                strcpy(
                    &raw mut temp_word as *mut ::core::ffi::c_char,
                    &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256])
                        .offset(j as isize) as *mut ::core::ffi::c_char,
                );
                strcpy(
                    &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256])
                        .offset(j as isize) as *mut ::core::ffi::c_char,
                    &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256])
                        .offset((j + 1 as ::core::ffi::c_int) as isize)
                        as *mut ::core::ffi::c_char,
                );
                strcpy(
                    &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256])
                        .offset((j + 1 as ::core::ffi::c_int) as isize)
                        as *mut ::core::ffi::c_char,
                    &raw mut temp_word as *mut ::core::ffi::c_char,
                );
            }
            j += 1;
        }
        i_0 += 1;
    }
    let mut limit: ::core::ffi::c_int = if num_common_words < 10 as ::core::ffi::c_int {
        num_common_words
    } else {
        10 as ::core::ffi::c_int
    };
    let mut i_1: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i_1 < limit {
        printf(
            b"%d. %s: %d times\n\0" as *const u8 as *const ::core::ffi::c_char,
            i_1 + 1 as ::core::ffi::c_int,
            &raw mut *(&raw mut common_words as *mut [::core::ffi::c_char; 256])
                .offset(i_1 as isize) as *mut ::core::ffi::c_char,
            common_word_counts[i_1 as usize],
        );
        i_1 += 1;
    }
}
#[no_mangle]
pub unsafe extern "C" fn calculate_complexity_score() -> ::core::ffi::c_int {
    let mut score: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    score +=
        token_type_counts[TOKEN_KEYWORD as ::core::ffi::c_int as usize] * 2 as ::core::ffi::c_int;
    score += token_type_counts[TOKEN_OPERATOR as ::core::ffi::c_int as usize];
    score += token_type_counts[TOKEN_PUNCTUATION as ::core::ffi::c_int as usize]
        / 10 as ::core::ffi::c_int;
    score -= token_type_counts[TOKEN_COMMENT as ::core::ffi::c_int as usize];
    if score < 0 as ::core::ffi::c_int {
        score = 0 as ::core::ffi::c_int;
    }
    return score;
}
#[no_mangle]
pub unsafe extern "C" fn find_patterns(mut pattern: *const ::core::ffi::c_char) {
    if initialized == 0 || pattern.is_null() {
        return;
    }
    printf(
        b"\n=== Searching for pattern: '%s' ===\n\0" as *const u8 as *const ::core::ffi::c_char,
        pattern,
    );
    tokenizer_ops.reset.expect("non-null function pointer")();
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut token: token_t = token_t {
        type_0: TOKEN_EOF,
        value: [0; 256],
        length: 0,
        line: 0,
        column: 0,
    };
    loop {
        token = tokenizer_ops.next_token.expect("non-null function pointer")();
        if !(token.type_0 as ::core::ffi::c_uint
            != TOKEN_EOF as ::core::ffi::c_int as ::core::ffi::c_uint)
        {
            break;
        }
        if !strstr(&raw mut token.value as *mut ::core::ffi::c_char, pattern).is_null() {
            printf(
                b"Line %d, Column %d: %s\n\0" as *const u8 as *const ::core::ffi::c_char,
                token.line,
                token.column,
                &raw mut token.value as *mut ::core::ffi::c_char,
            );
            count += 1;
        }
    }
    printf(
        b"Found %d occurrences\n\0" as *const u8 as *const ::core::ffi::c_char,
        count,
    );
}
