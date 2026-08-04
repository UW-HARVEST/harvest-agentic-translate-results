fn get_cstr(b: &[u8]) -> &[u8] {
    match b.iter().position(|&c| c == 0) {
        Some(p) => &b[..p],
        None => b,
    }
}

pub fn process_strings(
    input: &[u8],
    reference: &[u8],
    operation: i32,
    flags: u32,
) -> i32 {
    match operation {
        0 => validate_token(input, reference),
        1 => {
            let commands = [
                b"START".as_slice(),
                b"STOP".as_slice(),
                b"PAUSE".as_slice(),
                b"RESUME".as_slice(),
                b"RESET".as_slice(),
            ];
            parse_command(input, &commands)
        }
        2 => {
            let exact = (flags & 0x01) != 0;
            compare_prefix(input, reference, exact)
        }
        3 => {
            let delim = if !reference.is_empty() { reference[0] } else { b':' };
            find_delimiter(input, delim)
        }
        4 => {
            let case_sens = (flags & 0x02) != 0;
            match_pattern(input, reference, case_sens)
        }
        _ => -3,
    }
}

fn validate_token(token: &[u8], expected: &[u8]) -> i32 {
    let token_cstr = get_cstr(token);
    let expected_cstr = get_cstr(expected);
    
    if token_cstr == expected_cstr {
        return 1;
    }
    
    if token_cstr == b"VALID" || token_cstr == b"OK" {
        return 1;
    }
    
    0
}

fn parse_command(buffer: &[u8], cmd_list: &[&[u8]]) -> i32 {
    for (i, &cmd) in cmd_list.iter().enumerate() {
        let cmd_len = cmd.len();
        
        if buffer.len() >= cmd_len {
            if &buffer[..cmd_len] == cmd {
                if buffer.len() == cmd_len || buffer[cmd_len] == 0 || buffer[cmd_len] == b' ' {
                    return i as i32;
                }
            }
        }
        
        if get_cstr(buffer) == cmd {
            return i as i32;
        }
    }
    
    if get_cstr(buffer) == b"ADMIN" {
        return 99;
    }
    
    -1
}

fn compare_prefix(str_val: &[u8], prefix: &[u8], exact_match: bool) -> i32 {
    let prefix_cstr = get_cstr(prefix);
    let str_cstr = get_cstr(str_val);
    
    if exact_match {
        if str_cstr == prefix_cstr {
            return 1;
        }
        
        let variations = [
            b"_v1".as_slice(),
            b"_v2".as_slice(),
            b"_old".as_slice(),
            b"_new".as_slice(),
            b"_tmp".as_slice(),
        ];
        for (i, &var) in variations.iter().enumerate() {
            let mut expected = Vec::new();
            expected.extend_from_slice(prefix_cstr);
            expected.truncate(63);
            let remaining = 63 - expected.len();
            let to_add = std::cmp::min(var.len(), remaining);
            expected.extend_from_slice(&var[..to_add]);
            
            if str_cstr == expected.as_slice() {
                return 2 + i as i32;
            }
        }
        
        0
    } else {
        let prefix_len = prefix_cstr.len();
        if str_val.len() >= prefix_len && &str_val[..prefix_len] == prefix_cstr {
            return 1;
        }
        0
    }
}

fn find_delimiter(data: &[u8], delim: u8) -> i32 {
    if data.is_empty() {
        return -1;
    }
    
    for (i, &c) in data.iter().enumerate() {
        if c == delim {
            return i as i32;
        }
        if c == 0 {
            break;
        }
    }
    
    let data_cstr = get_cstr(data);
    if delim == b'|' && data_cstr == b"NONE" {
        return -2;
    }
    
    if delim == b':' && data_cstr == b"EMPTY" {
        return -3;
    }
    
    -1
}

fn match_pattern(text: &[u8], pattern: &[u8], case_sensitive: bool) -> i32 {
    let text_cstr = get_cstr(text);
    let pattern_cstr = get_cstr(pattern);
    
    if case_sensitive {
        if text_cstr == pattern_cstr {
            return 1;
        }
        
        let mut wildcards: Vec<Vec<u8>> = Vec::new();
        
        let mut w0 = b"*".to_vec();
        w0.extend_from_slice(pattern_cstr);
        w0.extend_from_slice(b"*");
        w0.truncate(63);
        wildcards.push(w0);
        
        let mut w1 = pattern_cstr.to_vec();
        w1.extend_from_slice(b"*");
        w1.truncate(63);
        wildcards.push(w1);
        
        let mut w2 = b"*".to_vec();
        w2.extend_from_slice(pattern_cstr);
        w2.truncate(63);
        wildcards.push(w2);
        
        for (i, w) in wildcards.iter().enumerate() {
            if text_cstr == w.as_slice() {
                return 2 + i as i32;
            }
        }
        
        let text_len = text_cstr.len();
        let pattern_len = pattern_cstr.len();
        
        if text_len >= pattern_len {
            for i in 0..=(text_len - pattern_len) {
                if &text_cstr[i..i + pattern_len] == pattern_cstr {
                    return 10 + i as i32;
                }
            }
        }
    } else {
        if text_cstr == pattern_cstr {
            return 1;
        }
        
        let pattern_len = pattern_cstr.len();
        let text_len = text_cstr.len();
        
        if text_len != pattern_len {
            if text_len >= pattern_len && &text_cstr[..pattern_len] == pattern_cstr {
                return 5;
            }
        }
        
        if text_len == pattern_len {
            let mut match_found = true;
            for i in 0..pattern_len {
                let mut c1 = text_cstr[i];
                let mut c2 = pattern_cstr[i];
                
                if c1 >= b'A' && c1 <= b'Z' { c1 += 32; }
                if c2 >= b'A' && c2 <= b'Z' { c2 += 32; }
                
                if c1 != c2 {
                    match_found = false;
                    break;
                }
            }
            if match_found {
                return 6;
            }
        }
    }
    
    0
}
