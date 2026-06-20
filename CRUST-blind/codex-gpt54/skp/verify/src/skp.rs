use std::any::Any;
use std::io::Write;

/// SKP version information.
pub const SKP_VER: u32 = 0x0003001C;
pub const SKP_VER_STR: &str = "0.3.1rc";

const SKP_DEBUG: i8 = 0x01;
const SKP_STARTNODES: i32 = 8;
const ASTNULL: i32 = -1;
const SKP_DELTA_MAX: i32 = i32::MAX;
const SKP_N_INFO: &str = "#";
const SKP_MSG_NONE: &str = "";

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
    if start.len() < to.len() || &start[start.len() - to.len()..] != to {
        return 0;
    }
    let ret = (start.len() - to.len()) as i32;
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
pub fn skptrace(args: std::fmt::Arguments) {
    eprintln!("TRCE: {}", args);
}

fn slice_from_idx(s: &str, mut idx: usize) -> &str {
    if idx > s.len() {
        idx = s.len();
    }
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    &s[idx..]
}

fn slice_range(s: &str, mut start: usize, mut end: usize) -> &str {
    if start > s.len() {
        start = s.len();
    }
    if end > s.len() {
        end = s.len();
    }
    while start < s.len() && !s.is_char_boundary(start) {
        start += 1;
    }
    while end < s.len() && !s.is_char_boundary(end) {
        end += 1;
    }
    if start > end {
        ""
    } else {
        &s[start..end]
    }
}

fn next_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx + 1;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn first_byte(s: &str) -> u8 {
    s.as_bytes().first().copied().unwrap_or(0)
}

fn skip_pattern_spaces(mut pat: &str) -> &str {
    while !pat.is_empty() && is_space(first_byte(pat) as u32) {
        pat = &pat[1..];
    }
    pat
}

fn skp_next_idx(s: &str, idx: usize, iso: i32) -> (u32, usize) {
    let bytes = s.as_bytes();
    if idx >= bytes.len() {
        return (0, idx);
    }

    let mut next = idx;
    let mut c = bytes[next] as u32;
    next += 1;

    if iso == 0 {
        if next < bytes.len() && (bytes[next] & 0xC0) == 0x80 {
            c = (c << 8) | bytes[next] as u32;
            next += 1;
            if next < bytes.len() && (bytes[next] & 0xC0) == 0x80 {
                c = (c << 8) | bytes[next] as u32;
                next += 1;
                if next < bytes.len() && (bytes[next] & 0xC0) == 0x80 {
                    c = (c << 8) | bytes[next] as u32;
                    next += 1;
                }
            }
        }
    }

    if c == 0x0D && next < bytes.len() && bytes[next] == 0x0A {
        c = 0x0D0A;
        next += 1;
    }

    (c, next)
}

fn get_next_byte(src: &str, s_end: &mut usize, s_tmp: &mut usize, s_chr: &mut u32) {
    let bytes = src.as_bytes();
    *s_end = *s_tmp;
    if *s_end < bytes.len() {
        *s_chr = bytes[*s_end] as u32;
        *s_tmp = *s_end + 1;
    } else {
        *s_chr = 0;
    }
}

fn is_string_bytes(s: &str, p: &str, len: usize, flg: i32) -> i32 {
    let mut start = 0usize;
    let mut s_idx = 0usize;
    let mut p_idx = 0usize;
    let mut mlen = 0i32;
    let mut remaining = len;

    while remaining > 0 {
        if p.as_bytes().get(p_idx) == Some(&0x0E) {
            return mlen;
        }

        let (p_chr, p_end) = skp_next_idx(p, p_idx, flg & 2);
        let (s_chr, s_end) = skp_next_idx(s, s_idx, flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += (s_end - s_idx) as i32;
            remaining = remaining.saturating_sub(p_end - p_idx);
            p_idx = p_end;
            s_idx = s_end;
        } else {
            while remaining > 0 {
                if p.as_bytes().get(p_idx) == Some(&0x0E) {
                    p_idx += 1;
                    remaining -= 1;
                    break;
                }
                p_idx += 1;
                remaining -= 1;
            }
            if remaining == 0 && p.as_bytes().get(p_idx.wrapping_sub(1)) != Some(&0x0E) {
                return 0;
            }
            s_idx = start;
            mlen = 0;
        }
        start = 0;
    }

    mlen
}

fn normalize_open(ast: &Ast, mut node: i32) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return ASTNULL;
    }
    let v = ast.par[node as usize];
    if v < 0 {
        node += v;
    }
    if node < 0 || node >= ast.par_cnt || ast.par[node as usize] < 0 {
        ASTNULL
    } else {
        node
    }
}

fn ast_newpar(ast: &mut Ast) -> i32 {
    ast.par.push(0);
    ast.par_cnt += 1;
    ast.par_cnt - 1
}

fn ast_newnode(ast: &mut Ast) -> i32 {
    ast.nodes.push(AstNode::default());
    ast.nodes_cnt += 1;
    ast.nodes_cnt - 1
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
    let mut start_idx = 0usize;
    let mut s_idx = start_idx;
    let mut pat_idx = 0usize;
    let mut skp_to = false;
    let mut matched = 0i32;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg = 0i32;

    if pat.is_empty() {
        return (0, src, src);
    }

    if pat.as_bytes()[0] == b'>' {
        skp_to = true;
        pat_idx = 1;
    }

    let mut p = skip_pattern_spaces(slice_from_idx(pat, pat_idx));
    pat_idx = pat.len() - p.len();

    while first_byte(slice_from_idx(pat, pat_idx)) > 7 {
        let (m, s_end, p_end) = match_pat(slice_from_idx(pat, pat_idx), slice_from_idx(src, s_idx), &mut flg);
        matched = m;

        if matched != 0 {
            s_idx = src.len() - s_end.len();
            pat_idx = pat.len() - p_end.len();
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s_idx);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s_idx);
            }
        } else {
            while first_byte(slice_from_idx(pat, pat_idx)) > 7 {
                pat_idx += 1;
            }
            if pat_idx < pat.len() && pat_idx + 1 < pat.len() {
                s_idx = start_idx;
                pat_idx += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                pat_idx = if pat.starts_with('>') { 1 } else { 0 };
                start_idx = next_char_boundary(src, start_idx);
                s_idx = start_idx;
                if start_idx >= src.len() {
                    break;
                }
            } else {
                break;
            }
        }

        p = skip_pattern_spaces(slice_from_idx(pat, pat_idx));
        pat_idx = pat.len() - p.len();
    }

    if matched == 0 {
        if let Some(gn) = goalnot {
            goal = Some(gn);
            matched = MATCHED;
            pat_idx = pat.len();
        }
    }

    if let Some(g) = goal {
        s_idx = g;
    }

    if matched != 0 && first_byte(slice_from_idx(pat, pat_idx)) <= 7 {
        let ret = if pat_idx < pat.len() {
            pat.as_bytes()[pat_idx] as i32
        } else {
            1
        };
        let to_idx = if skp_to { start_idx } else { s_idx };
        return (ret, slice_from_idx(src, to_idx), slice_from_idx(src, s_idx));
    }

    (0, src, src)
}

/// In the C header a set of macros provides variants:
///   skp(src, pat), skp(src, pat, end) and skp(src, pat, to, end).
///
/// The following functions mimic those overloads.
pub fn skp_4<'a>(src: &'a str, pat: &str, to: Option<&mut &'a str>, end: Option<&mut &'a str>) -> i32 {
    let (ret, to_s, end_s) = skp_(src, pat);
    if let Some(out) = to {
        *out = to_s;
    }
    if let Some(out) = end {
        *out = end_s;
    }
    ret
}

pub fn skp_3<'a>(src: &'a str, pat: &str, end: Option<&mut &'a str>) -> i32 {
    let (ret, _, end_s) = skp_(src, pat);
    if let Some(out) = end {
        *out = end_s;
    }
    ret
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_(src, pat).0
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let (c, next) = skp_next_idx(s, 0, iso);
    (c, slice_from_idx(s, next))
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
/// (Corresponds to `chr_cmp`.)
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        return (a as u8).to_ascii_lowercase() == (b as u8).to_ascii_lowercase();
    }
    a == b
}

/// Returns true if `c` is a blank character.
/// (Corresponds to `is_blank`.)
pub fn is_blank(c: u32) -> bool {
    if c < 0xFF {
        return c == 0x20 || c == 0x09;
    }
    match c & 0xFFFFFF00 {
        0x00000000 => c == 0xA0,
        0x0000C200 => c == 0xC2A0,
        0x00E19A00 => c == 0xE19A80,
        0x00E28000 => (0xE28080..=0xE2808A).contains(&c) || c == 0xE280AF,
        0x00E38000 => c == 0xE38080,
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
    is_digit(c) || (b'A' as u32..=b'F' as u32).contains(&c) || (b'a' as u32..=b'f' as u32).contains(&c)
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
    is_alnum(c) || c == b'_' as u32
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

    let mut idx = 0usize;
    let (mut p_ch, next) = skp_next_idx(set, idx, iso);
    idx = next;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        let (c, n) = skp_next_idx(set, idx, iso);
        p_ch = c;
        idx = n;
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (c, n) = skp_next_idx(set, idx, iso);
        p_ch = c;
        idx = n;
        if p_ch == b'-' as u32 && set.as_bytes().get(idx) != Some(&b']') {
            let (c2, n2) = skp_next_idx(set, idx, iso);
            p_ch = c2;
            idx = n2;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (c3, n3) = skp_next_idx(set, idx, iso);
            p_ch = c3;
            idx = n3;
        }
    }

    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    if len <= 0 {
        0
    } else {
        is_string_bytes(s, p, len as usize, flg)
    }
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    if open == b'(' as u32 {
        b')' as u32
    } else if open == b'[' as u32 {
        b']' as u32
    } else if open == b'{' as u32 {
        b'}' as u32
    } else if open == b'<' as u32 {
        b'>' as u32
    } else {
        0
    }
}

/// Given a quote character, returns the corresponding closing quote.
pub fn get_qclose(open: u32) -> u32 {
    if open == b'\'' as u32 || open == b'"' as u32 || open == b'`' as u32 {
        open
    } else {
        0
    }
}

/// Constants for match results.
pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`. The flag parameter is passed by mutable reference.
fn run_match_w(
    src: &str,
    s_chr: &mut u32,
    s_end: &mut usize,
    s_tmp: &mut usize,
    match_max: u32,
    match_min: u32,
    match_not: bool,
    flg: i32,
    pred: &dyn Fn(u32) -> bool,
) -> i32 {
    let mut cnt = 0u32;
    while cnt < match_max && *s_chr != 0 && (pred(*s_chr) != match_not) {
        cnt += 1;
        *s_end = *s_tmp;
        let next = skp_next_idx(src, *s_end, flg & 2);
        *s_chr = next.0;
        *s_tmp = next.1;
    }
    if cnt >= match_min {
        MATCHED
    } else {
        MATCHED_FAIL
    }
}

pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let mut pat_idx = 0usize;
    let pat_bytes = pat.as_bytes();
    let mut s_end = 0usize;
    let (mut s_chr, mut s_tmp) = skp_next_idx(src, s_end, *flg & 2);
    let mut ret = MATCHED_FAIL;
    let mut match_min = 1u32;
    let mut match_max = 1u32;
    let mut match_not = false;

    if pat_bytes.get(pat_idx) == Some(&b'*') {
        match_min = 0;
        match_max = u32::MAX;
        pat_idx += 1;
    } else if pat_bytes.get(pat_idx) == Some(&b'+') {
        match_max = u32::MAX;
        pat_idx += 1;
    } else if pat_bytes.get(pat_idx) == Some(&b'?') {
        match_min = 0;
        pat_idx += 1;
    }

    if pat_bytes.get(pat_idx) == Some(&b'!') {
        match_not = true;
        pat_idx += 1;
    }

    let Some(&op) = pat_bytes.get(pat_idx) else {
        return (MATCHED_FAIL, src, pat);
    };
    pat_idx += 1;

    match op {
        b'.' => {
            if match_not {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &|c| c != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_break);
            }
        }
        b'n' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_break),
        b'd' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_digit),
        b'x' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_xdigit),
        b'a' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_alpha),
        b'u' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_upper),
        b'l' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_lower),
        b's' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_space),
        b'w' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_blank),
        b'c' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_ctrl),
        b'i' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_idchr),
        b'@' => ret = run_match_w(src, &mut s_chr, &mut s_end, &mut s_tmp, match_max, match_min, match_not, *flg, &is_alnum),
        b'&' => {
            ret = if match_not { MATCHED_GOALNOT } else { MATCHED_GOAL };
        }
        b'[' => {
            ret = run_match_w(
                src,
                &mut s_chr,
                &mut s_end,
                &mut s_tmp,
                match_max,
                match_min,
                match_not,
                *flg,
                &|c| is_oneof(c, slice_from_idx(pat, pat_idx), *flg & 2),
            );
            if pat_bytes.get(pat_idx) == Some(&b']') {
                pat_idx += 1;
            }
            while pat_idx < pat.len() && pat_bytes[pat_idx] != b']' {
                pat_idx += 1;
            }
            if pat_idx < pat.len() {
                pat_idx += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = op;
            let mut l = 0usize;
            while pat_idx + l < pat.len() && pat_bytes[pat_idx + l] != quote {
                l += 1;
            }
            let ml = is_string_bytes(slice_from_idx(src, s_end), slice_range(pat, pat_idx, pat_idx + l), l, *flg);
            if l > 0 && ml > 0 {
                if !match_not {
                    s_end += ml as usize;
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not {
                ret = MATCHED;
            }
            pat_idx += l + usize::from(pat_idx + l < pat.len());
        }
        b'C' => {
            *flg = (*flg & !1) | i32::from(match_not);
            ret = MATCHED;
        }
        b'U' => {
            *flg = (*flg & !2) | (i32::from(match_not) * 2);
            ret = MATCHED;
        }
        b'S' => {
            while is_space(s_chr) {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            ret = MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !is_break(s_chr) {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            if s_chr != 0 {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            ret = MATCHED;
        }
        b'I' => {
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            if pat_bytes.get(pat_idx) == Some(&b')') && s_chr == b'(' as u32 {
                pat_idx += 1;
                let open = s_chr;
                let close = get_close(open);
                if close != 0 {
                    let mut count = 1i32;
                    while s_chr != 0 && count > 0 {
                        get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                        if s_chr == open {
                            count += 1;
                        }
                        if s_chr == close {
                            count -= 1;
                        }
                    }
                    if count == 0 {
                        get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                        ret = MATCHED;
                    }
                }
            } else {
                ret = MATCHED_FAIL;
            }
        }
        b'B' => {
            let open = s_chr;
            let close = get_close(open);
            if close != 0 {
                let mut count = 1i32;
                while s_chr != 0 && count > 0 {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if s_chr == open {
                        count += 1;
                    }
                    if s_chr == close {
                        count -= 1;
                    }
                }
                if count == 0 {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    ret = MATCHED;
                }
            }
        }
        b'Q' => {
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == b'\\' as u32 {
                        get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    }
                }
                if s_chr != 0 {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            let bytes = src.as_bytes();
            if s_chr == b'0' as u32
                && s_end + 2 < bytes.len()
                && (bytes[s_end + 1] == b'x' || bytes[s_end + 1] == b'X')
                && is_xdigit(bytes[s_end + 2] as u32)
            {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
        }
        b'D' => {
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if !is_space(s_chr) {
                        break;
                    }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
        }
        b'F' => {
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                    if !is_space(s_chr) {
                        break;
                    }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            if s_chr == b'.' as u32 {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
            }
            if ret == MATCHED && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                }
                while is_digit(s_chr) {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                }
                if s_chr == b'.' as u32 {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                }
                while is_digit(s_chr) {
                    get_next_byte(src, &mut s_end, &mut s_tmp, &mut s_chr);
                }
            }
        }
        _ => {
            ret = MATCHED_FAIL;
            pat_idx = pat_idx.saturating_sub(1);
        }
    }

    if ret != MATCHED_FAIL {
        (
            ret,
            slice_from_idx(src, s_end),
            slice_from_idx(pat, pat_idx),
        )
    } else {
        (MATCHED_FAIL, src, pat)
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
#[derive(Default)]
pub struct Ast {
    pub start: String,
    pub err_rule: Option<String>,
    pub err_msg: Option<String>,
    pub cur_rule: Option<String>,
    pub nodes: Vec<AstNode>,
    pub mmz: Vec<AstMmz>,
    pub par: Vec<i32>,
    pub auxptr: Option<Box<dyn Any>>,
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

impl std::fmt::Debug for Ast {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Ast")
            .field("start", &self.start)
            .field("err_rule", &self.err_rule)
            .field("err_msg", &self.err_msg)
            .field("cur_rule", &self.cur_rule)
            .field("nodes", &self.nodes)
            .field("mmz", &self.mmz)
            .field("par", &self.par)
            .field("nodes_cnt", &self.nodes_cnt)
            .field("nodes_max", &self.nodes_max)
            .field("par_cnt", &self.par_cnt)
            .field("par_max", &self.par_max)
            .field("mmz_cnt", &self.mmz_cnt)
            .field("mmz_max", &self.mmz_max)
            .field("pos", &self.pos)
            .field("lastpos", &self.lastpos)
            .field("err_pos", &self.err_pos)
            .field("cur_node", &self.cur_node)
            .field("lastinfo", &self.lastinfo)
            .field("ret", &self.ret)
            .field("depth", &self.depth)
            .field("fail", &self.fail)
            .field("flg", &self.flg)
            .finish()
    }
}

/// A function pointer type for parsing rules.
/// (In C: `typedef void (*skprule_t)(ast_t, int32_t *);`)
pub type SkpRule = fn(ast: &mut Ast, ret: &mut i32);

/// Parses the source string `src` using a given parsing rule.
/// (Corresponds to `ast_t skp_parse(char *src, skprule_t rule, char *rulename, int debug);`)
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
    if open >= 0 {
        let mut ret = 0;
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
            let lastinfo = ast.lastinfo;
            ast_setinfo(&mut ast, lastinfo, 0);
        }
    }

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
        Some(slice_from_idx(&ast.start, ast.err_pos as usize))
    }
}

/// Returns the start of the error line.
pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let mut idx = ast.err_pos as usize;
    let bytes = ast.start.as_bytes();
    while idx > 0 {
        if bytes[idx - 1] == b'\n' || bytes[idx - 1] == b'\r' {
            break;
        }
        idx -= 1;
    }
    slice_from_idx(&ast.start, idx)
}

/// Returns the error column number.
pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    (ast.start.len() - slice_from_idx(&ast.start, ast.err_pos as usize).len() - (ast.start.len() - line.len())) as i32
}

/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    Some(Ast {
        start: String::new(),
        err_rule: None,
        err_msg: Some(SKP_MSG_NONE.to_string()),
        cur_rule: None,
        nodes: Vec::with_capacity(SKP_STARTNODES as usize),
        mmz: Vec::with_capacity(64),
        par: Vec::with_capacity((SKP_STARTNODES * 2) as usize),
        auxptr: None,
        nodes_cnt: 0,
        nodes_max: SKP_STARTNODES,
        par_cnt: 0,
        par_max: SKP_STARTNODES * 2,
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

/// Opens a new AST node starting at position `from` with the given rule.
pub fn ast_open(ast: &mut Ast, from: i32, rule: &str) -> i32 {
    if ast.fail != 0 {
        return -1;
    }
    let par = ast_newpar(ast);
    let node = ast_newnode(ast);
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
    if open < 0 || open >= ast.par_cnt {
        return -1;
    }
    let node_idx = ast.par[open as usize];
    if ast.fail != 0 {
        let from = ast.nodes.get(node_idx as usize).map(|n| n.from).unwrap_or(ast.pos);
        ast.pos = from;
        ast.nodes.truncate(node_idx as usize);
        ast.nodes_cnt = node_idx;
        ast.par.truncate(open as usize);
        ast.par_cnt = open;
        return -1;
    }

    let par = ast_newpar(ast);
    if let Some(nd) = ast.nodes.get_mut(node_idx as usize) {
        nd.to = to;
        nd.delta = par - open;
        nd.tag = 0;
        ast.par[par as usize] = -nd.delta;
        ast.cur_node = par;
        ast.cur_rule = Some(nd.rule.clone());
    }
    par
}

/// Aborts parsing with the given message and rule.
#[allow(non_snake_case)]
pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    if !msg.is_empty() {
        ast.err_msg = Some(msg.to_string());
    }
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
}

/// Records memoization of AST nodes (for left recursion etc.).
pub fn skp_memoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str, old_pos: i32, mut start_par: i32) {
    let mut end_par = ast.par_cnt;
    if ast.fail != 0 || end_par <= start_par {
        start_par = -1;
        end_par = -1;
    }
    let numnodes = (end_par - start_par) / 2;

    mmz.pos = old_pos;
    mmz.endpos = ast.pos;
    mmz.numnodes = if ast.fail != 0 { -1 } else { numnodes };
    mmz.maxnodes = mmz.maxnodes.max(numnodes);
    mmz.lastinfo = ast.lastinfo;
    mmz.nodes.clear();

    if start_par >= 0 {
        for k in start_par..end_par {
            let par_val = ast.par[k as usize];
            if par_val >= 0 {
                mmz.nodes.push(ast.nodes[par_val as usize].clone());
            }
        }
    }
}

/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str) -> i32 {
    if mmz.nodes.is_empty() && mmz.numnodes == 0 && mmz.pos == 0 && mmz.endpos == 0 && mmz.lastinfo == 0 {
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
        let add_par = (2 * numnodes) as usize;
        ast.par.extend(std::iter::repeat(SKP_DELTA_MAX).take(add_par));
        let mut cur_par = ast.par_cnt as usize;

        for node in mmz.nodes.iter().take(numnodes as usize) {
            ast.nodes.push(node.clone());
            let node_idx = ast.nodes_cnt;
            ast.nodes_cnt += 1;

            while ast.par[cur_par] != SKP_DELTA_MAX {
                cur_par += 1;
            }
            ast.par[cur_par] = node_idx;
            let delta = node.delta as usize;
            if cur_par + delta < ast.par.len() {
                ast.par[cur_par + delta] = -node.delta;
            }
        }
        ast.par_cnt += 2 * numnodes;
    }

    1
}

/// Sets AST node information.
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= 0 {
        return;
    }
    let mut node = if node == ASTNULL { ast.par_cnt - 1 } else { node };
    if node < 0 || node >= ast.par_cnt {
        return;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node >= 0 && node < ast.par_cnt {
        let idx = ast.par[node as usize];
        if idx >= 0 {
            ast.nodes[idx as usize].tag = info;
        }
    }
}

/// Records a new AST info node.
pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail == 0 {
        let par = ast_open(ast, ast.pos, SKP_N_INFO);
        ast_close(ast, ast.pos, par);
        if par >= 0 {
            let idx = ast.par[par as usize];
            ast.nodes[idx as usize].tag = info;
            ast.lastinfo = info;
        }
    }
}

/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    let node = normalize_open(ast, node);
    if node == ASTNULL {
        return 0;
    }
    let idx = ast.par[node as usize];
    ast.nodes.get(idx as usize).map(|n| n.tag).unwrap_or(0)
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

    let mut new_par = Vec::with_capacity(ast.par.len());
    new_par.extend_from_slice(&ast.par[..o2 as usize]);
    new_par.extend_from_slice(&ast.par[o1 as usize..=c1 as usize]);
    new_par.extend_from_slice(&ast.par[o2 as usize..=c2 as usize]);
    if (c1 as usize) + 1 < ast.par.len() {
        new_par.extend_from_slice(&ast.par[(c1 as usize) + 1..]);
    }
    ast.par = new_par;
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

    let left_idx = ast.par[lft as usize];
    let right_idx = ast.par[rgt as usize];
    let node_from = ast.nodes[left_idx as usize].from;
    let node_to = ast.nodes[right_idx as usize].to;
    rgt += ast.nodes[right_idx as usize].delta;

    let node = ast_newnode(ast);
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
    ast.par_cnt += 2;
}

/// Lifts a node (removes a level from the AST).
pub fn ast_lift(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 {
        return;
    }
    let c1 = ast.par_cnt - 1;
    if ast.par[c1 as usize] >= 0 {
        return;
    }
    let c2 = c1 - 1;
    if c2 < 0 || ast.par[c2 as usize] >= 0 {
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
    let outer_idx = ast.par[o1 as usize];
    if ast.nodes[outer_idx as usize].tag == 0 {
        ast.par.remove(c1 as usize);
        ast.par.remove(o1 as usize);
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
    if ast.par[c1 as usize] >= 0 {
        return;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    if c1 == o1 + 1 {
        ast.par.truncate((ast.par_cnt - 2) as usize);
        ast.par_cnt -= 2;
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
    let idx = ast.par[o1 as usize];
    if ast.nodes[idx as usize].from == ast.nodes[idx as usize].to {
        ast.par.truncate((ast.par_cnt - 2) as usize);
        ast.par_cnt -= 2;
    }
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
    let idx = ast.par[node as usize];
    ast.nodes[idx as usize].from == ast.nodes[idx as usize].to
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
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return;
    }
    ast.par.truncate(o1 as usize);
    ast.par_cnt = o1;
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
        let ndx = ast.par[node as usize];
        node += ast.nodes[ndx as usize].delta;
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
    let node = node - 1;
    if node < 0 || ast.par[node as usize] < 0 {
        ASTNULL
    } else {
        node
    }
}

/// Returns the first child of a node.
pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let node = node + 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        ASTNULL
    } else {
        node
    }
}

/// Returns the leftmost sibling (first child) of a node.
pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    loop {
        let n = astleft(ast, node);
        if n == ASTNULL {
            break;
        }
        node = n;
    }
    node
}

/// Returns the rightmost sibling of a node.
pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    loop {
        let n = astright(ast, node);
        if n == ASTNULL {
            break;
        }
        node = n;
    }
    node
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
    node >= 0 && node < ast.par_cnt && ast.par[node as usize] >= 0
}

/// Checks if the given index is an exit (closing parenthesis) node.
pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node >= 0 && node < ast.par_cnt && ast.par[node as usize] < 0
}

/// Returns the rule name associated with a node.
pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    let node = normalize_open(ast, node);
    if node == ASTNULL {
        ""
    } else {
        let idx = ast.par[node as usize];
        ast.nodes.get(idx as usize).map(|n| n.rule.as_str()).unwrap_or("")
    }
}

/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    let node = normalize_open(ast, node);
    if node == ASTNULL {
        ""
    } else {
        let idx = ast.par[node as usize];
        let from = ast.nodes[idx as usize].from.max(0) as usize;
        slice_from_idx(&ast.start, from)
    }
}

/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    let node = normalize_open(ast, node);
    if node == ASTNULL {
        ""
    } else {
        let idx = ast.par[node as usize];
        let to = ast.nodes[idx as usize].to.max(0) as usize;
        slice_from_idx(&ast.start, to)
    }
}

/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    let node = normalize_open(ast, node);
    if node == ASTNULL {
        0
    } else {
        let idx = ast.par[node as usize];
        ast.nodes[idx as usize].to - ast.nodes[idx as usize].from
    }
}

/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    let node = normalize_open(ast, node);
    if node == ASTNULL {
        false
    } else {
        let idx = ast.par[node as usize];
        ast.nodes[idx as usize].delta == 1
    }
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
    i32::from(
        ast_is(ast, node, r1) != 0
            || r2.is_some_and(|r| ast_is(ast, node, r) != 0)
            || r3.is_some_and(|r| ast_is(ast, node, r) != 0)
            || r4.is_some_and(|r| ast_is(ast, node, r) != 0)
            || r5.is_some_and(|r| ast_is(ast, node, r) != 0),
    )
}

/// Checks if a node’s rule matches a given rule.
pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    i32::from(astnoderule(ast, node) == rulename)
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
                if astnoderule(ast, node) == SKP_N_INFO {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from = astnodefrom(ast, node);
                    let to = astnodeto(ast, node);
                    let len = from.len().saturating_sub(to.len());
                    let piece = &from[..len];
                    for ch in piece.chars() {
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
pub fn astprinttree(ast: &Ast, f: &mut dyn Write) {
    let mut node = ASTNULL;
    let mut levl = 0i32;
    while {
        node = astnextdf(ast, node);
        node != ASTNULL
    } {
        if astisnodeentry(ast, node) {
            for _ in (0..levl).step_by(4) {
                let _ = write!(f, "    ");
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
                let from = astnodefrom(ast, node);
                let to = astnodeto(ast, node);
                let len = from.len().saturating_sub(to.len());
                let piece = &from[..len];
                for ch in piece.chars() {
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
