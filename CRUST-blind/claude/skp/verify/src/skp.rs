/// SKP version information.
pub const SKP_VER: u32 = 0x0003001C;
pub const SKP_VER_STR: &str = "0.3.1rc";
/// A loop state used for scanning.
/// (In the C header this is defined only via macros; here we provide a Rust struct.)
#[derive(Debug, Default, Clone)]
pub struct SkpLoop {
    pub start: String,
    pub to: Option<String>,
    pub end: Option<String>,
    pub alt: i32,
}
/// Returns the “length” from start to to. (This mimics the inline function `skp_loop_len`.)
pub fn skp_loop_len(start: &str, to: &str) -> i32 {
    let s_ptr = start.as_ptr() as isize;
    let t_ptr = to.as_ptr() as isize;
    let ret = (t_ptr - s_ptr) as i32;
    if ret >= 0 && ret <= (1 << 16) {
        ret
    } else {
        0
    }
}
/// Global variable used in the C code.
/// (In C declared as `volatile int skp_zero;`—here we use a mutable static.)
pub static mut SKP_ZERO: i32 = 0;
/// Trace function (corresponds to the C macro skptrace).
pub fn skptrace(args: std::fmt::Arguments) {
    eprintln!("TRCE: {}", args);
}

// ==================================================================
// Helper utilities (byte-based, internal)
// ==================================================================

/// Return the number of bytes in s up to (but not including) the first NUL byte,
/// or s.len() if no NUL byte is present.  Mirrors C's null-terminated semantics.
fn effective_len(s: &str) -> usize {
    let bytes = s.as_bytes();
    bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())
}

/// Read the next packed UTF-8 codepoint at `idx` from byte slice `bytes`.
/// `iso` non-zero means treat each byte independently (no UTF-8 grouping).
/// Returns (codepoint_packed_into_u32, byte_index_just_after).
fn skp_next_idx(bytes: &[u8], idx: usize, iso: bool) -> (u32, usize) {
    if idx >= bytes.len() || bytes[idx] == 0 {
        return (0, idx);
    }
    let mut c: u32 = bytes[idx] as u32;
    let mut i = idx + 1;
    if !iso {
        // Read up to 3 continuation bytes
        for _ in 0..3 {
            if i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                c = (c << 8) | (bytes[i] as u32);
                i += 1;
            } else {
                break;
            }
        }
    }
    if c == 0x0D && i < bytes.len() && bytes[i] == 0x0A {
        c = 0x0D0A;
        i += 1;
    }
    (c, i)
}

// ==================================================================
// Public character helpers
// ==================================================================

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, i) = skp_next_idx(bytes, 0, iso != 0);
    let i = i.min(bytes.len());
    // Try to slice on a UTF-8 boundary.  If `i` lands inside a multi-byte
    // character (only possible for malformed input), fall back to ASCII view.
    let rest = if s.is_char_boundary(i) {
        &s[i..]
    } else {
        // Should not happen for valid UTF-8 inputs, but guard anyway.
        &s[bytes.len()..]
    };
    (c, rest)
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        if (b'A' as u32..=b'Z' as u32).contains(&a) {
            a += 0x20;
        }
        if (b'A' as u32..=b'Z' as u32).contains(&b) {
            b += 0x20;
        }
    }
    a == b
}

/// Returns true if `c` is a blank character.
pub fn is_blank(c: u32) -> bool {
    if c < 0xFF {
        return c == 0x20 || c == 0x09;
    }
    match c & 0xFFFFFF00 {
        0x00000000 => c == 0xA0,
        0x0000C200 => c == 0xC2A0,
        0x00E19A00 => c == 0xE19A80,
        0x00E28000 => (0xE28080..=0xE2808A).contains(&c) || c == 0xE280AF,
        0x00E38080 => c == 0xE38080,
        _ => false,
    }
}

/// Returns true if `c` is a line-break character.
pub fn is_break(c: u32) -> bool {
    if c < 0x0F {
        return c == 0x0A || c == 0x0C || c == 0x0D;
    }
    if c < 0xFF {
        return c == 0x85;
    }
    c == 0x0D0A || c == 0xC285 || c == 0xE280A8 || c == 0xE280A9
}

/// Returns true if `c` is a space (blank or break).
pub fn is_space(c: u32) -> bool {
    is_blank(c) || is_break(c)
}

/// Returns true if `c` is a digit.
pub fn is_digit(c: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&c)
}

/// Returns true if `c` is a hexadecimal digit.
pub fn is_xdigit(c: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&c)
        || (b'A' as u32..=b'F' as u32).contains(&c)
        || (b'a' as u32..=b'f' as u32).contains(&c)
}

/// Returns true if `c` is an uppercase letter.
pub fn is_upper(c: u32) -> bool {
    (b'A' as u32..=b'Z' as u32).contains(&c)
}

/// Returns true if `c` is a lowercase letter.
pub fn is_lower(c: u32) -> bool {
    (b'a' as u32..=b'z' as u32).contains(&c)
}

/// Returns true if `c` is an alphabetic character.
pub fn is_alpha(c: u32) -> bool {
    is_upper(c) || is_lower(c)
}

/// Returns true if `c` is a valid identifier character.
pub fn is_idchr(c: u32) -> bool {
    is_alpha(c) || is_digit(c) || c == b'_' as u32
}

/// Returns true if `c` is alphanumeric.
pub fn is_alnum(c: u32) -> bool {
    is_alpha(c) || is_digit(c)
}

/// Returns true if `c` is a control character.
pub fn is_ctrl(c: u32) -> bool {
    c < 0x20 || (0xC280..0xC2A0).contains(&c) || (0x7F..0xA0).contains(&c)
}

/// Returns true if `ch` is one of the characters in `set`. The `iso` flag is used for encoding.
pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let bytes = set.as_bytes();
    let isob = iso != 0;
    let mut s_idx = 0;
    let (mut p_ch, ns) = skp_next_idx(bytes, s_idx, isob);
    s_idx = ns;
    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let (np, ns2) = skp_next_idx(bytes, s_idx, isob);
            p_ch = np;
            s_idx = ns2;
        }
    }
    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, ns2) = skp_next_idx(bytes, s_idx, isob);
        p_ch = np;
        s_idx = ns2;
        // peek next byte
        let next_byte = if s_idx < bytes.len() { bytes[s_idx] } else { 0 };
        if p_ch == b'-' as u32 && next_byte != b']' {
            let (np2, ns3) = skp_next_idx(bytes, s_idx, isob);
            p_ch = np2;
            s_idx = ns3;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, ns4) = skp_next_idx(bytes, s_idx, isob);
            p_ch = np3;
            s_idx = ns4;
        }
    }
    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
/// Returns the number of bytes matched in s, or 0 on no match.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_idx(s.as_bytes(), 0, p.as_bytes(), 0, len, flg)
}

fn is_string_idx(
    s_bytes: &[u8],
    s_start_idx: usize,
    p_bytes: &[u8],
    p_start_idx: usize,
    len: i32,
    flg: i32,
) -> i32 {
    let isob = (flg & 2) != 0;
    let fold = flg & 1;
    let start_s = s_start_idx;
    let mut s_idx = s_start_idx;
    let mut p_idx = p_start_idx;
    let mut len = len;
    let mut mlen: i32 = 0;
    while len > 0 {
        // Hitting the alternation marker '\xE' (0x0E) means a successful alternative
        if p_idx < p_bytes.len() && p_bytes[p_idx] == 0x0E {
            return mlen;
        }
        let (p_chr, p_end_idx) = skp_next_idx(p_bytes, p_idx, isob);
        let (s_chr, s_end_idx) = skp_next_idx(s_bytes, s_idx, isob);
        if chr_cmp(s_chr, p_chr, fold) {
            mlen += (s_end_idx - s_idx) as i32;
            len -= (p_end_idx - p_idx) as i32;
            p_idx = p_end_idx;
            s_idx = s_end_idx;
        } else {
            // search for an alternative '\xE'
            while len > 0 && p_idx < p_bytes.len() {
                let b = p_bytes[p_idx];
                p_idx += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
            len -= 1;
            if len < 0 {
                return 0;
            }
            s_idx = start_s;
            mlen = 0;
        }
    }
    mlen
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    match open as u8 as char {
        '(' => ')' as u32,
        '[' => ']' as u32,
        '{' => '}' as u32,
        '<' => '>' as u32,
        _ => 0,
    }
}

/// Given a quote character, returns the corresponding closing quote.
pub fn get_qclose(open: u32) -> u32 {
    match open as u8 as char {
        '\'' | '"' | '`' => open,
        _ => 0,
    }
}

/// Constants for match results.
pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Internal byte-index based match function.
/// Returns (match_result, src_end_idx, pat_end_idx).
fn match_idx(
    p_bytes: &[u8],
    p_start_idx: usize,
    s_bytes: &[u8],
    s_start_idx: usize,
    flg: &mut i32,
) -> (i32, usize, usize) {
    let mut p_idx = p_start_idx;
    let mut s_end_idx = s_start_idx;
    let isob = (*flg & 2) != 0;
    let (mut s_chr, mut s_tmp_idx) = skp_next_idx(s_bytes, s_end_idx, isob);

    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut match_cnt: u32;
    let mut intnumber = false;
    let mut ret: i32 = MATCHED_FAIL;

    if p_idx < p_bytes.len() {
        match p_bytes[p_idx] {
            b'*' => {
                match_min = 0;
                match_max = u32::MAX;
                p_idx += 1;
            }
            b'+' => {
                match_max = u32::MAX;
                p_idx += 1;
            }
            b'?' => {
                match_min = 0;
                p_idx += 1;
            }
            _ => {}
        }
    }

    if p_idx < p_bytes.len() && p_bytes[p_idx] == b'!' {
        match_not = 1;
        p_idx += 1;
    }

    if p_idx >= p_bytes.len() {
        return (MATCHED_FAIL, s_start_idx, p_start_idx);
    }

    let opcode = p_bytes[p_idx];
    p_idx += 1;

    // Helper closure to repeat matching using skp_next-based advancement
    macro_rules! do_w {
        ($pred:expr) => {{
            let mut cnt: u32 = 0;
            while cnt < match_max && (s_chr != 0 && (($pred) != (match_not != 0))) {
                s_end_idx = s_tmp_idx;
                let (nc, nt) = skp_next_idx(s_bytes, s_end_idx, isob);
                s_chr = nc;
                s_tmp_idx = nt;
                cnt += 1;
            }
            match_cnt = cnt;
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    // Helper to advance one byte (mirrors get_next_s_chr in C)
    macro_rules! get_next_s_chr_byte {
        () => {
            s_end_idx = s_tmp_idx;
            s_chr = if s_end_idx < s_bytes.len() {
                s_bytes[s_end_idx] as u32
            } else {
                0
            };
            s_tmp_idx = s_end_idx + 1;
        };
    }

    match opcode {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                do_w!(s_chr != 0);
            }
        }
        b'$' | b'n' => {
            // For '$': if s_chr == 0, ret = 1, else fall through to 'n' handling
            // We replicate the fallthrough: for '$' if not zero, treat as 'n'
            if opcode == b'$' && s_chr == 0 {
                ret = MATCHED;
            } else {
                do_w!(is_break(s_chr));
            }
        }
        b'd' => {
            do_w!(is_digit(s_chr));
        }
        b'x' => {
            do_w!(is_xdigit(s_chr));
        }
        b'a' => {
            do_w!(is_alpha(s_chr));
        }
        b'u' => {
            do_w!(is_upper(s_chr));
        }
        b'l' => {
            do_w!(is_lower(s_chr));
        }
        b's' => {
            do_w!(is_space(s_chr));
        }
        b'w' => {
            do_w!(is_blank(s_chr));
        }
        b'c' => {
            do_w!(is_ctrl(s_chr));
        }
        b'i' => {
            do_w!(is_idchr(s_chr));
        }
        b'@' => {
            do_w!(is_alnum(s_chr));
        }
        b'&' => {
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'[' => {
            // Use a string-based slice into pat for is_oneof.
            // We can't construct &str from bytes directly without UTF-8 check, so
            // use a helper that operates on bytes.
            do_w!(is_oneof_bytes(s_chr, p_bytes, p_idx, isob));
            // advance past the set
            if p_idx < p_bytes.len() && p_bytes[p_idx] == b']' {
                p_idx += 1;
            }
            while p_idx < p_bytes.len() && p_bytes[p_idx] != 0 && p_bytes[p_idx] != b']' {
                p_idx += 1;
            }
            if p_idx < p_bytes.len() {
                p_idx += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = opcode;
            let mut l: i32 = 0;
            while p_idx + (l as usize) < p_bytes.len() && p_bytes[p_idx + l as usize] != 0 {
                if p_bytes[p_idx + l as usize] == quote {
                    break;
                }
                l += 1;
            }
            if l > 0 {
                let ml = is_string_idx(s_bytes, s_end_idx, p_bytes, p_idx, l, *flg);
                if ml > 0 {
                    if match_not == 0 {
                        s_end_idx += ml as usize;
                        ret = MATCHED;
                    }
                    // If match_not != 0 and matched, leave ret = FAIL (handled later)
                } else if match_min == 0 || match_not != 0 {
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            }
            // skip past the closing quote
            p_idx += l as usize + 1;
        }
        b'C' => {
            *flg = (*flg & !1) | (match_not as i32);
            ret = MATCHED;
        }
        b'U' => {
            *flg = (*flg & !2) | ((match_not as i32) * 2);
            ret = MATCHED;
        }
        b'S' => {
            while is_space(s_chr) {
                get_next_s_chr_byte!();
            }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) {
                get_next_s_chr_byte!();
            }
            ret = MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !is_break(s_chr) {
                get_next_s_chr_byte!();
            }
            if s_chr != 0 {
                get_next_s_chr_byte!();
            }
            ret = MATCHED;
        }
        b'I' => {
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next_s_chr_byte!();
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            // ( ) means balanced parenthesis only with '()'
            if p_idx < p_bytes.len() && p_bytes[p_idx] == b')' && s_chr == b'(' as u32 {
                p_idx += 1;
                // fall through to balanced handling
                let open = s_chr;
                let close = get_close(open);
                if close != 0 {
                    let mut count: i32 = 1;
                    while s_chr != 0 && count > 0 {
                        get_next_s_chr_byte!();
                        if s_chr == open {
                            count += 1;
                        }
                        if s_chr == close {
                            count -= 1;
                        }
                    }
                    if count == 0 {
                        get_next_s_chr_byte!();
                        ret = MATCHED;
                    }
                }
            }
        }
        b'B' => {
            let open = s_chr;
            let close = get_close(open);
            if close != 0 {
                let mut count: i32 = 1;
                while s_chr != 0 && count > 0 {
                    get_next_s_chr_byte!();
                    if s_chr == open {
                        count += 1;
                    }
                    if s_chr == close {
                        count -= 1;
                    }
                }
                if count == 0 {
                    get_next_s_chr_byte!();
                    ret = MATCHED;
                }
            }
        }
        b'Q' => {
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next_s_chr_byte!();
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == b'\\' as u32 {
                        get_next_s_chr_byte!();
                    }
                }
                if s_chr != 0 {
                    get_next_s_chr_byte!();
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            // hex number
            if s_chr == b'0' as u32
                && s_end_idx + 2 < s_bytes.len()
                && (s_bytes[s_end_idx + 1] == b'x' || s_bytes[s_end_idx + 1] == b'X')
                && is_xdigit(s_bytes[s_end_idx + 2] as u32)
            {
                get_next_s_chr_byte!();
                get_next_s_chr_byte!();
                get_next_s_chr_byte!();
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                get_next_s_chr_byte!();
            }
        }
        b'D' => {
            intnumber = true;
            // fall through to F handling
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next_s_chr_byte!();
                    if !is_space(s_chr) {
                        break;
                    }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_s_chr_byte!();
            }
            if !intnumber {
                if s_chr == b'.' as u32 {
                    get_next_s_chr_byte!();
                }
                while is_digit(s_chr) {
                    ret = MATCHED;
                    get_next_s_chr_byte!();
                }
                if ret == MATCHED && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                    get_next_s_chr_byte!();
                    if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                        get_next_s_chr_byte!();
                    }
                    while is_digit(s_chr) {
                        get_next_s_chr_byte!();
                    }
                    if s_chr == b'.' as u32 {
                        get_next_s_chr_byte!();
                    }
                    while is_digit(s_chr) {
                        get_next_s_chr_byte!();
                    }
                }
            }
        }
        b'F' => {
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next_s_chr_byte!();
                    if !is_space(s_chr) {
                        break;
                    }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_s_chr_byte!();
            }
            if s_chr == b'.' as u32 {
                get_next_s_chr_byte!();
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_s_chr_byte!();
            }
            if ret == MATCHED && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                get_next_s_chr_byte!();
                if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                    get_next_s_chr_byte!();
                }
                while is_digit(s_chr) {
                    get_next_s_chr_byte!();
                }
                if s_chr == b'.' as u32 {
                    get_next_s_chr_byte!();
                }
                while is_digit(s_chr) {
                    get_next_s_chr_byte!();
                }
            }
        }
        _ => {
            ret = MATCHED_FAIL;
            p_idx -= 1;
        }
    }

    let _ = match_cnt; // suppress warning if unused

    if ret != MATCHED_FAIL {
        (ret, s_end_idx, p_idx)
    } else {
        (MATCHED_FAIL, s_start_idx, p_start_idx)
    }
}

fn is_oneof_bytes(ch: u32, set_bytes: &[u8], set_start_idx: usize, isob: bool) -> bool {
    if ch == 0 {
        return false;
    }
    let mut s_idx = set_start_idx;
    let (mut p_ch, ns) = skp_next_idx(set_bytes, s_idx, isob);
    s_idx = ns;
    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let (np, ns2) = skp_next_idx(set_bytes, s_idx, isob);
            p_ch = np;
            s_idx = ns2;
        }
    }
    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, ns2) = skp_next_idx(set_bytes, s_idx, isob);
        p_ch = np;
        s_idx = ns2;
        let next_byte = if s_idx < set_bytes.len() {
            set_bytes[s_idx]
        } else {
            0
        };
        if p_ch == b'-' as u32 && next_byte != b']' {
            let (np2, ns3) = skp_next_idx(set_bytes, s_idx, isob);
            p_ch = np2;
            s_idx = ns3;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, ns4) = skp_next_idx(set_bytes, s_idx, isob);
            p_ch = np3;
            s_idx = ns4;
        }
    }
    false
}

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`. The flag parameter is passed by mutable reference.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let p_bytes = pat.as_bytes();
    let s_bytes = src.as_bytes();
    let (ret, s_end, p_end) = match_idx(p_bytes, 0, s_bytes, 0, flg);
    let s_end = s_end.min(s_bytes.len());
    let p_end = p_end.min(p_bytes.len());
    let src_out = if src.is_char_boundary(s_end) {
        &src[s_end..]
    } else {
        src
    };
    let pat_out = if pat.is_char_boundary(p_end) {
        &pat[p_end..]
    } else {
        pat
    };
    (ret, src_out, pat_out)
}

/// Internal byte-based skp_ implementation.
/// Returns (alt, to_byte_idx, end_byte_idx)
fn skp_inner_idx(src: &str, pat: &str) -> (i32, usize, usize) {
    if pat.is_empty() {
        return (0, 0, 0);
    }
    let p_bytes = pat.as_bytes();
    let s_bytes = src.as_bytes();
    let mut p_pat_start_idx = 0usize;
    let mut skp_to = false;
    if p_pat_start_idx < p_bytes.len() && p_bytes[p_pat_start_idx] == b'>' {
        skp_to = true;
        p_pat_start_idx += 1;
    }

    let mut start_idx: usize = 0;
    let mut s_idx: usize = start_idx;
    let mut p_idx = p_pat_start_idx;

    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    // Skip leading spaces
    while p_idx < p_bytes.len() && is_space(p_bytes[p_idx] as u32) {
        p_idx += 1;
    }

    while p_idx < p_bytes.len() && p_bytes[p_idx] > 0x07 {
        let (m, s_end_idx, p_end_idx) = match_idx(p_bytes, p_idx, s_bytes, s_idx, &mut flg);
        matched = m;
        if matched != 0 {
            s_idx = s_end_idx;
            p_idx = p_end_idx;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s_idx);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s_idx);
            }
        } else {
            // Skip rest of current alternative
            while p_idx < p_bytes.len() && p_bytes[p_idx] > 0x07 {
                p_idx += 1;
            }
            // Try alternative pattern: separator > 0 and next byte > 0
            if p_idx < p_bytes.len()
                && p_bytes[p_idx] > 0
                && p_idx + 1 < p_bytes.len()
                && p_bytes[p_idx + 1] > 0
            {
                s_idx = start_idx;
                p_idx += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                p_idx = p_pat_start_idx;
                start_idx += 1;
                s_idx = start_idx;
                if start_idx >= s_bytes.len() || s_bytes[start_idx] == 0 {
                    break;
                }
            } else {
                break;
            }
        }
        // Skip spaces in pattern
        while p_idx < p_bytes.len() && is_space(p_bytes[p_idx] as u32) {
            p_idx += 1;
        }
    }

    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        // Force end-of-pattern condition
        p_idx = p_bytes.len();
    }

    if let Some(g) = goal {
        s_idx = g;
    }

    let at_end = p_idx >= p_bytes.len() || p_bytes[p_idx] <= 0x07;
    if matched != 0 && at_end {
        let ret_val = if p_idx < p_bytes.len() && p_bytes[p_idx] > 0 {
            p_bytes[p_idx] as i32
        } else {
            1
        };
        let to_idx = if skp_to { start_idx } else { s_idx };
        let end_idx = s_idx;
        return (ret_val, to_idx, end_idx);
    }

    (0, 0, 0)
}

/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    if pat.is_empty() || src.is_empty() && pat.is_empty() {
        // No pattern -> failure
    }
    if src.is_empty() && pat.is_empty() {
        return (0, src, src);
    }
    // Check null-equivalent: in C, null pat or null src returns 0
    let (alt, to_idx, end_idx) = skp_inner_idx(src, pat);
    if alt > 0 {
        let s_bytes = src.as_bytes();
        let to_idx = to_idx.min(s_bytes.len());
        let end_idx = end_idx.min(s_bytes.len());
        let to_str = if src.is_char_boundary(to_idx) {
            &src[to_idx..]
        } else {
            src
        };
        let end_str = if src.is_char_boundary(end_idx) {
            &src[end_idx..]
        } else {
            src
        };
        (alt, to_str, end_str)
    } else {
        (0, src, src)
    }
}

/// Variant: skp_4(src, pat, to, end).
pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    let (alt, to_idx, end_idx) = skp_inner_idx(src, pat);
    if alt > 0 {
        let s_bytes = src.as_bytes();
        let to_i = to_idx.min(s_bytes.len());
        let end_i = end_idx.min(s_bytes.len());
        let to_str: &str = if src.is_char_boundary(to_i) {
            &src[to_i..]
        } else {
            src
        };
        let end_str: &str = if src.is_char_boundary(end_i) {
            &src[end_i..]
        } else {
            src
        };
        // SAFETY: we assign references derived from `src` to slots that may
        // have been declared with a different lifetime.  Callers are expected
        // to ensure the supplied references outlive `src`.  This mirrors the
        // C API that simply writes through the pointer.
        if let Some(t) = to {
            unsafe {
                *t = std::mem::transmute::<&str, &str>(to_str);
            }
        }
        if let Some(e) = end {
            unsafe {
                *e = std::mem::transmute::<&str, &str>(end_str);
            }
        }
        alt
    } else {
        if let Some(t) = to {
            unsafe {
                *t = std::mem::transmute::<&str, &str>(src);
            }
        }
        if let Some(e) = end {
            unsafe {
                *e = std::mem::transmute::<&str, &str>(src);
            }
        }
        0
    }
}

/// Variant: skp_3(src, pat, end). Note: maps to C's `to` slot (3rd arg).
pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    let (alt, to_idx, _end_idx) = skp_inner_idx(src, pat);
    if alt > 0 {
        let s_bytes = src.as_bytes();
        let to_i = to_idx.min(s_bytes.len());
        let to_str: &str = if src.is_char_boundary(to_i) {
            &src[to_i..]
        } else {
            src
        };
        if let Some(e) = end {
            unsafe {
                *e = std::mem::transmute::<&str, &str>(to_str);
            }
        }
        alt
    } else {
        if let Some(e) = end {
            unsafe {
                *e = std::mem::transmute::<&str, &str>(src);
            }
        }
        0
    }
}

/// Variant: skp_2(src, pat).
pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_inner_idx(src, pat).0
}

// ==================================================================
// AST functions
// ==================================================================

const ASTNULL: i32 = -1;
const SKP_DEBUG: i8 = 0x01;
#[allow(dead_code)]
const SKP_LEFTRECUR: i8 = 0x02;

/// In C: `typedef int32_t astnode_t;`
pub type AstNodeT = i32;

/// Representation of an AST node.
#[derive(Debug, Default, Clone)]
pub struct AstNode {
    pub rule: String,
    pub from: AstNodeT,
    pub to: AstNodeT,
    pub delta: i32, // delta between open and close parenthesis (>0)
    pub tag: i32,
}

/// AST memoization structure
#[derive(Debug, Default)]
pub struct AstMmz {
    pub pos: i32,
    pub endpos: i32,
    pub numnodes: i32,
    pub maxnodes: i32,
    pub lastinfo: i32,
    pub nodes: Vec<AstNode>,
}

/// In C: `typedef struct ast_s *ast_t;`
#[derive(Debug, Default)]
pub struct Ast {
    pub start: String,
    pub err_rule: Option<String>,
    pub err_msg: Option<String>,
    pub cur_rule: Option<String>,
    pub nodes: Vec<AstNode>,
    pub mmz: Vec<AstMmz>,
    pub par: Vec<i32>,
    pub auxptr: Option<Box<dyn std::any::Any>>,
    pub nodes_cnt: i32,
    pub nodes_max: i32,
    pub par_cnt: i32,
    pub par_max: i32,
    pub mmz_cnt: i32,
    pub mmz_max: i32,
    pub pos: i32,
    pub lastpos: i32,
    pub err_pos: i32,
    pub cur_node: i32,
    pub lastinfo: i32,
    pub ret: i32,
    pub depth: u16,
    pub fail: i8,
    pub flg: i8,
}

pub type SkpRule = fn(ast: &mut Ast, ret: &mut i32);

/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    Some(Ast {
        start: String::new(),
        err_rule: None,
        err_msg: Some(String::new()),
        cur_rule: None,
        nodes: Vec::with_capacity(8),
        mmz: Vec::new(),
        par: Vec::with_capacity(16),
        auxptr: None,
        nodes_cnt: 0,
        nodes_max: 8,
        par_cnt: 0,
        par_max: 16,
        mmz_cnt: 0,
        mmz_max: 64,
        pos: 0,
        lastpos: 0,
        err_pos: -1,
        cur_node: ASTNULL,
        lastinfo: 0,
        ret: 0,
        depth: 0,
        fail: 0,
        flg: 0,
    })
}

/// Frees an AST.
pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
}

/// Parses the source string `src` using a given parsing rule.
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let pos0 = ast.pos;
    let open = ast_open(&mut ast, pos0, rulename);
    if open >= 0 {
        let mut ret = ast.ret;
        rule(&mut ast, &mut ret);
        ast.ret = ret;

        if ast.fail != 0 && ast.err_pos < ast.pos {
            ast.err_pos = ast.pos;
            ast.err_rule = Some(rulename.to_string());
        }

        let pos = ast.pos;
        ast_close(&mut ast, pos, open);
        if ast.nodes_cnt > 0 {
            ast.err_pos = -1;
            let info = ast.lastinfo;
            ast_setinfo(&mut ast, info, 0);
        }
    }
    // mmz cleanup not strictly needed as Vec drops naturally
    ast.mmz.clear();
    ast.mmz_cnt = 0;
    Some(ast)
}

/// Debug function for AST.
pub fn skp_debug2(ast: &mut Ast, d: u8) -> i32 {
    match d {
        0 => ast.flg &= !SKP_DEBUG,
        1 => ast.flg |= SKP_DEBUG,
        _ => ast.flg ^= SKP_DEBUG,
    }
    (ast.flg & SKP_DEBUG) as i32
}

fn skp_par_makeroom(ast: &mut Ast, needed: i32) -> bool {
    if ast.par_cnt + needed > ast.par_max {
        let mut new_max = ast.par_max;
        loop {
            new_max += new_max / 2;
            new_max += new_max & 1;
            if ast.par_cnt + needed <= new_max {
                break;
            }
        }
        ast.par.resize(new_max as usize, 0);
        ast.par_max = new_max;
    } else {
        // ensure underlying Vec has the capacity to be indexed
        if (ast.par.len() as i32) < ast.par_cnt + needed {
            ast.par.resize((ast.par_cnt + needed) as usize, 0);
        }
    }
    true
}

fn skp_nodes_makeroom(ast: &mut Ast, needed: i32) -> bool {
    if ast.nodes_cnt + needed > ast.nodes_max {
        let mut new_max = ast.nodes_max;
        loop {
            new_max += new_max / 2;
            new_max += new_max & 1;
            if ast.nodes_cnt + needed <= new_max {
                break;
            }
        }
        ast.nodes
            .resize(new_max as usize, AstNode::default());
        ast.nodes_max = new_max;
    } else if (ast.nodes.len() as i32) < ast.nodes_cnt + needed {
        ast.nodes
            .resize((ast.nodes_cnt + needed) as usize, AstNode::default());
    }
    true
}

fn ast_newpar(ast: &mut Ast) -> i32 {
    if !skp_par_makeroom(ast, 1) {
        return -1;
    }
    let p = ast.par_cnt;
    ast.par_cnt += 1;
    if (ast.par.len() as i32) <= p {
        ast.par.resize((p + 1) as usize, 0);
    }
    p
}

fn ast_newnode(ast: &mut Ast) -> i32 {
    if !skp_nodes_makeroom(ast, 1) {
        return -1;
    }
    let n = ast.nodes_cnt;
    ast.nodes_cnt += 1;
    if (ast.nodes.len() as i32) <= n {
        ast.nodes.resize((n + 1) as usize, AstNode::default());
    }
    n
}

/// Opens a new AST node starting at position `from` with the given rule.
pub fn ast_open(ast: &mut Ast, from: i32, rule: &str) -> i32 {
    if ast.fail != 0 {
        return -1;
    }
    let par = ast_newpar(ast);
    if par < 0 {
        return -1;
    }
    let node = ast_newnode(ast);
    if node < 0 {
        return -1;
    }
    ast.par[par as usize] = node;
    ast.nodes[node as usize] = AstNode {
        rule: rule.to_string(),
        from,
        to: 0,
        delta: 0,
        tag: 0,
    };
    par
}

/// Closes the current AST node at position `to`, linking with the open node `open`.
pub fn ast_close(ast: &mut Ast, to: i32, open: i32) -> i32 {
    if open < 0 {
        return -1;
    }
    let node_idx = ast.par[open as usize];
    if ast.fail != 0 {
        let from = ast.nodes[node_idx as usize].from;
        ast.pos = from;
        ast.nodes_cnt = node_idx;
        ast.par_cnt = open;
        return -1;
    }
    let par = ast_newpar(ast);
    if par < 0 {
        return -1;
    }
    let delta = par - open;
    {
        let nd = &mut ast.nodes[node_idx as usize];
        nd.to = to;
        nd.delta = delta;
        nd.tag = 0;
    }
    ast.par[par as usize] = -delta;

    let rule = ast.nodes[node_idx as usize].rule.clone();
    ast.cur_node = par;
    ast.cur_rule = Some(rule);
    par
}

/// Aborts parsing with the given message and rule.
pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    if !msg.is_empty() {
        ast.err_msg = Some(msg.to_string());
    }
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
    // Note: Rust does not use setjmp/longjmp.  Set the failure flag so the
    // caller observes the abort condition.
}

/// Records memoization of AST nodes (for left recursion etc.).
pub fn skp_memoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str, old_pos: i32, start_par: i32) {
    let end_par = ast.par_cnt;
    let (start_par, end_par) = if ast.fail != 0 || end_par <= start_par {
        (-1i32, -1i32)
    } else {
        (start_par, end_par)
    };
    let numnodes = if start_par < 0 || end_par < 0 {
        0
    } else {
        (end_par - start_par) / 2
    };

    mmz.pos = old_pos;
    mmz.endpos = ast.pos;
    mmz.numnodes = if ast.fail != 0 { -1 } else { numnodes };
    mmz.maxnodes = numnodes;
    mmz.lastinfo = ast.lastinfo;

    mmz.nodes.clear();
    if start_par >= 0 && end_par >= 0 {
        for k in start_par..end_par {
            let p = ast.par[k as usize];
            if p >= 0 {
                mmz.nodes.push(ast.nodes[p as usize].clone());
            }
        }
    }
}

/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str) -> i32 {
    if mmz.pos != ast.pos {
        return 0;
    }
    let numnodes = mmz.numnodes;
    ast.fail = if numnodes < 0 { 1 } else { 0 };
    ast.lastpos = ast.pos;
    ast.pos = mmz.endpos;
    ast.lastinfo = mmz.lastinfo;
    if numnodes > 0 {
        skp_nodes_makeroom(ast, numnodes);
        skp_par_makeroom(ast, 2 * numnodes);
        let start_par = ast.par_cnt;
        let total_par = (2 * numnodes) as usize;
        for i in 0..total_par {
            let idx = start_par as usize + i;
            if ast.par.len() <= idx {
                ast.par.resize(idx + 1, i32::MAX);
            } else {
                ast.par[idx] = i32::MAX;
            }
        }
        let mut cur_par = start_par;
        for k in 0..numnodes {
            let n = ast.nodes_cnt;
            if (ast.nodes.len() as i32) <= n {
                ast.nodes.resize((n + 1) as usize, AstNode::default());
            }
            ast.nodes[n as usize] = mmz.nodes[k as usize].clone();
            while cur_par < ast.par_max && ast.par[cur_par as usize] != i32::MAX {
                cur_par += 1;
            }
            if cur_par >= ast.par_max {
                break;
            }
            ast.par[cur_par as usize] = n;
            let delta = mmz.nodes[k as usize].delta;
            let close_idx = (cur_par + delta) as usize;
            if close_idx < ast.par.len() {
                ast.par[close_idx] = -delta;
            }
            ast.nodes_cnt += 1;
        }
        ast.par_cnt += 2 * numnodes;
    }
    1
}

/// Sets AST node information (tag).
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= node {
        return;
    }
    let mut node = node;
    if node == ASTNULL {
        node = ast.par_cnt - 1;
    }
    if node < 0 || node >= ast.par_cnt {
        return;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return;
    }
    let pn = ast.par[idx as usize];
    if pn >= 0 && (pn as usize) < ast.nodes.len() {
        ast.nodes[pn as usize].tag = info;
    }
}

/// Records a new AST info node.
pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail != 0 {
        return;
    }
    let pos = ast.pos;
    let par = ast_open(ast, pos, "#");
    ast_close(ast, pos, par);
    if par >= 0 && (par as usize) < ast.par.len() {
        let pn = ast.par[par as usize];
        if pn >= 0 && (pn as usize) < ast.nodes.len() {
            ast.nodes[pn as usize].tag = info;
        }
    }
    ast.lastinfo = info;
}

/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return 0;
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return 0;
    }
    ast.nodes[pn as usize].tag
}

/// Swaps the last two AST nodes.
pub fn ast_swap(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    let c2 = o1 - 1;
    if c2 < 0 || ast.par[c2 as usize] >= 0 {
        return;
    }
    let o2 = c2 + ast.par[c2 as usize];
    if o2 < 0 || ast.par[o2 as usize] < 0 {
        return;
    }
    let len2 = (c2 - o2 + 1) as usize;
    let len1 = (c1 - o1 + 1) as usize;
    let tmp: Vec<i32> = ast.par[o2 as usize..(o2 as usize + len2)].to_vec();
    // Move first block to o2
    for i in 0..len1 {
        ast.par[o2 as usize + i] = ast.par[o1 as usize + i];
    }
    // Place tmp after the moved block
    for i in 0..len2 {
        ast.par[o2 as usize + len1 + i] = tmp[i];
    }
}

/// Lowers a node (wraps a group of nodes into a new parent).
pub fn ast_lower(ast: &mut Ast, rule: &str, lft: AstNodeT, rgt: AstNodeT) {
    if ast.par_cnt <= lft || ast.par_cnt <= rgt || lft >= rgt {
        return;
    }
    let mut lft = lft;
    let mut rgt = rgt;
    if ast.par[lft as usize] < 0 {
        lft += ast.par[lft as usize];
    }
    if ast.par[rgt as usize] < 0 {
        rgt += ast.par[rgt as usize];
    }
    let lft_node = ast.par[lft as usize];
    let rgt_node = ast.par[rgt as usize];
    let node_from = ast.nodes[lft_node as usize].from;
    let node_to = ast.nodes[rgt_node as usize].to;
    rgt += ast.nodes[rgt_node as usize].delta;

    let new_node = ast_newnode(ast);
    if new_node < 0 {
        return;
    }
    let delta = rgt - lft + 2;
    ast.nodes[new_node as usize] = AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta,
        tag: 0,
    };
    if ast_newpar(ast) < 0 {
        return;
    }
    if ast_newpar(ast) < 0 {
        return;
    }

    // Move nodes after rgt down by 2
    if ast.par_cnt - 1 - rgt > 2 {
        let count = (ast.par_cnt - 1 - rgt - 2) as usize;
        for i in (0..count).rev() {
            ast.par[(rgt + 3) as usize + i] = ast.par[(rgt + 1) as usize + i];
        }
    }
    // Move block [lft..=rgt] right by 1
    let block_len = (rgt - lft + 1) as usize;
    for i in (0..block_len).rev() {
        ast.par[(lft + 1) as usize + i] = ast.par[lft as usize + i];
    }
    ast.par[lft as usize] = new_node;
    ast.par[(rgt + 2) as usize] = -delta;
}

/// Lifts a node (removes a level from the AST).
pub fn ast_lift(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return;
    }
    let c2 = c1 - 1;
    if c2 < 0 || ast.par[c2 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    let o2 = c2 + ast.par[c2 as usize];
    if o2 < 0 || ast.par[o2 as usize] < 0 {
        return;
    }
    if o2 != o1 + 1 {
        return;
    }
    if ast.nodes[ast.par[o1 as usize] as usize].tag == 0 {
        let span = (c2 - o2 + 1) as usize;
        for i in 0..span {
            ast.par[o1 as usize + i] = ast.par[o2 as usize + i];
        }
        ast.par_cnt -= 2;
    }
}

/// Lifts all single-child nodes.
pub fn ast_lift_all(ast: &mut Ast) {
    loop {
        let n = ast.par_cnt;
        ast_lift(ast);
        if n == ast.par_cnt {
            break;
        }
    }
}

/// Removes the last leaf node.
pub fn ast_noleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    if c1 == o1 + 1 {
        ast.par_cnt -= 2;
    }
}

/// Removes the last empty leaf node.
pub fn ast_noemptyleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    if c1 != o1 + 1 {
        return;
    }
    let pn = ast.par[o1 as usize];
    if ast.nodes[pn as usize].from != ast.nodes[pn as usize].to {
        return;
    }
    ast.par_cnt -= 2;
}

/// Returns the index of the last AST node.
pub fn ast_lastnode(ast: &Ast) -> AstNodeT {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return ASTNULL;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return ASTNULL;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return ASTNULL;
    }
    o1
}

/// Checks if the last node is empty.
pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == ASTNULL {
        return false;
    }
    let pn = ast.par[node as usize];
    if pn < 0 {
        return false;
    }
    let nd = &ast.nodes[pn as usize];
    nd.from == nd.to
}

/// Deletes the last node.
pub fn ast_delete(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    ast.par_cnt -= c1 - o1 + 1;
}

/// Returns the “left” sibling of a node.
pub fn astleft(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    node -= 1;
    if node <= 0 || ast.par[node as usize] >= 0 {
        return ASTNULL;
    }
    node += ast.par[node as usize];
    node
}

/// Returns the “right” sibling of a node.
pub fn astright(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    if ast.par[node as usize] > 0 {
        let pn = ast.par[node as usize];
        node += ast.nodes[pn as usize].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        return ASTNULL;
    }
    node
}

/// Returns the parent of a node.
pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let node = astfirst(ast, node);
    if node == ASTNULL {
        return ASTNULL;
    }
    let n = node - 1;
    if n < 0 || ast.par[n as usize] < 0 {
        return ASTNULL;
    }
    n
}

/// Returns the first child of a node.
pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let n = node + 1;
    if n >= ast.par_cnt || ast.par[n as usize] < 0 {
        return ASTNULL;
    }
    n
}

/// Returns the leftmost sibling of a node.
pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut cur = node;
    loop {
        let n = astleft(ast, cur);
        if n == ASTNULL {
            break;
        }
        cur = n;
    }
    cur
}

/// Returns the rightmost sibling of a node.
pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut cur = node;
    loop {
        let n = astright(ast, cur);
        if n == ASTNULL {
            break;
        }
        cur = n;
    }
    cur
}

/// Returns the next node in a depth-first traversal.
pub fn astnextdf(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let n = node + 1;
    if n < 0 {
        return 0;
    }
    if n >= ast.par_cnt {
        return ASTNULL;
    }
    n
}

/// Checks if the given index is an entry (open parenthesis) node.
pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && node >= 0 && ast.par[node as usize] >= 0
}

/// Checks if the given index is an exit (closing parenthesis) node.
pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && node >= 0 && ast.par[node as usize] < 0
}

/// Returns the rule name associated with a node.
pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return "";
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return "";
    }
    &ast.nodes[pn as usize].rule
}

/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return "";
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return "";
    }
    let from = ast.nodes[pn as usize].from as usize;
    let bytes = ast.start.as_bytes();
    let f = from.min(bytes.len());
    if ast.start.is_char_boundary(f) {
        &ast.start[f..]
    } else {
        ""
    }
}

/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return "";
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return "";
    }
    let to = ast.nodes[pn as usize].to as usize;
    let bytes = ast.start.as_bytes();
    let t = to.min(bytes.len());
    if ast.start.is_char_boundary(t) {
        &ast.start[t..]
    } else {
        ""
    }
}

/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return 0;
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return 0;
    }
    ast.nodes[pn as usize].to - ast.nodes[pn as usize].from
}

/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node < 0 || node >= ast.par_cnt {
        return false;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return false;
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return false;
    }
    ast.nodes[pn as usize].delta == 1
}

/// Returns the next node in the AST (wrapper for astnextdf).
pub fn astnext(ast: &Ast, node: AstNodeT) -> AstNodeT {
    astnextdf(ast, node)
}

/// Returns a match code if the node’s rule is one of several provided.
pub fn ast_isn(
    ast: &Ast,
    node: AstNodeT,
    r1: &str,
    r2: Option<&str>,
    r3: Option<&str>,
    r4: Option<&str>,
    r5: Option<&str>,
) -> i32 {
    if ast_is(ast, node, r1) != 0 {
        return 1;
    }
    if let Some(r) = r2 {
        if ast_is(ast, node, r) != 0 {
            return 1;
        }
    }
    if let Some(r) = r3 {
        if ast_is(ast, node, r) != 0 {
            return 1;
        }
    }
    if let Some(r) = r4 {
        if ast_is(ast, node, r) != 0 {
            return 1;
        }
    }
    if let Some(r) = r5 {
        if ast_is(ast, node, r) != 0 {
            return 1;
        }
    }
    0
}

/// Checks if a node’s rule matches a given rule.
pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 {
        return 0;
    }
    let pn = ast.par[idx as usize];
    if pn < 0 || (pn as usize) >= ast.nodes.len() {
        return 0;
    }
    if ast.nodes[pn as usize].rule == rulename {
        1
    } else {
        0
    }
}

/// Checks if the AST contains an error.
pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

/// Returns the rule name at which an error occurred.
pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return Some("");
    }
    ast.err_rule.as_deref()
}

/// Returns the error position as a string pointer.
pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return Some("");
    }
    let p = ast.err_pos as usize;
    let bytes = ast.start.as_bytes();
    if p > bytes.len() || !ast.start.is_char_boundary(p) {
        return Some("");
    }
    Some(&ast.start[p..])
}

/// Returns the start of the error line.
pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let bytes = ast.start.as_bytes();
    let mut p = (ast.err_pos as usize).min(bytes.len());
    while p > 0 {
        let prev = bytes[p - 1];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        p -= 1;
    }
    if ast.start.is_char_boundary(p) {
        &ast.start[p..]
    } else {
        ""
    }
}

/// Returns the error column number.
pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let bytes = ast.start.as_bytes();
    let mut p = (ast.err_pos as usize).min(bytes.len());
    let err_pos = p;
    while p > 0 {
        let prev = bytes[p - 1];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        p -= 1;
    }
    (err_pos - p) as i32
}

/// Prints the AST in s-expression format.
pub fn astprintsexpr(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: AstNodeT = ASTNULL;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL {
            break;
        }
        if astisnodeentry(ast, node) {
            let _ = write!(f, "({} ", astnoderule(ast, node));
            if astisleaf(ast, node) {
                let _ = write!(f, "'");
                if astnoderule(ast, node) == "#" {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from_s = astnodefrom(ast, node);
                    let to_s = astnodeto(ast, node);
                    let from_len = from_s.len();
                    let to_len = to_s.len();
                    let take = from_len.saturating_sub(to_len);
                    let slice = &from_s[..take.min(from_len)];
                    for ch in slice.chars() {
                        if ch == '\'' {
                            let _ = write!(f, "\\");
                        }
                        let _ = write!(f, "{}", ch);
                    }
                }
                let _ = write!(f, "'");
            }
        } else {
            let _ = write!(f, ")");
        }
    }
}

/// Prints the AST as a tree.
pub fn astprinttree(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: AstNodeT = ASTNULL;
    let mut levl: i32 = 0;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL {
            break;
        }
        if astisnodeentry(ast, node) {
            let mut k = 0;
            while k < levl {
                let _ = write!(f, "    ");
                k += 4;
            }
            let _ = write!(f, "[{}", astnoderule(ast, node));
            let tag = astnodeinfo(ast, node);
            if tag != 0 {
                let _ = write!(f, " ({})", tag);
            }
            let _ = write!(f, "]");
            levl += 4;
            if astisleaf(ast, node) {
                let _ = write!(f, " '");
                let from_s = astnodefrom(ast, node);
                let to_s = astnodeto(ast, node);
                let from_len = from_s.len();
                let to_len = to_s.len();
                let take = from_len.saturating_sub(to_len);
                let slice = &from_s[..take.min(from_len)];
                for ch in slice.chars() {
                    if ch == '\'' {
                        let _ = write!(f, "\\");
                    }
                    let _ = write!(f, "{}", ch);
                }
                let _ = write!(f, "'");
            }
            let _ = writeln!(f);
        } else {
            levl -= 4;
        }
    }
}

// effective_len is referenced by some helpers but currently unused; suppress warnings
#[allow(dead_code)]
fn _unused() {
    let _ = effective_len("");
}
