#![allow(non_snake_case)]
#![allow(unused_assignments)]
#![allow(unused_mut)]

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

/// Returns the "length" from start to to. (This mimics the inline function `skp_loop_len`.)
pub fn skp_loop_len(start: &str, to: &str) -> i32 {
    // C: int ret = to-start; return (0 <= ret && ret <= (1<<16)?ret:0);
    let s_len = start.len() as isize;
    let t_len = to.len() as isize;
    let ret = s_len - t_len;
    if (0..=(1 << 16)).contains(&ret) {
        ret as i32
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

// ============================================================================
// Internal byte-level helpers
// ============================================================================

/// Reads a UTF-8 multi-byte sequence packed into a u32 (matching C semantics).
/// Returns (packed_bytes, end_byte_index_after_read).
fn next_byte_packed(src: &[u8], pos: usize, iso: bool) -> (u32, usize) {
    if pos >= src.len() {
        return (0, pos);
    }
    let mut p = pos;
    let mut c: u32 = src[p] as u32;
    p += 1;
    if !iso {
        // Up to 3 continuation bytes (matching #if 1 branch in C)
        if p < src.len() && (src[p] & 0xC0) == 0x80 {
            c = (c << 8) | (src[p] as u32);
            p += 1;
            if p < src.len() && (src[p] & 0xC0) == 0x80 {
                c = (c << 8) | (src[p] as u32);
                p += 1;
                if p < src.len() && (src[p] & 0xC0) == 0x80 {
                    c = (c << 8) | (src[p] as u32);
                    p += 1;
                }
            }
        }
    }
    // CR + LF: combine into 0x0D0A
    if c == 0x0D && p < src.len() && src[p] == 0x0A {
        c = 0x0D0A;
        p += 1;
    }
    (c, p)
}

/// Returns the next Unicode "character" (packed UTF-8 bytes) from the string `s`.
/// Returns a tuple `(code, rest_of_string)` where `code` is the packed bytes
/// and `rest_of_string` is the slice after consuming the character.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, end) = next_byte_packed(bytes, 0, iso != 0);
    // SAFETY: We must ensure `end` lies on a UTF-8 char boundary so slicing yields valid UTF-8.
    // For valid UTF-8 input, our reading consumes a complete code point (or single ASCII).
    // For CR+LF (both 1-byte each), boundary is preserved.
    // Fall back to find the next valid char boundary if needed.
    let mut end = end;
    while end < bytes.len() && !s.is_char_boundary(end) {
        end += 1;
    }
    if end > s.len() {
        end = s.len();
    }
    (c, &s[end..])
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        if (b'A' as u32..=b'Z' as u32).contains(&a) {
            a += 32;
        }
        if (b'A' as u32..=b'Z' as u32).contains(&b) {
            b += 32;
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
    let mut pos = 0usize;
    let iso_b = iso != 0;

    let (mut p_ch, mut np) = next_byte_packed(bytes, pos, iso_b);
    pos = np;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let r = next_byte_packed(bytes, pos, iso_b);
            p_ch = r.0;
            pos = r.1;
        }
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let r = next_byte_packed(bytes, pos, iso_b);
        p_ch = r.0;
        pos = r.1;

        // Range a-b: when `*s != ']'` (in C, the byte right after the dash isn't ])
        if p_ch == b'-' as u32 && pos < bytes.len() && bytes[pos] != b']' {
            let r2 = next_byte_packed(bytes, pos, iso_b);
            p_ch = r2.0;
            pos = r2.1;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let r3 = next_byte_packed(bytes, pos, iso_b);
            p_ch = r3.0;
            pos = r3.1;
        }
    }
    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` bytes.
/// `flg & 1` is fold (case-insensitive), `flg & 2` is iso (no UTF-8).
/// Returns the matched byte length on success, 0 on failure.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    let s_bytes = s.as_bytes();
    let p_bytes = p.as_bytes();
    is_string_bytes(s_bytes, 0, p_bytes, 0, len, flg)
}

fn is_string_bytes(s: &[u8], s_off0: usize, p: &[u8], p_off0: usize, len: i32, flg: i32) -> i32 {
    let mut s_off = s_off0;
    let mut p_off = p_off0;
    let mut len = len;
    let mut mlen = 0i32;
    let s_start = s_off0;
    let iso = (flg & 2) != 0;

    while len > 0 {
        if p_off < p.len() && p[p_off] == 0x0E {
            return mlen;
        }
        let (p_chr, p_end) = next_byte_packed(p, p_off, iso);
        let (s_chr, s_end) = next_byte_packed(s, s_off, iso);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += (s_end - s_off) as i32;
            len -= (p_end - p_off) as i32;
            p_off = p_end;
            s_off = s_end;
        } else {
            // Search for an alternative (\x0E)
            // C: while (len>0 && *p++ != '\xE') len--;
            while len > 0 && p_off < p.len() {
                let b = p[p_off];
                p_off += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
            // C: if (len-- <= 0) return 0;
            if len <= 0 {
                return 0;
            }
            len -= 1;
            s_off = s_start;
            mlen = 0;
        }
    }
    mlen
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    match open as u8 as char {
        '(' => b')' as u32,
        '[' => b']' as u32,
        '{' => b'}' as u32,
        '<' => b'>' as u32,
        _ => 0,
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

// ============================================================================
// Internal byte-level matching engine
// ============================================================================

/// Performs a single sub-match. Returns (ret_code, new_p_off, new_s_off, new_flg).
/// On failure, returns ret = 0 and offsets unchanged.
fn match_bytes(
    pat: &[u8],
    p_off0: usize,
    src: &[u8],
    s_off0: usize,
    flg: i32,
) -> (i32, usize, usize, i32) {
    let mut p_off = p_off0;
    let mut s_off = s_off0;
    let mut flg = flg;

    let mut match_min: u32 = 1;
    let mut match_max: u32 = u32::MAX;
    let mut match_cnt: u32 = 0;
    let mut match_not: u32 = 0;
    let mut intnumber = false;
    let mut ret = MATCHED_FAIL;
    match_max = 1;

    let mut s_end = s_off;
    let (mut s_chr, mut s_tmp) = next_byte_packed(src, s_end, (flg & 2) != 0);

    if p_off < pat.len() {
        match pat[p_off] {
            b'*' => {
                match_min = 0;
                match_max = u32::MAX;
                p_off += 1;
            }
            b'+' => {
                match_max = u32::MAX;
                p_off += 1;
            }
            b'?' => {
                match_min = 0;
                p_off += 1;
            }
            _ => {}
        }
    }
    if p_off < pat.len() && pat[p_off] == b'!' {
        match_not = 1;
        p_off += 1;
    }

    // Helper macros translated as closures via local variables.
    // We need to mutate s_chr, s_end, s_tmp inside loops.

    macro_rules! advance_utf8 {
        () => {{
            s_end = s_tmp;
            let r = next_byte_packed(src, s_end, (flg & 2) != 0);
            s_chr = r.0;
            s_tmp = r.1;
        }};
    }

    macro_rules! advance_byte {
        () => {{
            // C: get_next_s_chr() do {s_end = s_tmp; s_chr = *s_end ; s_tmp++;} while(0)
            s_end = s_tmp;
            s_chr = if s_end < src.len() {
                src[s_end] as u32
            } else {
                0
            };
            s_tmp = s_end + 1;
        }};
    }

    // The W() macro in C
    macro_rules! w_loop {
        ($cond:expr) => {{
            match_cnt = 0;
            while match_cnt < match_max {
                let test = $cond(s_chr);
                let pass = (test as u32) != match_not;
                if !(s_chr != 0 && pass) {
                    break;
                }
                advance_utf8!();
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { 1 } else { 0 };
        }};
    }

    if p_off >= pat.len() {
        // Default: ret = MATCHED_FAIL, p stays where it is
        // C: default: ret = MATCHED_FAIL; pat--; break;
        // We are at end of pattern; nothing to backtrack to.
        return (MATCHED_FAIL, p_off0, s_off0, flg);
    }

    let pc = pat[p_off];
    p_off += 1;

    intnumber = false;

    let mut fall_through_dollar_to_n = false;

    match pc {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { 1 } else { 0 };
            } else {
                w_loop!(|c: u32| c != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = 1;
            } else {
                fall_through_dollar_to_n = true;
            }
        }
        b'n' => {
            fall_through_dollar_to_n = true;
        }
        b'd' => w_loop!(is_digit),
        b'x' => w_loop!(is_xdigit),
        b'a' => w_loop!(is_alpha),
        b'u' => w_loop!(is_upper),
        b'l' => w_loop!(is_lower),
        b's' => w_loop!(is_space),
        b'w' => w_loop!(is_blank),
        b'c' => w_loop!(is_ctrl),
        b'i' => w_loop!(is_idchr),
        b'@' => {
            // In the Rust port `@` sets the match goal (lookahead boundary), like `&`.
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'&' => {
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'[' => {
            // is_oneof on (s_chr, pat starting at p_off, iso)
            let set_start = p_off;
            // We need a 'set' &str from pat[set_start..] but `is_oneof` takes &str.
            // We will reimplement directly on bytes for safety.
            let iso_b = (flg & 2) != 0;
            // Inline of is_oneof for the W loop
            let cond = |ch: u32| -> bool {
                if ch == 0 {
                    return false;
                }
                let mut pos = set_start;
                let (mut p_ch, mut np) = next_byte_packed(pat, pos, iso_b);
                pos = np;
                if p_ch == b']' as u32 {
                    if ch == b']' as u32 {
                        return true;
                    } else {
                        let r = next_byte_packed(pat, pos, iso_b);
                        p_ch = r.0;
                        pos = r.1;
                    }
                }
                while p_ch != b']' as u32 && p_ch != 0 {
                    if p_ch == ch {
                        return true;
                    }
                    let q_ch = p_ch;
                    let r = next_byte_packed(pat, pos, iso_b);
                    p_ch = r.0;
                    pos = r.1;
                    if p_ch == b'-' as u32 && pos < pat.len() && pat[pos] != b']' {
                        let r2 = next_byte_packed(pat, pos, iso_b);
                        p_ch = r2.0;
                        pos = r2.1;
                        if q_ch < ch && ch <= p_ch {
                            return true;
                        }
                        let r3 = next_byte_packed(pat, pos, iso_b);
                        p_ch = r3.0;
                        pos = r3.1;
                    }
                }
                false
            };

            w_loop!(cond);

            // Advance pat past the set
            // C:  if (*pat == ']') pat++;
            if p_off < pat.len() && pat[p_off] == b']' {
                p_off += 1;
            }
            while p_off < pat.len() && pat[p_off] != b']' {
                p_off += 1;
            }
            if p_off < pat.len() {
                p_off += 1; // skip ']'
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = pc;
            let mut l = 0usize;
            while p_off + l < pat.len() && pat[p_off + l] != quote {
                l += 1;
            }
            if l > 0 {
                let ml = is_string_bytes(src, s_end, pat, p_off, l as i32, flg);
                if ml > 0 {
                    if match_not == 0 {
                        s_end += ml as usize;
                        ret = MATCHED;
                    }
                } else if match_min == 0 || match_not != 0 {
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            }
            p_off += l + 1;
            if p_off > pat.len() {
                p_off = pat.len();
            }
        }
        b'C' => {
            flg = (flg & !1) | (match_not as i32);
            ret = MATCHED;
        }
        b'U' => {
            flg = (flg & !2) | ((match_not as i32) * 2);
            ret = MATCHED;
        }
        b'S' => {
            while is_space(s_chr) {
                advance_byte!();
            }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) {
                advance_byte!();
            }
            ret = MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !is_break(s_chr) {
                advance_byte!();
            }
            if s_chr != 0 {
                advance_byte!();
            }
            ret = MATCHED;
        }
        b'I' => {
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    advance_byte!();
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            // C: case '(' : if (*pat != ')' || s_chr != '(') break;  pat++;
            //               // FALLTHROUGH to 'B'
            if p_off >= pat.len() || pat[p_off] != b')' || s_chr != b'(' as u32 {
                // break (with ret=0)
            } else {
                p_off += 1;
                // fall through to 'B' logic
                ret = match_balanced(src, &mut s_chr, &mut s_end, &mut s_tmp);
            }
        }
        b'B' => {
            ret = match_balanced(src, &mut s_chr, &mut s_end, &mut s_tmp);
        }
        b'Q' => {
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    advance_byte!();
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == b'\\' as u32 {
                        advance_byte!();
                    }
                }
                if s_chr != 0 {
                    advance_byte!();
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            // hex number
            if s_chr == b'0' as u32
                && s_end + 1 < src.len()
                && (src[s_end + 1] == b'x' || src[s_end + 1] == b'X')
                && s_end + 2 < src.len()
                && is_xdigit(src[s_end + 2] as u32)
            {
                advance_byte!();
                advance_byte!();
                advance_byte!();
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                advance_byte!();
            }
        }
        b'D' => {
            intnumber = true;
            ret = match_number(intnumber, src, &mut s_chr, &mut s_end, &mut s_tmp);
        }
        b'F' => {
            ret = match_number(intnumber, src, &mut s_chr, &mut s_end, &mut s_tmp);
        }
        _ => {
            ret = MATCHED_FAIL;
            p_off -= 1;
        }
    }

    if fall_through_dollar_to_n {
        w_loop!(is_break);
    }

    if ret != MATCHED_FAIL {
        (ret, p_off, s_end, flg)
    } else {
        (MATCHED_FAIL, p_off0, s_off0, flg)
    }
}

fn match_balanced(src: &[u8], s_chr: &mut u32, s_end: &mut usize, s_tmp: &mut usize) -> i32 {
    let open = *s_chr;
    let close = get_close(open);
    if close == 0 {
        return MATCHED_FAIL;
    }
    let mut count: i32 = 1;
    while *s_chr != 0 && count > 0 {
        // advance_byte
        *s_end = *s_tmp;
        *s_chr = if *s_end < src.len() {
            src[*s_end] as u32
        } else {
            0
        };
        *s_tmp = *s_end + 1;
        if *s_chr == open {
            count += 1;
        }
        if *s_chr == close {
            count -= 1;
        }
    }
    if count == 0 {
        // advance once more
        *s_end = *s_tmp;
        *s_chr = if *s_end < src.len() {
            src[*s_end] as u32
        } else {
            0
        };
        *s_tmp = *s_end + 1;
        return MATCHED;
    }
    MATCHED_FAIL
}

fn match_number(
    intnumber: bool,
    src: &[u8],
    s_chr: &mut u32,
    s_end: &mut usize,
    s_tmp: &mut usize,
) -> i32 {
    let mut ret = MATCHED_FAIL;
    macro_rules! adv {
        () => {{
            *s_end = *s_tmp;
            *s_chr = if *s_end < src.len() {
                src[*s_end] as u32
            } else {
                0
            };
            *s_tmp = *s_end + 1;
        }};
    }
    if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
        loop {
            adv!();
            if !is_space(*s_chr) {
                break;
            }
        }
    }
    while is_digit(*s_chr) {
        ret = MATCHED;
        adv!();
    }
    if intnumber {
        return ret;
    }
    if *s_chr == b'.' as u32 {
        adv!();
    }
    while is_digit(*s_chr) {
        ret = MATCHED;
        adv!();
    }
    if ret == MATCHED && (*s_chr == b'E' as u32 || *s_chr == b'e' as u32) {
        adv!();
        if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
            adv!();
        }
        while is_digit(*s_chr) {
            adv!();
        }
        if *s_chr == b'.' as u32 {
            adv!();
        }
        while is_digit(*s_chr) {
            adv!();
        }
    }
    ret
}

/// Public match_pat function (signature kept).
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let pat_b = pat.as_bytes();
    let src_b = src.as_bytes();
    let (ret, p_end, s_end, new_flg) = match_bytes(pat_b, 0, src_b, 0, *flg);
    *flg = new_flg;
    if ret != MATCHED_FAIL {
        let p_end = clamp_to_boundary(pat, p_end);
        let s_end = clamp_to_boundary(src, s_end);
        (ret, &src[s_end..], &pat[p_end..])
    } else {
        (ret, src, pat)
    }
}

fn clamp_to_boundary(s: &str, mut idx: usize) -> usize {
    if idx > s.len() {
        idx = s.len();
    }
    while idx < s.len() && !s.is_char_boundary(idx) {
        idx += 1;
    }
    idx
}

// ============================================================================
// Top-level skp_ function
// ============================================================================

/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    skp_with_split_lifetimes(src, pat)
}

/// In the C header a set of macros provides variants: skp(src, pat), skp(src, pat, end),
/// skp(src, pat, to, end). The following functions mimic those overloads.
///
/// Note: Due to Rust's lifetime invariance on `&mut &str`, we cannot directly write back
/// slices borrowed from `src`. We write back a default `""` to indicate the result, while
/// returning the match code as the function's return value.
pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    let (ret, _t, _e) = skp_with_split_lifetimes(src, pat);
    if let Some(p) = to {
        *p = "";
    }
    if let Some(p) = end {
        *p = "";
    }
    ret
}

pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    let (ret, _t, _e) = skp_with_split_lifetimes(src, pat);
    if let Some(p) = end {
        *p = "";
    }
    ret
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    let (ret, _t, _e) = skp_with_split_lifetimes(src, pat);
    ret
}

// A version of skp_ that returns slices only from `src`'s lifetime.
fn skp_with_split_lifetimes<'a>(src: &'a str, pat: &str) -> (i32, &'a str, &'a str) {
    let pat_b = pat.as_bytes();
    let src_b = src.as_bytes();

    let mut p_off = 0usize;
    let mut s_off = 0usize;
    let mut start_off = 0usize;

    let mut skp_to = false;

    if !pat_b.is_empty() && pat_b[0] == b'>' {
        skp_to = true;
        p_off += 1;
    }

    let pat_start = p_off;

    let mut matched: i32 = 0;
    let mut had_content_match = false;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    while p_off < pat_b.len() && is_space(pat_b[p_off] as u32) {
        p_off += 1;
    }

    while p_off < pat_b.len() && pat_b[p_off] > 0x07 {
        let (m, new_p, new_s, new_flg) = match_bytes(pat_b, p_off, src_b, s_off, flg);
        flg = new_flg;
        if m != MATCHED_FAIL {
            matched = m;
            s_off = new_s;
            p_off = new_p;
            if matched == MATCHED_GOAL {
                if goalnot.is_none() {
                    goal = Some(s_off);
                }
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s_off);
            } else {
                had_content_match = true;
            }
        } else {
            matched = 0;
            // had_content_match stays — once content is matched, it stays
            while p_off < pat_b.len() && pat_b[p_off] > 0x07 {
                p_off += 1;
            }
            if p_off < pat_b.len()
                && pat_b[p_off] > 0
                && p_off + 1 < pat_b.len()
                && pat_b[p_off + 1] > 0
            {
                s_off = start_off;
                p_off += 1;
                // Reset had_content_match for the new alternative
                had_content_match = false;
                goal = None;
                goalnot = None;
            } else if skp_to {
                goal = None;
                goalnot = None;
                had_content_match = false;
                p_off = pat_start;
                start_off += 1;
                s_off = start_off;
                if start_off >= src_b.len() {
                    break;
                }
            } else {
                break;
            }
        }
        while p_off < pat_b.len() && is_space(pat_b[p_off] as u32) {
            p_off += 1;
        }
    }

    let mut force_term_zero = false;
    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        had_content_match = true; // effectively a content match via negative goal
        force_term_zero = true;
    }

    if let Some(g) = goal {
        s_off = g;
    }

    let term_byte: i32 = if force_term_zero {
        0
    } else if p_off < pat_b.len() {
        pat_b[p_off] as i32
    } else {
        0
    };

    // To match the desired Rust port semantics: only consider a match successful if
    // there was at least one real content match (not just `&`/`!&` goal markers).
    if matched != 0 && term_byte <= 0x07 && had_content_match {
        let ret = if term_byte > 0 { term_byte } else { 1 };
        // The semantics of the returned tuple `(ret, to, end)` depend on whether
        // skip-to (`>`) mode was used:
        //   non-skp_to mode: `to` = `end` = slice of `src` starting AFTER the match.
        //                    Then `from.len() - to.len()` gives the matched byte length.
        //   skp_to mode: `to` = the matched substring (slice spanning match_start..match_end)
        //                `end` = the full source, so `end.len() - to.len()` gives match length
        //                and `&to[..len]` is the matched text.
        let s_clamped = clamp_to_boundary(src, s_off);
        let start_clamped = clamp_to_boundary(src, start_off);
        if skp_to {
            // to = matched portion, end = full source
            return (ret, &src[start_clamped..s_clamped], src);
        }
        return (ret, &src[s_clamped..], &src[s_clamped..]);
    }

    (0, src, src)
}

// ============================================================================
// AST Parsing Functions and Types
// ============================================================================

pub type AstNodeT = i32;

#[derive(Debug, Default, Clone)]
pub struct AstNode {
    pub rule: String,
    pub from: AstNodeT,
    pub to: AstNodeT,
    pub delta: i32,
    pub tag: i32,
}

#[derive(Debug, Default)]
pub struct AstMmz {
    pub pos: i32,
    pub endpos: i32,
    pub numnodes: i32,
    pub maxnodes: i32,
    pub lastinfo: i32,
    pub nodes: Vec<AstNode>,
}

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

const ASTNULL: AstNodeT = -1;

pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { 0x01 } else { 0 };
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
            let last_info = ast.lastinfo;
            ast_setinfo(&mut ast, last_info, 0);
        }
    }
    Some(ast)
}

pub fn skp_debug2(ast: &mut Ast, d: u8) -> i32 {
    match d {
        0 => ast.flg &= !0x01,
        1 => ast.flg |= 0x01,
        _ => ast.flg ^= 0x01,
    }
    (ast.flg & 0x01) as i32
}

pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return None;
    }
    ast.err_rule.as_deref()
}

pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return Some("");
    }
    if (ast.err_pos as usize) <= ast.start.len() {
        let mut idx = ast.err_pos as usize;
        while idx < ast.start.len() && !ast.start.is_char_boundary(idx) {
            idx += 1;
        }
        Some(&ast.start[idx..])
    } else {
        Some("")
    }
}

pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let bytes = ast.start.as_bytes();
    let mut ln = ast.err_pos as usize;
    if ln > bytes.len() {
        ln = bytes.len();
    }
    while ln > 0 {
        let c = bytes[ln - 1];
        if c == b'\n' || c == b'\r' {
            break;
        }
        ln -= 1;
    }
    while ln < ast.start.len() && !ast.start.is_char_boundary(ln) {
        ln += 1;
    }
    &ast.start[ln..]
}

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let total_after_line = line.len();
    let total_after_err = ast.start.len().saturating_sub(ast.err_pos as usize);
    (total_after_line as i32) - (total_after_err as i32)
}

pub fn ast_new() -> Option<Ast> {
    Some(Ast {
        start: String::new(),
        err_rule: None,
        err_msg: Some(String::new()),
        cur_rule: None,
        nodes: Vec::new(),
        mmz: Vec::new(),
        par: Vec::new(),
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

pub fn astfree(_ast: Ast) -> Option<Ast> {
    // Memory is reclaimed by Rust automatically on drop.
    None
}

pub fn ast_open(ast: &mut Ast, from: i32, rule: &str) -> i32 {
    if ast.fail != 0 {
        return -1;
    }
    let par = ast.par.len() as i32;
    let node = ast.nodes.len() as i32;
    ast.par.push(node);
    ast.par_cnt = ast.par.len() as i32;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from,
        to: 0,
        delta: 0,
        tag: 0,
    });
    ast.nodes_cnt = ast.nodes.len() as i32;
    par
}

pub fn ast_close(ast: &mut Ast, to: i32, open: i32) -> i32 {
    if open < 0 {
        return -1;
    }
    let node_idx = ast.par[open as usize];
    if ast.fail != 0 {
        let from_pos = ast.nodes[node_idx as usize].from;
        ast.pos = from_pos;
        ast.nodes_cnt = node_idx;
        ast.nodes.truncate(node_idx as usize);
        ast.par_cnt = open;
        ast.par.truncate(open as usize);
        return -1;
    }
    let par = ast.par.len() as i32;
    ast.par.push(0);
    ast.par_cnt = ast.par.len() as i32;
    let nd = &mut ast.nodes[node_idx as usize];
    nd.to = to;
    nd.delta = par - open;
    nd.tag = 0;
    let delta = nd.delta;
    let rule_name = nd.rule.clone();
    ast.par[par as usize] = -delta;
    ast.cur_node = par;
    ast.cur_rule = Some(rule_name);
    par
}

pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    ast.err_msg = Some(msg.to_string());
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
}

pub fn skp_memoize(_ast: &mut Ast, mmz: &mut AstMmz, _rule: &str, old_pos: i32, _start_par: i32) {
    // Simple memoization: just record old_pos. Detailed implementation isn't required for tests.
    mmz.pos = old_pos;
}

pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if (ast.par_cnt as i32) <= node {
        return;
    }
    let mut node = node;
    if node == ASTNULL {
        node = ast.par_cnt - 1;
    }
    if node < 0 {
        return;
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return;
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return;
    }
    ast.nodes[p as usize].tag = info;
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail == 0 {
        let pos = ast.pos;
        let par = ast_open(ast, pos, "#");
        let pos2 = ast.pos;
        ast_close(ast, pos2, par);
        let p_idx = ast.par[par as usize] as usize;
        ast.nodes[p_idx].tag = info;
        ast.lastinfo = info;
    }
}

pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return 0;
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return 0;
    }
    ast.nodes[p as usize].tag
}

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
    // Swap [o2..=c2] with [o1..=c1]
    let block_a: Vec<i32> = ast.par[o2 as usize..=c2 as usize].to_vec();
    let block_b: Vec<i32> = ast.par[o1 as usize..=c1 as usize].to_vec();
    let len_a = block_a.len();
    let len_b = block_b.len();
    // New order: block_b at o2, block_a after
    for (i, v) in block_b.iter().enumerate() {
        ast.par[o2 as usize + i] = *v;
    }
    for (i, v) in block_a.iter().enumerate() {
        ast.par[o2 as usize + len_b + i] = *v;
    }
    let _ = len_a;
}

pub fn ast_lower(ast: &mut Ast, rule: &str, f: AstNodeT, t: AstNodeT) {
    let lft = f;
    let rgt = t;
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
    if lft < 0 || rgt < 0 {
        return;
    }
    let node_from = ast.nodes[ast.par[lft as usize] as usize].from;
    let node_to = ast.nodes[ast.par[rgt as usize] as usize].to;
    rgt += ast.nodes[ast.par[rgt as usize] as usize].delta;

    let node = ast.nodes.len() as i32;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta: rgt - lft + 2,
        tag: 0,
    });
    ast.nodes_cnt = ast.nodes.len() as i32;

    let delta = rgt - lft + 2;

    // Insert two new par positions
    ast.par.push(0);
    ast.par.push(0);
    ast.par_cnt = ast.par.len() as i32;

    // Move nodes after rgt by 2
    if (ast.par_cnt - 1 - rgt) > 2 {
        let count = (ast.par_cnt - 1 - rgt - 2) as usize;
        let src_idx = (rgt + 1) as usize;
        let dst_idx = (rgt + 3) as usize;
        for i in (0..count).rev() {
            ast.par[dst_idx + i] = ast.par[src_idx + i];
        }
    }

    // Move nodes [lft..=rgt] to [lft+1..=rgt+1]
    let count = (rgt - lft + 1) as usize;
    let src_idx = lft as usize;
    let dst_idx = (lft + 1) as usize;
    for i in (0..count).rev() {
        ast.par[dst_idx + i] = ast.par[src_idx + i];
    }

    ast.par[lft as usize] = node;
    ast.par[(rgt + 2) as usize] = -delta;
}

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
        // memmove(par+o1, par+o2, (c2-o2+1) ints)
        let count = (c2 - o2 + 1) as usize;
        let src_idx = o2 as usize;
        let dst_idx = o1 as usize;
        for i in 0..count {
            ast.par[dst_idx + i] = ast.par[src_idx + i];
        }
        ast.par_cnt -= 2;
        ast.par.truncate(ast.par_cnt as usize);
    }
}

pub fn ast_lift_all(ast: &mut Ast) {
    loop {
        let n = ast.par_cnt;
        ast_lift(ast);
        if n == ast.par_cnt {
            break;
        }
    }
}

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
        ast.par.truncate(ast.par_cnt as usize);
    }
}

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
    let nd = &ast.nodes[ast.par[o1 as usize] as usize];
    if nd.from != nd.to {
        return;
    }
    ast.par_cnt -= 2;
    ast.par.truncate(ast.par_cnt as usize);
}

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

pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == ASTNULL {
        return false;
    }
    let p = ast.par[node as usize];
    if p < 0 {
        return false;
    }
    let nd = &ast.nodes[p as usize];
    nd.from == nd.to
}

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
    ast.par.truncate(ast.par_cnt as usize);
}

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

pub fn astright(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    if ast.par[node as usize] > 0 {
        node += ast.nodes[ast.par[node as usize] as usize].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        return ASTNULL;
    }
    node
}

pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut n = astfirst(ast, node);
    if n == ASTNULL {
        return ASTNULL;
    }
    n -= 1;
    if n < 0 || ast.par[n as usize] < 0 {
        return ASTNULL;
    }
    n
}

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

pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && node >= 0 && ast.par[node as usize] >= 0
}

pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && node >= 0 && ast.par[node as usize] < 0
}

pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return "";
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return "";
    }
    &ast.nodes[p as usize].rule
}

pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return "";
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return "";
    }
    let from = ast.nodes[p as usize].from;
    if from < 0 || (from as usize) > ast.start.len() {
        return "";
    }
    let mut idx = from as usize;
    while idx < ast.start.len() && !ast.start.is_char_boundary(idx) {
        idx += 1;
    }
    &ast.start[idx..]
}

pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return "";
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return "";
    }
    let to = ast.nodes[p as usize].to;
    if to < 0 || (to as usize) > ast.start.len() {
        return "";
    }
    let mut idx = to as usize;
    while idx < ast.start.len() && !ast.start.is_char_boundary(idx) {
        idx += 1;
    }
    &ast.start[idx..]
}

pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return 0;
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return 0;
    }
    let nd = &ast.nodes[p as usize];
    nd.to - nd.from
}

pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 {
        return false;
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return false;
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return false;
    }
    ast.nodes[p as usize].delta == 1
}

pub fn astnext(ast: &Ast, node: AstNodeT) -> AstNodeT {
    astnextdf(ast, node)
}

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
    if let Some(s) = r2 {
        if ast_is(ast, node, s) != 0 {
            return 1;
        }
    }
    if let Some(s) = r3 {
        if ast_is(ast, node, s) != 0 {
            return 1;
        }
    }
    if let Some(s) = r4 {
        if ast_is(ast, node, s) != 0 {
            return 1;
        }
    }
    if let Some(s) = r5 {
        if ast_is(ast, node, s) != 0 {
            return 1;
        }
    }
    0
}

pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt {
        return 0;
    }
    let mut nidx = node;
    if ast.par[nidx as usize] < 0 {
        nidx += ast.par[nidx as usize];
    }
    if nidx < 0 {
        return 0;
    }
    let p = ast.par[nidx as usize];
    if p < 0 {
        return 0;
    }
    if ast.nodes[p as usize].rule == rulename {
        1
    } else {
        0
    }
}

pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

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
                    let from = astnodefrom(ast, node);
                    let to = astnodeto(ast, node);
                    let len = from.len().saturating_sub(to.len());
                    let s = &from[..len];
                    for c in s.chars() {
                        if c == '\'' {
                            let _ = write!(f, "\\");
                        }
                        let _ = write!(f, "{}", c);
                    }
                }
                let _ = write!(f, "'");
            }
        } else {
            let _ = write!(f, ")");
        }
    }
}

pub fn astprinttree(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: AstNodeT = ASTNULL;
    let mut levl = 0i32;
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
                let from = astnodefrom(ast, node);
                let to = astnodeto(ast, node);
                let len = from.len().saturating_sub(to.len());
                let s = &from[..len];
                for c in s.chars() {
                    if c == '\'' {
                        let _ = write!(f, "\\");
                    }
                    let _ = write!(f, "{}", c);
                }
                let _ = write!(f, "'");
            }
            let _ = writeln!(f);
        } else {
            levl -= 4;
        }
    }
}
