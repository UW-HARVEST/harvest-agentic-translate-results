use std::fmt;
use std::io::Write;
use std::panic::{catch_unwind, panic_any, resume_unwind, AssertUnwindSafe};

/// SKP version information.
pub const SKP_VER: u32 = 0x0003001C;
pub const SKP_VER_STR: &str = "0.3.1rc";

const SKP_DEBUG: i8 = 0x01;
const SKP_DELTA_MAX: i32 = i32::MAX;
const SKP_STARTNODES: usize = 8;
const SKP_EMPTY: &str = "";
const SKP_INFO_RULE: &str = "#";

#[derive(Debug)]
struct AbortSignal;

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
    let ret = to.len() as i32 - start.len() as i32;
    if (0..=(1 << 16)).contains(&ret) {
        ret
    } else {
        0
    }
}

/// Global variable used in the C code.
/// (In C declared as `volatile int skp_zero;`—here we use a mutable static.)
pub static mut SKP_ZERO: i32 = 0;

/// Trace function (corresponds to the C macro skptrace).
pub fn skptrace(args: fmt::Arguments) {
    eprintln!("TRCE: {}", args);
}

fn ascii_lower_u32(c: u32) -> u32 {
    if (b'A' as u32..=b'Z' as u32).contains(&c) {
        c + 32
    } else {
        c
    }
}

fn next_boundary(s: &str, mut idx: usize) -> usize {
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

fn byte_at(s: &str, idx: usize) -> u8 {
    s.as_bytes().get(idx).copied().unwrap_or(0)
}

fn slice_from(s: &str, idx: usize) -> &str {
    &s[next_boundary(s, idx)..]
}

fn slice_range(s: &str, start: usize, end: usize) -> &str {
    &s[next_boundary(s, start)..next_boundary(s, end)]
}

fn skp_next_raw(s: &str, iso: i32) -> (u32, usize) {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return (0, 0);
    }

    let mut c = bytes[0] as u32;
    let mut pos = 1usize;

    if iso == 0 {
        while pos < bytes.len() && pos < 4 && (bytes[pos] & 0xC0) == 0x80 {
            c = (c << 8) | bytes[pos] as u32;
            pos += 1;
        }
    }

    if c == 0x0D && bytes.get(pos) == Some(&0x0A) {
        c = 0x0D0A;
        pos += 1;
    }

    (c, pos)
}

fn advance_byte(src: &str, s_end: &mut usize, s_tmp: &mut usize, s_chr: &mut u32) {
    *s_end = *s_tmp;
    if *s_end < src.len() {
        *s_chr = byte_at(src, *s_end) as u32;
        *s_tmp = *s_end + 1;
    } else {
        *s_chr = 0;
        *s_tmp = *s_end;
    }
}

fn skip_pat_spaces(pat: &str, mut idx: usize) -> usize {
    while idx < pat.len() && is_space(byte_at(pat, idx) as u32) {
        idx += 1;
    }
    idx
}

/// The core scanning function from the C header.
///
/// This corresponds to:
/// ```c
/// int skp_(char *src, char *pat, char **to, char **end);
/// ```
/// In Rust we take `&str` for both source and pattern and return a tuple:
/// `(match_code, to, end)`.
pub fn skp_<'a>(src: &'a str, pat: &str) -> (i32, &'a str, &'a str) {
    if src.is_empty() && pat.is_empty() {
        return (0, src, src);
    }

    let mut start_idx = 0usize;
    let mut skp_to = false;
    let mut pat_idx = 0usize;
    let mut matched = 0i32;
    let mut goal_idx: Option<usize> = None;
    let mut goalnot_idx: Option<usize> = None;
    let mut flg = 0i32;
    let mut had_non_goal_match = false;

    if byte_at(pat, pat_idx) == b'>' {
        skp_to = true;
        pat_idx += 1;
    }

    pat_idx = skip_pat_spaces(pat, pat_idx);
    let pat_start = pat_idx;
    let mut s_idx = start_idx;

    while pat_idx < pat.len() && byte_at(pat, pat_idx) > 7 {
        let (m, s_end, p_end) = match_pat(slice_from(pat, pat_idx), slice_from(src, s_idx), &mut flg);
        if m != MATCHED_FAIL {
            let prev_s_idx = s_idx;
            matched = m;
            s_idx = src.len() - s_end.len();
            pat_idx = pat.len() - p_end.len();
            if matched == MATCHED && s_idx >= prev_s_idx {
                had_non_goal_match = true;
            }
            if matched == MATCHED_GOAL && goalnot_idx.is_none() {
                goal_idx = Some(s_idx);
            } else if matched == MATCHED_GOALNOT {
                goalnot_idx = Some(s_idx);
            }
        } else {
            matched = MATCHED_FAIL;
            while pat_idx < pat.len() && byte_at(pat, pat_idx) > 7 {
                pat_idx += 1;
            }

            if pat_idx < pat.len() && byte_at(pat, pat_idx) > 0 && byte_at(pat, pat_idx + 1) > 0 {
                s_idx = start_idx;
                pat_idx += 1;
            } else if skp_to {
                goal_idx = None;
                goalnot_idx = None;
                pat_idx = pat_start;
                start_idx = next_boundary(src, start_idx.saturating_add(1));
                s_idx = start_idx;
                if start_idx >= src.len() {
                    break;
                }
            } else {
                break;
            }
        }

        pat_idx = skip_pat_spaces(pat, pat_idx);
    }

    if matched == MATCHED_FAIL {
        if let Some(goalnot) = goalnot_idx {
            goal_idx = Some(goalnot);
            matched = MATCHED;
            pat_idx = pat.len();
        }
    }

    if let Some(goal) = goal_idx {
        s_idx = goal;
    }

    if !had_non_goal_match && matches!(matched, MATCHED_GOAL | MATCHED_GOALNOT) {
        return (0, src, src);
    }

    if matched != MATCHED_FAIL && (pat_idx >= pat.len() || byte_at(pat, pat_idx) <= 7) {
        let ret = if pat_idx < pat.len() && byte_at(pat, pat_idx) > 0 {
            byte_at(pat, pat_idx) as i32
        } else {
            1
        };
        if skp_to {
            return (ret, slice_range(src, start_idx, s_idx), src);
        }
        return (ret, slice_from(src, s_idx), slice_from(src, s_idx));
    }

    (0, src, src)
}

/// In the C header a set of macros provides variants:
///   skp(src, pat), skp(src, pat, end) and skp(src, pat, to, end).
///
/// The following functions mimic those overloads.
pub fn skp_4<'a>(src: &'a str, pat: &str, to: Option<&mut &'a str>, end: Option<&mut &'a str>) -> i32 {
    let (ret, to_s, end_s) = skp_(src, pat);
    if let Some(to_ref) = to {
        *to_ref = to_s;
    }
    if let Some(end_ref) = end {
        *end_ref = end_s;
    }
    ret
}

pub fn skp_3<'a>(src: &'a str, pat: &str, end: Option<&mut &'a str>) -> i32 {
    skp_4(src, pat, end, None)
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_(src, pat).0
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let (c, consumed) = skp_next_raw(s, iso);
    (c, slice_from(s, consumed))
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
/// (Corresponds to `chr_cmp`.)
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        ascii_lower_u32(a) == ascii_lower_u32(b)
    } else {
        a == b
    }
}

/// Returns true if `c` is a blank character.
/// (Corresponds to `is_blank`.)
pub fn is_blank(c: u32) -> bool {
    if c < 0xFF {
        return c == 0x20 || c == 0x09;
    }

    match c & 0xFFFF_FF00 {
        0x0000_0000 => c == 0xA0,
        0x0000_C200 => c == 0xC2A0,
        0x00E1_9A00 => c == 0xE19A80,
        0x00E2_8000 => (0xE28080..=0xE2808A).contains(&c) || c == 0xE280AF,
        0x00E3_8080 => c == 0xE38080,
        _ => false,
    }
}

/// Returns true if `c` is a line-break character.
/// (Corresponds to `is_break`.)
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
    is_digit(c)
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

    let mut rest = set;
    let (mut p_ch, next_rest) = skp_next(rest, iso);
    rest = next_rest;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        let next = skp_next(rest, iso);
        p_ch = next.0;
        rest = next.1;
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }

        let q_ch = p_ch;
        let next = skp_next(rest, iso);
        p_ch = next.0;
        rest = next.1;

        if p_ch == b'-' as u32 && !rest.is_empty() && byte_at(rest, 0) != b']' {
            let next = skp_next(rest, iso);
            p_ch = next.0;
            rest = next.1;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let next = skp_next(rest, iso);
            p_ch = next.0;
            rest = next.1;
        }
    }

    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    let target_len = len.max(0) as usize;
    let mut p_idx = 0usize;
    let mut s_cur = s;
    let s_start = s;
    let mut mlen = 0i32;
    let iso = flg & 2;
    let fold = flg & 1;

    while p_idx < target_len {
        if byte_at(p, p_idx) == 0x0E {
            return mlen;
        }

        let p_slice = &p[p_idx..];
        let (p_chr, p_rest) = skp_next(p_slice, iso);
        let (s_chr, s_rest) = skp_next(s_cur, iso);

        if chr_cmp(s_chr, p_chr, fold) {
            let p_consumed = p_slice.len() - p_rest.len();
            let s_consumed = s_cur.len() - s_rest.len();
            mlen += s_consumed as i32;
            p_idx += p_consumed;
            s_cur = s_rest;
        } else {
            while p_idx < target_len && byte_at(p, p_idx) != 0x0E {
                p_idx += 1;
            }
            if p_idx >= target_len {
                return 0;
            }
            p_idx += 1;
            s_cur = s_start;
            mlen = 0;
        }
    }

    mlen
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    match open {
        40 => b')' as u32,
        91 => b']' as u32,
        123 => b'}' as u32,
        60 => b'>' as u32,
        _ => 0,
    }
}

/// Given a quote character, returns the corresponding closing quote.
pub fn get_qclose(open: u32) -> u32 {
    match open {
        39 | 34 | 96 => open,
        _ => 0,
    }
}

/// Constants for match results.
pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`. The flag parameter is passed by mutable reference.
pub fn match_pat<'a, 'b>(pat: &'b str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'b str) {
    let mut p_idx = 0usize;
    let mut s_end = 0usize;
    let mut s_tmp;
    let mut ret = MATCHED_FAIL;
    let mut match_min = 1u32;
    let mut match_max = 1u32;
    let mut match_not = 0u32;
    let (mut s_chr, next_s) = skp_next_raw(src, *flg & 2);
    s_tmp = next_s;

    match byte_at(pat, p_idx) {
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

    if byte_at(pat, p_idx) == b'!' {
        match_not = 1;
        p_idx += 1;
    }

    let token = byte_at(pat, p_idx);
    p_idx += usize::from(token != 0);

    macro_rules! do_while_match {
        ($cond:expr) => {{
            let mut match_cnt = 0u32;
            while match_cnt < match_max && s_chr != 0 && ((($cond) as u32) != match_not) {
                match_cnt += 1;
                let new_end = s_tmp;
                let (new_chr, new_tmp) = skp_next_raw(slice_from(src, new_end), *flg & 2);
                s_end = new_end;
                s_chr = new_chr;
                s_tmp = new_end + new_tmp;
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    match token {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                do_while_match!(s_chr != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                do_while_match!(is_break(s_chr));
            }
        }
        b'n' => do_while_match!(is_break(s_chr)),
        b'd' => do_while_match!(is_digit(s_chr)),
        b'x' => do_while_match!(is_xdigit(s_chr)),
        b'a' => do_while_match!(is_alpha(s_chr)),
        b'u' => do_while_match!(is_upper(s_chr)),
        b'l' => do_while_match!(is_lower(s_chr)),
        b's' => do_while_match!(is_space(s_chr)),
        b'w' => do_while_match!(is_blank(s_chr)),
        b'c' => do_while_match!(is_ctrl(s_chr)),
        b'i' => do_while_match!(is_idchr(s_chr)),
        b'@' => {
            if is_alnum(s_chr) {
                ret = MATCHED_GOAL;
            }
        }
        b'&' => {
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'[' => {
            do_while_match!(is_oneof(s_chr, slice_from(pat, p_idx), *flg & 2));
            if byte_at(pat, p_idx) == b']' {
                p_idx += 1;
            }
            while p_idx < pat.len() && byte_at(pat, p_idx) != b']' {
                p_idx += 1;
            }
            if p_idx < pat.len() {
                p_idx += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = token;
            let mut l = 0usize;
            while p_idx + l < pat.len() && byte_at(pat, p_idx + l) != quote {
                l += 1;
            }

            let ml = if l > 0 {
                is_string(slice_from(src, s_end), slice_from(pat, p_idx), l as i32, *flg)
            } else {
                0
            };

            if l > 0 && ml > 0 {
                if match_not == 0 {
                    s_end += ml as usize;
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            }

            p_idx += l + usize::from(p_idx + l < pat.len());
        }
        b'C' => {
            *flg = (*flg & !1) | match_not as i32;
            ret = MATCHED;
        }
        b'U' => {
            *flg = (*flg & !2) | ((match_not as i32) * 2);
            ret = MATCHED;
        }
        b'S' => {
            while is_space(s_chr) {
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) {
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            ret = MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !is_break(s_chr) {
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            if s_chr != 0 {
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            ret = MATCHED;
        }
        b'I' => {
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            if byte_at(pat, p_idx) == b')' && s_chr == b'(' as u32 {
                p_idx += 1;
                let open = s_chr;
                let close = get_close(open);
                if close != 0 {
                    let mut count = 1i32;
                    while s_chr != 0 && count > 0 {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                        if s_chr == open {
                            count += 1;
                        }
                        if s_chr == close {
                            count -= 1;
                        }
                    }
                    if count == 0 {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                        ret = MATCHED;
                    }
                }
            } else {
                p_idx = p_idx.saturating_sub(1);
            }
        }
        b'B' => {
            let open = s_chr;
            let close = get_close(open);
            if close != 0 {
                let mut count = 1i32;
                while s_chr != 0 && count > 0 {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if s_chr == open {
                        count += 1;
                    }
                    if s_chr == close {
                        count -= 1;
                    }
                }
                if count == 0 {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    ret = MATCHED;
                }
            }
        }
        b'Q' => {
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == b'\\' as u32 {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    }
                }
                if s_chr != 0 {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            if s_chr == b'0' as u32
                && matches!(byte_at(src, s_end + 1), b'x' | b'X')
                && is_xdigit(byte_at(src, s_end + 2) as u32)
            {
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
        }
        b'D' | b'F' => {
            let intnumber = token == b'D';
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if !is_space(s_chr) {
                        break;
                    }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            if !intnumber {
                if s_chr == b'.' as u32 {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                }
                while is_digit(s_chr) {
                    ret = MATCHED;
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                }
                if ret == MATCHED && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                    advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    }
                    while is_digit(s_chr) {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    }
                    if s_chr == b'.' as u32 {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    }
                    while is_digit(s_chr) {
                        advance_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    }
                }
            }
        }
        _ => {
            ret = MATCHED_FAIL;
            p_idx = p_idx.saturating_sub(1);
        }
    }

    if ret != MATCHED_FAIL {
        (ret, slice_from(src, s_end), slice_from(pat, p_idx))
    } else {
        (ret, src, pat)
    }
}

/// ---------------------------------------------------------------------------
///
/// # AST Parsing Functions and Types
///
/// ---------------------------------------------------------------------------
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

/// In C, the AST “memory zone” structure:
#[derive(Debug, Default, Clone)]
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
    // jmp_buf omitted because Rust does not use setjmp/longjmp
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

/// A function pointer type for parsing rules.
/// (In C: `typedef void (*skprule_t)(ast_t, int32_t *);`)
pub type SkpRule = fn(ast: &mut Ast, ret: &mut i32);

fn sync_counts(ast: &mut Ast) {
    ast.nodes_cnt = ast.nodes.len() as i32;
    ast.nodes_max = ast.nodes.capacity() as i32;
    ast.par_cnt = ast.par.len() as i32;
    ast.par_max = ast.par.capacity() as i32;
    ast.mmz_cnt = ast.mmz.len() as i32;
    ast.mmz_max = ast.mmz.capacity() as i32;
}

fn ast_newpar(ast: &mut Ast) -> i32 {
    ast.par.push(0);
    sync_counts(ast);
    ast.par_cnt - 1
}

fn ast_newnode(ast: &mut Ast) -> i32 {
    ast.nodes.push(AstNode::default());
    sync_counts(ast);
    ast.nodes_cnt - 1
}

fn node_open_index(ast: &Ast, node: AstNodeT) -> Option<usize> {
    if node < 0 || node >= ast.par_cnt {
        return None;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = idx.checked_add_signed(ast.par[idx] as isize)?;
    }
    if idx >= ast.par.len() || ast.par[idx] < 0 {
        None
    } else {
        Some(idx)
    }
}

fn node_info_index(ast: &Ast, node: AstNodeT) -> Option<usize> {
    let open = node_open_index(ast, node)?;
    let idx = ast.par[open];
    if idx < 0 {
        None
    } else {
        Some(idx as usize)
    }
}

/// Parses the source string `src` using a given parsing rule.
/// (Corresponds to `ast_t skp_parse(char *src, skprule_t rule, char *rulename, int debug);`)
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
    if open < 0 {
        return Some(ast);
    }

    let parse_result = catch_unwind(AssertUnwindSafe(|| {
        let mut ret = ast.ret;
        rule(&mut ast, &mut ret);
        ast.ret = ret;
    }));

    match parse_result {
        Ok(()) => {}
        Err(payload) => {
            if payload.is::<AbortSignal>() {
                ast.fail = 1;
            } else {
                resume_unwind(payload);
            }
        }
    }

    if ast.fail != 0 && ast.err_pos < ast.pos {
        ast.err_pos = ast.pos;
        ast.err_rule = Some(rulename.to_string());
    }

    let pos = ast.pos;
    ast_close(&mut ast, pos, open);
    if ast.nodes_cnt > 0 {
        let lastinfo = ast.lastinfo;
        ast.err_pos = -1;
        ast_setinfo(&mut ast, lastinfo, 0);
    }
    ast.mmz.clear();
    sync_counts(&mut ast);
    Some(ast)
}

/// Debug function for AST.
pub fn skp_debug2(ast: &mut Ast, d: u8) -> i32 {
    match d {
        0 => ast.flg &= !SKP_DEBUG,
        1 => ast.flg |= SKP_DEBUG,
        _ => ast.flg ^= SKP_DEBUG,
    }
    i32::from(ast.flg & SKP_DEBUG)
}

/// Returns the rule name at which an error occurred.
pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        None
    } else {
        ast.err_rule.as_deref()
    }
}

/// Returns the error position as a string pointer.
pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        None
    } else {
        Some(slice_from(&ast.start, ast.err_pos as usize))
    }
}

/// Returns the start of the error line.
pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return SKP_EMPTY;
    }
    let mut idx = ast.err_pos as usize;
    while idx > 0 {
        let prev = byte_at(&ast.start, idx - 1);
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        idx -= 1;
    }
    slice_from(&ast.start, idx)
}

/// Returns the error column number.
pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let line_start = ast.start.len() as i32 - line.len() as i32;
    ast.err_pos - line_start
}

/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    let mut ast = Ast {
        nodes: Vec::with_capacity(SKP_STARTNODES),
        mmz: Vec::with_capacity(64),
        par: Vec::with_capacity(SKP_STARTNODES * 2),
        err_msg: Some(SKP_EMPTY.to_string()),
        err_pos: -1,
        cur_node: ASTNULL,
        ..Ast::default()
    };
    sync_counts(&mut ast);
    Some(ast)
}

/// Frees an AST.
pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
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

    let open_idx = open as usize;
    let node_idx = ast.par.get(open_idx).copied().unwrap_or(-1);
    if node_idx < 0 {
        return -1;
    }

    if ast.fail != 0 {
        ast.pos = ast.nodes[node_idx as usize].from;
        ast.nodes.truncate(node_idx as usize);
        ast.par.truncate(open_idx);
        sync_counts(ast);
        return -1;
    }

    let par = ast_newpar(ast);
    if par < 0 {
        return -1;
    }

    let delta = par - open;
    let node = &mut ast.nodes[node_idx as usize];
    node.to = to;
    node.delta = delta;
    node.tag = 0;
    ast.par[par as usize] = -delta;
    ast.cur_node = par;
    ast.cur_rule = Some(node.rule.clone());
    par
}

/// Aborts parsing with the given message and rule.
#[allow(non_snake_case)]
pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    ast.err_msg = Some(msg.to_string());
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    panic_any(AbortSignal);
}

/// Records memoization of AST nodes (for left recursion etc.).
pub fn skp_memoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str, old_pos: i32, start_par: i32) {
    let mut start = start_par;
    let mut end = ast.par_cnt;
    if ast.fail != 0 || end <= start {
        start = -1;
        end = -1;
    }

    let numnodes = if start >= 0 && end >= 0 {
        (end - start) / 2
    } else {
        -1
    };

    mmz.pos = old_pos;
    mmz.endpos = ast.pos;
    mmz.numnodes = numnodes;
    mmz.lastinfo = ast.lastinfo;
    mmz.nodes.clear();

    if numnodes > 0 {
        for k in start as usize..end as usize {
            if ast.par[k] >= 0 {
                mmz.nodes.push(ast.nodes[ast.par[k] as usize].clone());
            }
        }
    }

    mmz.maxnodes = (mmz.nodes.len() + 1) as i32;
}

/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str) -> i32 {
    if mmz.maxnodes == 0 {
        return 0;
    }
    if mmz.pos != ast.pos {
        return 0;
    }

    let numnodes = mmz.numnodes;
    ast.fail = if numnodes < 0 { 1 } else { 0 };
    ast.lastpos = ast.pos;
    ast.pos = mmz.endpos;
    ast.lastinfo = mmz.lastinfo;

    if numnodes > 0 {
        let old_par_len = ast.par.len();
        ast.par
            .extend(std::iter::repeat_n(SKP_DELTA_MAX, (2 * numnodes) as usize));
        let mut cur_par = old_par_len;

        for node in &mmz.nodes {
            while ast.par[cur_par] != SKP_DELTA_MAX {
                cur_par += 1;
            }
            let new_idx = ast.nodes.len() as i32;
            ast.nodes.push(node.clone());
            ast.par[cur_par] = new_idx;
            let delta = node.delta as usize;
            ast.par[cur_par + delta] = -node.delta;
        }
        sync_counts(ast);
    }

    1
}

/// Sets AST node information.
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= 0 {
        return;
    }

    let node = if node == ASTNULL { ast.par_cnt - 1 } else { node };
    if let Some(idx) = node_info_index(ast, node) {
        ast.nodes[idx].tag = info;
    }
}

/// Records a new AST info node.
pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail != 0 {
        return;
    }
    let par = ast_open(ast, ast.pos, SKP_INFO_RULE);
    if par >= 0 {
        ast_close(ast, ast.pos, par);
        if let Some(idx) = node_info_index(ast, par) {
            ast.nodes[idx].tag = info;
        }
        ast.lastinfo = info;
    }
}

/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    node_info_index(ast, node)
        .map(|idx| ast.nodes[idx].tag)
        .unwrap_or(0)
}

/// Swaps the last two AST nodes.
pub fn ast_swap(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 {
        return;
    }

    let c1 = ast.par_cnt - 1;
    if ast.par[c1 as usize] >= 0 {
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

    let left = ast.par[o2 as usize..=c2 as usize].to_vec();
    let right = ast.par[o1 as usize..=c1 as usize].to_vec();
    ast.par.splice(o2 as usize..=c1 as usize, right.into_iter().chain(left));
    sync_counts(ast);
}

/// Lowers a node (wraps a group of nodes into a new parent).
pub fn ast_lower(ast: &mut Ast, rule: &str, f: AstNodeT, t: AstNodeT) {
    if ast.par_cnt <= f || ast.par_cnt <= t || f >= t {
        return;
    }

    let mut lft = f;
    let mut rgt = t;
    if ast.par[lft as usize] < 0 {
        lft += ast.par[lft as usize];
    }
    if ast.par[rgt as usize] < 0 {
        rgt += ast.par[rgt as usize];
    }

    let lft_idx = match node_info_index(ast, lft) {
        Some(idx) => idx,
        None => return,
    };
    let rgt_idx = match node_info_index(ast, rgt) {
        Some(idx) => idx,
        None => return,
    };

    let node_from = ast.nodes[lft_idx].from;
    let node_to = ast.nodes[rgt_idx].to;
    rgt += ast.nodes[rgt_idx].delta;

    let node = ast_newnode(ast);
    if node < 0 {
        return;
    }
    let delta = rgt - lft + 2;
    ast.nodes[node as usize] = AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta,
        tag: 0,
    };

    ast.par.insert(lft as usize, node);
    ast.par.insert((rgt + 2) as usize, -delta);
    sync_counts(ast);
}

/// Lifts a node (removes a level from the AST).
pub fn ast_lift(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 {
        return;
    }

    let c1 = ast.par_cnt - 1;
    let c2 = c1 - 1;
    if ast.par[c1 as usize] >= 0 || ast.par[c2 as usize] >= 0 {
        return;
    }

    let o1 = c1 + ast.par[c1 as usize];
    let o2 = c2 + ast.par[c2 as usize];
    if o1 < 0 || o2 < 0 || ast.par[o1 as usize] < 0 || ast.par[o2 as usize] < 0 {
        return;
    }
    if o2 != o1 + 1 {
        return;
    }

    let idx = ast.par[o1 as usize] as usize;
    if ast.nodes[idx].tag == 0 {
        ast.par.remove(c1 as usize);
        ast.par.remove(o1 as usize);
        sync_counts(ast);
    }
}

/// Lifts all single-child nodes.
pub fn ast_lift_all(ast: &mut Ast) {
    loop {
        let before = ast.par_cnt;
        ast_lift(ast);
        if ast.par_cnt == before {
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
    if ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 >= 0 && ast.par[o1 as usize] >= 0 && c1 == o1 + 1 {
        ast.par.truncate(o1 as usize);
        sync_counts(ast);
    }
}

/// Removes the last empty leaf node.
pub fn ast_noemptyleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 || c1 != o1 + 1 {
        return;
    }
    let idx = ast.par[o1 as usize] as usize;
    if ast.nodes[idx].from == ast.nodes[idx].to {
        ast.par.truncate(o1 as usize);
        sync_counts(ast);
    }
}

/// Returns the index of the last AST node.
pub fn ast_lastnode(ast: &Ast) -> AstNodeT {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return ASTNULL;
    }
    let c1 = ast.par_cnt - 1;
    if ast.par[c1 as usize] >= 0 {
        return ASTNULL;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        ASTNULL
    } else {
        o1
    }
}

/// Checks if the last node is empty.
pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == ASTNULL {
        return false;
    }
    if let Some(idx) = node_info_index(ast, node) {
        ast.nodes[idx].from == ast.nodes[idx].to
    } else {
        false
    }
}

/// Deletes the last node.
pub fn ast_delete(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 >= 0 && ast.par[o1 as usize] >= 0 {
        ast.par.truncate(o1 as usize);
        sync_counts(ast);
    }
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
    node + ast.par[node as usize]
}

/// Returns the “right” sibling of a node.
pub fn astright(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    if ast.par[node as usize] > 0 {
        let idx = ast.par[node as usize] as usize;
        node += ast.nodes[idx].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        ASTNULL
    } else {
        node
    }
}

/// Returns the parent of a node.
pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let node = astfirst(ast, node);
    if node == ASTNULL {
        return ASTNULL;
    }
    let parent = node - 1;
    if parent < 0 || ast.par[parent as usize] < 0 {
        ASTNULL
    } else {
        parent
    }
}

/// Returns the first child of a node.
pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let child = node + 1;
    if child >= ast.par_cnt || ast.par[child as usize] < 0 {
        ASTNULL
    } else {
        child
    }
}

/// Returns the leftmost sibling (first child) of a node.
pub fn astfirst(ast: &Ast, mut node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    loop {
        let next = astleft(ast, node);
        if next == ASTNULL {
            return node;
        }
        node = next;
    }
}

/// Returns the rightmost sibling of a node.
pub fn astlast(ast: &Ast, mut node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    loop {
        let next = astright(ast, node);
        if next == ASTNULL {
            return node;
        }
        node = next;
    }
}

/// Returns the next node in a depth-first traversal.
pub fn astnextdf(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let node = node + 1;
    if node < 0 {
        0
    } else if node >= ast.par_cnt {
        ASTNULL
    } else {
        node
    }
}

/// Checks if the given index is an entry (open parenthesis) node.
pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    (0..ast.par_cnt).contains(&node) && ast.par[node as usize] >= 0
}

/// Checks if the given index is an exit (closing parenthesis) node.
pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    (0..ast.par_cnt).contains(&node) && ast.par[node as usize] < 0
}

/// Returns the rule name associated with a node.
pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    node_info_index(ast, node)
        .map(|idx| ast.nodes[idx].rule.as_str())
        .unwrap_or(SKP_EMPTY)
}

/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    node_info_index(ast, node)
        .map(|idx| slice_from(&ast.start, ast.nodes[idx].from as usize))
        .unwrap_or(SKP_EMPTY)
}

/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    node_info_index(ast, node)
        .map(|idx| slice_from(&ast.start, ast.nodes[idx].to as usize))
        .unwrap_or(SKP_EMPTY)
}

/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    node_info_index(ast, node)
        .map(|idx| ast.nodes[idx].to - ast.nodes[idx].from)
        .unwrap_or(0)
}

/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    node_info_index(ast, node)
        .map(|idx| ast.nodes[idx].delta == 1)
        .unwrap_or(false)
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
    if ast_is(ast, node, r1) != 0
        || r2.map(|r| ast_is(ast, node, r) != 0).unwrap_or(false)
        || r3.map(|r| ast_is(ast, node, r) != 0).unwrap_or(false)
        || r4.map(|r| ast_is(ast, node, r) != 0).unwrap_or(false)
        || r5.map(|r| ast_is(ast, node, r) != 0).unwrap_or(false)
    {
        1
    } else {
        0
    }
}

/// Checks if a node’s rule matches a given rule.
pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node_info_index(ast, node)
        .map(|idx| ast.nodes[idx].rule == rulename)
        .unwrap_or(false)
    {
        1
    } else {
        0
    }
}

/// Checks if the AST contains an error.
pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

/// Prints the AST in s-expression format.
pub fn astprintsexpr(ast: &Ast, f: &mut dyn Write) {
    let mut node = ASTNULL;
    while {
        node = astnextdf(ast, node);
        node != ASTNULL
    } {
        if astisnodeentry(ast, node) {
            let _ = write!(f, "({} ", astnoderule(ast, node));
            if astisleaf(ast, node) {
                let _ = write!(f, "'");
                if astnoderule(ast, node) == SKP_INFO_RULE {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else if let Some(idx) = node_info_index(ast, node) {
                    let text = slice_range(
                        &ast.start,
                        ast.nodes[idx].from as usize,
                        ast.nodes[idx].to as usize,
                    );
                    for ch in text.chars() {
                        if ch == '\'' {
                            let _ = write!(f, "\\");
                        }
                        let _ = write!(f, "{ch}");
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
pub fn astprinttree(ast: &Ast, f: &mut dyn Write) {
    let mut node = ASTNULL;
    let mut level = 0i32;
    while {
        node = astnextdf(ast, node);
        node != ASTNULL
    } {
        if astisnodeentry(ast, node) {
            for _ in (0..level).step_by(4) {
                let _ = write!(f, "    ");
            }
            let _ = write!(f, "[{}", astnoderule(ast, node));
            let tag = astnodeinfo(ast, node);
            if tag != 0 {
                let _ = write!(f, " ({tag})");
            }
            let _ = write!(f, "]");
            level += 4;
            if astisleaf(ast, node) {
                let _ = write!(f, " '");
                if let Some(idx) = node_info_index(ast, node) {
                    let text = slice_range(
                        &ast.start,
                        ast.nodes[idx].from as usize,
                        ast.nodes[idx].to as usize,
                    );
                    for ch in text.chars() {
                        if ch == '\'' {
                            let _ = write!(f, "\\");
                        }
                        let _ = write!(f, "{ch}");
                    }
                }
                let _ = write!(f, "'");
            }
            let _ = writeln!(f);
        } else {
            level -= 4;
        }
    }
}

pub const ASTNULL: i32 = -1;
