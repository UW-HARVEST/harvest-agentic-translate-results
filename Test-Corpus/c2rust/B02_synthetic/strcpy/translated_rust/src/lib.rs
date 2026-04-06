extern "C" {
    fn strncpy(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strncat(
        __dest: *mut ::core::ffi::c_char,
        __src: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> *mut ::core::ffi::c_char;
    fn strcmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
    ) -> ::core::ffi::c_int;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn snprintf(
        __s: *mut ::core::ffi::c_char,
        __maxlen: size_t,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
}
pub type size_t = usize;
pub type __uint32_t = u32;
pub type uint32_t = __uint32_t;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[no_mangle]
pub unsafe extern "C" fn process_strings(
    mut input: *mut ::core::ffi::c_char,
    mut input_len: size_t,
    mut reference: *const ::core::ffi::c_char,
    mut ref_len: size_t,
    mut operation: ::core::ffi::c_int,
    mut flags: uint32_t,
) -> ::core::ffi::c_int {
    if input.is_null() {
        return -(1 as ::core::ffi::c_int);
    }
    match operation {
        0 => {
            if reference.is_null() {
                return -(2 as ::core::ffi::c_int);
            }
            return validate_token(input, reference);
        }
        1 => {
            let mut commands: [*const ::core::ffi::c_char; 5] = [
                b"START\0" as *const u8 as *const ::core::ffi::c_char,
                b"STOP\0" as *const u8 as *const ::core::ffi::c_char,
                b"PAUSE\0" as *const u8 as *const ::core::ffi::c_char,
                b"RESUME\0" as *const u8 as *const ::core::ffi::c_char,
                b"RESET\0" as *const u8 as *const ::core::ffi::c_char,
            ];
            return parse_command(
                input,
                input_len,
                &raw mut commands as *mut *const ::core::ffi::c_char,
                5 as ::core::ffi::c_int,
            );
        }
        2 => {
            if reference.is_null() {
                return -(2 as ::core::ffi::c_int);
            }
            let mut exact: ::core::ffi::c_int = (flags & 0x1 as uint32_t) as ::core::ffi::c_int;
            return compare_prefix(input, reference, exact);
        }
        3 => {
            let mut delim: ::core::ffi::c_char = (if !reference.is_null() && ref_len > 0 as size_t {
                *reference.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            } else {
                ':' as i32
            }) as ::core::ffi::c_char;
            return find_delimiter(input, input_len, delim);
        }
        4 => {
            if reference.is_null() {
                return -(2 as ::core::ffi::c_int);
            }
            let mut case_sens: ::core::ffi::c_int = (flags & 0x2 as uint32_t) as ::core::ffi::c_int;
            return match_pattern(input, reference, case_sens);
        }
        _ => return -(3 as ::core::ffi::c_int),
    };
}
unsafe extern "C" fn validate_token(
    mut token: *const ::core::ffi::c_char,
    mut expected: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if strcmp(token, expected) == 0 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if strcmp(token, b"VALID\0" as *const u8 as *const ::core::ffi::c_char)
        == 0 as ::core::ffi::c_int
        || strcmp(token, b"OK\0" as *const u8 as *const ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn parse_command(
    mut buffer: *mut ::core::ffi::c_char,
    mut buf_size: size_t,
    mut cmd_list: *mut *const ::core::ffi::c_char,
    mut list_size: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < list_size {
        let mut cmd_len: size_t = strlen(*cmd_list.offset(i as isize));
        if buf_size >= cmd_len {
            if strncmp(buffer, *cmd_list.offset(i as isize), cmd_len) == 0 as ::core::ffi::c_int {
                if *buffer.offset(cmd_len as isize) as ::core::ffi::c_int == '\0' as i32
                    || *buffer.offset(cmd_len as isize) as ::core::ffi::c_int == ' ' as i32
                {
                    return i;
                }
            }
        }
        if strcmp(buffer, *cmd_list.offset(i as isize)) == 0 as ::core::ffi::c_int {
            return i;
        }
        i += 1;
    }
    if strcmp(
        buffer,
        b"ADMIN\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0 as ::core::ffi::c_int
    {
        return 99 as ::core::ffi::c_int;
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn compare_prefix(
    mut str: *const ::core::ffi::c_char,
    mut prefix: *const ::core::ffi::c_char,
    mut exact_match: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut prefix_len: size_t = strlen(prefix);
    if exact_match != 0 {
        if strcmp(str, prefix) == 0 as ::core::ffi::c_int {
            return 1 as ::core::ffi::c_int;
        }
        let mut variations: [[::core::ffi::c_char; 32]; 5] = [
            ::core::mem::transmute::<[u8; 32], [::core::ffi::c_char; 32]>(
                *b"_v1\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            ::core::mem::transmute::<[u8; 32], [::core::ffi::c_char; 32]>(
                *b"_v2\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            ::core::mem::transmute::<[u8; 32], [::core::ffi::c_char; 32]>(
                *b"_old\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            ::core::mem::transmute::<[u8; 32], [::core::ffi::c_char; 32]>(
                *b"_new\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
            ::core::mem::transmute::<[u8; 32], [::core::ffi::c_char; 32]>(
                *b"_tmp\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
            ),
        ];
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 5 as ::core::ffi::c_int {
            let mut expected: [::core::ffi::c_char; 64] = [0; 64];
            strncpy(
                &raw mut expected as *mut ::core::ffi::c_char,
                prefix,
                63 as size_t,
            );
            expected[63 as ::core::ffi::c_int as usize] = '\0' as i32 as ::core::ffi::c_char;
            strncat(
                &raw mut expected as *mut ::core::ffi::c_char,
                &raw mut *(&raw mut variations as *mut [::core::ffi::c_char; 32]).offset(i as isize)
                    as *mut ::core::ffi::c_char,
                (63 as size_t).wrapping_sub(strlen(&raw mut expected as *mut ::core::ffi::c_char)),
            );
            if strcmp(str, &raw mut expected as *mut ::core::ffi::c_char) == 0 as ::core::ffi::c_int
            {
                return 2 as ::core::ffi::c_int + i;
            }
            i += 1;
        }
        return 0 as ::core::ffi::c_int;
    } else {
        if strncmp(str, prefix, prefix_len) == 0 as ::core::ffi::c_int {
            return 1 as ::core::ffi::c_int;
        }
        return 0 as ::core::ffi::c_int;
    };
}
unsafe extern "C" fn find_delimiter(
    mut data: *const ::core::ffi::c_char,
    mut len: size_t,
    mut delim: ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if len == 0 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    let mut i: size_t = 0 as size_t;
    while i < len {
        if *data.offset(i as isize) as ::core::ffi::c_int == delim as ::core::ffi::c_int {
            return i as ::core::ffi::c_int;
        }
        if *data.offset(i as isize) as ::core::ffi::c_int == '\0' as i32 {
            break;
        }
        i = i.wrapping_add(1);
    }
    if delim as ::core::ffi::c_int == '|' as i32
        && strcmp(data, b"NONE\0" as *const u8 as *const ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
    {
        return -(2 as ::core::ffi::c_int);
    }
    if delim as ::core::ffi::c_int == ':' as i32
        && strcmp(data, b"EMPTY\0" as *const u8 as *const ::core::ffi::c_char)
            == 0 as ::core::ffi::c_int
    {
        return -(3 as ::core::ffi::c_int);
    }
    return -(1 as ::core::ffi::c_int);
}
unsafe extern "C" fn match_pattern(
    mut text: *const ::core::ffi::c_char,
    mut pattern: *const ::core::ffi::c_char,
    mut case_sensitive: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if case_sensitive != 0 {
        if strcmp(text, pattern) == 0 as ::core::ffi::c_int {
            return 1 as ::core::ffi::c_int;
        }
        let mut wildcard_patterns: [[::core::ffi::c_char; 64]; 3] = [[0; 64]; 3];
        snprintf(
            &raw mut *(&raw mut wildcard_patterns as *mut [::core::ffi::c_char; 64])
                .offset(0 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
            64 as size_t,
            b"*%s*\0" as *const u8 as *const ::core::ffi::c_char,
            pattern,
        );
        snprintf(
            &raw mut *(&raw mut wildcard_patterns as *mut [::core::ffi::c_char; 64])
                .offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
            64 as size_t,
            b"%s*\0" as *const u8 as *const ::core::ffi::c_char,
            pattern,
        );
        snprintf(
            &raw mut *(&raw mut wildcard_patterns as *mut [::core::ffi::c_char; 64])
                .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_char,
            64 as size_t,
            b"*%s\0" as *const u8 as *const ::core::ffi::c_char,
            pattern,
        );
        let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while i < 3 as ::core::ffi::c_int {
            if strcmp(
                text,
                &raw mut *(&raw mut wildcard_patterns as *mut [::core::ffi::c_char; 64])
                    .offset(i as isize) as *mut ::core::ffi::c_char,
            ) == 0 as ::core::ffi::c_int
            {
                return 2 as ::core::ffi::c_int + i;
            }
            i += 1;
        }
        let mut text_len: size_t = strlen(text);
        let mut pattern_len: size_t = strlen(pattern);
        let mut i_0: size_t = 0 as size_t;
        while i_0 <= text_len.wrapping_sub(pattern_len) {
            if strncmp(
                text.offset(i_0 as isize) as *const ::core::ffi::c_char,
                pattern,
                pattern_len,
            ) == 0 as ::core::ffi::c_int
            {
                return (10 as size_t).wrapping_add(i_0) as ::core::ffi::c_int;
            }
            i_0 = i_0.wrapping_add(1);
        }
    } else {
        if strcmp(text, pattern) == 0 as ::core::ffi::c_int {
            return 1 as ::core::ffi::c_int;
        }
        let mut pattern_len_0: size_t = strlen(pattern);
        let mut text_len_0: size_t = strlen(text);
        if text_len_0 != pattern_len_0 {
            if strncmp(text, pattern, pattern_len_0) == 0 as ::core::ffi::c_int {
                return 5 as ::core::ffi::c_int;
            }
        }
        if text_len_0 == pattern_len_0 {
            let mut match_0: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
            let mut i_1: size_t = 0 as size_t;
            while i_1 < pattern_len_0 {
                let mut c1: ::core::ffi::c_char = *text.offset(i_1 as isize);
                let mut c2: ::core::ffi::c_char = *pattern.offset(i_1 as isize);
                if c1 as ::core::ffi::c_int >= 'A' as i32 && c1 as ::core::ffi::c_int <= 'Z' as i32
                {
                    c1 = (c1 as ::core::ffi::c_int + 32 as ::core::ffi::c_int)
                        as ::core::ffi::c_char;
                }
                if c2 as ::core::ffi::c_int >= 'A' as i32 && c2 as ::core::ffi::c_int <= 'Z' as i32
                {
                    c2 = (c2 as ::core::ffi::c_int + 32 as ::core::ffi::c_int)
                        as ::core::ffi::c_char;
                }
                if c1 as ::core::ffi::c_int != c2 as ::core::ffi::c_int {
                    match_0 = 0 as ::core::ffi::c_int;
                    break;
                } else {
                    i_1 = i_1.wrapping_add(1);
                }
            }
            if match_0 != 0 {
                return 6 as ::core::ffi::c_int;
            }
        }
    }
    return 0 as ::core::ffi::c_int;
}
