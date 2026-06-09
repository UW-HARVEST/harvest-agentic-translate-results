// Translation of c_src/src/lib.c

const REPLACEMENT_INC: usize = 4096;

/// Single byte: 0xxxxxxx
fn valid_1(b: &[u8]) -> bool {
    (b[0] & 0x80) == 0
}

/// Two bytes: 110xxxxx 10xxxxxx
/// Starting bytes 0xC0 and 0xC1 are forbidden (overlong)
fn valid_2(b: &[u8]) -> bool {
    if b.len() < 2 {
        return false;
    }
    (b[0] & 0xE0) == 0xC0
        && b[0] >= 0xC2
        && (b[1] & 0xC0) == 0x80
}

/// Three bytes: 1110xxxx 10xxxxxx 10xxxxxx
/// 0xE0 could start overlong encodings
/// 0xED (range U+D800-U+DFFF) is reserved for UTF-16 surrogate halves
fn valid_3(b: &[u8]) -> bool {
    if b.len() < 3 {
        return false;
    }
    (b[0] & 0xF0) == 0xE0
        && (b[1] & 0xC0) == 0x80
        && (b[2] & 0xC0) == 0x80
        && (b[0] != 0xE0 || b[1] >= 0xA0)
        && (b[0] != 0xED || b[1] < 0xA0)
        && (b[0] != 0xEF || b[1] <= 0xBF)
}

/// Four bytes: 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
/// 0xF0 could start overlong encodings
/// Start bytes 0xF5 and above are invalid for UTF-8
fn valid_4(b: &[u8]) -> bool {
    if b.len() < 4 {
        return false;
    }
    (b[0] & 0xF8) == 0xF0
        && b[0] <= 0xF4
        && (b[1] & 0xC0) == 0x80
        && (b[2] & 0xC0) == 0x80
        && (b[3] & 0xC0) == 0x80
        && (b[0] != 0xF0 || b[1] >= 0x90)
        && (b[0] != 0xF4 || b[1] <= 0x8F)
}

/// Return index of first byte that does not match UTF-8, or end-of-string index
pub fn w_utf8_drop(string: &[u8]) -> usize {
    let mut i = 0;
    while i < string.len() && string[i] != 0 {
        let rest = &string[i..];
        if valid_1(rest) {
            i += 1;
        } else if valid_2(rest) {
            i += 2;
        } else if valid_3(rest) {
            i += 3;
        } else if valid_4(rest) {
            i += 4;
        } else {
            return i;
        }
    }
    i
}

/// Filter invalid UTF-8 from a NUL-terminated byte string. If `replacement` is
/// true, replace invalid bytes with the U+FFFD replacement character (in UTF-8:
/// 0xEF 0xBF 0xBD). Returns a Vec<u8> NOT containing the trailing NUL.
pub fn w_utf8_filter(string: &[u8], replacement: bool) -> Vec<u8> {
    // Find length up to NUL terminator (mimic C string semantics)
    let strlen = string.iter().position(|&b| b == 0).unwrap_or(string.len());
    let s = &string[..strlen];

    let valid_idx = w_utf8_drop(s);

    if valid_idx == s.len() {
        // Equivalent to strdup of input
        return s.to_vec();
    }

    let mut size = strlen + 1;
    let mut copy: Vec<u8> = vec![0u8; size];
    let mut i = valid_idx;
    let mut repl: usize = 0;

    copy[..i].copy_from_slice(&s[..i]);

    let mut valid = valid_idx;
    while valid < s.len() {
        let rest = &s[valid..];
        if valid_1(rest) {
            copy[i] = s[valid];
            i += 1;
            valid += 1;
        } else if valid_2(rest) {
            copy[i] = s[valid];
            i += 1;
            valid += 1;
            copy[i] = s[valid];
            i += 1;
            valid += 1;
        } else if valid_3(rest) {
            copy[i] = s[valid];
            i += 1;
            valid += 1;
            copy[i] = s[valid];
            i += 1;
            valid += 1;
            copy[i] = s[valid];
            i += 1;
            valid += 1;
        } else if valid_4(rest) {
            copy[i] = s[valid];
            i += 1;
            valid += 1;
            copy[i] = s[valid];
            i += 1;
            valid += 1;
            copy[i] = s[valid];
            i += 1;
            valid += 1;
            copy[i] = s[valid];
            i += 1;
            valid += 1;
        } else {
            if replacement {
                if repl < 3 {
                    size += REPLACEMENT_INC;
                    copy.resize(size, 0);
                    repl += REPLACEMENT_INC;
                }

                copy[i] = 0xEF;
                i += 1;
                copy[i] = 0xBF;
                i += 1;
                copy[i] = 0xBD;
                i += 1;
                repl -= 3;
            }

            valid += 1;
        }
    }

    copy.truncate(i);
    copy
}
