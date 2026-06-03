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
    // In C: int ret = to-start; return (0 <= ret && ret <= (1<<16)?ret:0);
    // We model `start` and `to` as suffixes of the same backing string.
    let ret: i32 = (start.len() as i32) - (to.len() as i32);
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

// ----------------------------------------------------------------------------
// Internal helpers for byte-level processing.
// The C code treats text as raw bytes and (for UTF-8) packs the bytes of a
// multi-byte sequence into a single u32 (so e.g. 'è' → 0xC3A8).
// We therefore work on the byte representation of the input strings and
// convert back to &str by slicing on byte indices.
// ----------------------------------------------------------------------------

fn skp_next_bytes(s: &[u8], iso: i32) -> (u32, usize) {
    // Returns (codepoint_packed_as_in_C, number_of_bytes_consumed)
    if s.is_empty() || s[0] == 0 {
        return (0, 0);
    }
    let mut c: u32 = s[0] as u32;
    let mut i: usize = 1;

    if (iso & 2) == 0 {
        // UTF-8: append continuation bytes (up to three more)
        if i < s.len() && (s[i] & 0xC0) == 0x80 {
            c = (c << 8) | (s[i] as u32);
            i += 1;
            if i < s.len() && (s[i] & 0xC0) == 0x80 {
                c = (c << 8) | (s[i] as u32);
                i += 1;
                if i < s.len() && (s[i] & 0xC0) == 0x80 {
                    c = (c << 8) | (s[i] as u32);
                    i += 1;
                }
            }
        }
    }
    // CRLF combination
    if c == 0x0D && i < s.len() && s[i] == 0x0A {
        c = 0x0D0A;
        i += 1;
    }
    (c, i)
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let (c, n) = skp_next_bytes(s.as_bytes(), iso);
    (c, &s[n..])
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
/// (Corresponds to `chr_cmp`.)
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if (fold & 1) != 0 && a <= 0x7F && b <= 0x7F {
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
        0x00E38080 => c == 0xE38080,
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
    c < 0x20
        || (0xC280..0xC2A0).contains(&c)
        || (0x7F..0xA0).contains(&c)
}

// Internal byte version of is_oneof so the implementation can advance through
// the pattern without reconstructing &str repeatedly.
fn is_oneof_bytes(ch: u32, set: &[u8], iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let (mut p_ch, mut consumed) = skp_next_bytes(set, iso);
    let mut s = &set[consumed..];

    // Special case: if the set starts with ']', it's treated as a literal
    // unless ch itself is ']'.
    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        let r = skp_next_bytes(s, iso);
        p_ch = r.0;
        consumed = r.1;
        s = &s[consumed..];
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let r = skp_next_bytes(s, iso);
        p_ch = r.0;
        consumed = r.1;
        s = &s[consumed..];
        if p_ch == b'-' as u32 && !s.is_empty() && s[0] != b']' {
            let r = skp_next_bytes(s, iso);
            p_ch = r.0;
            consumed = r.1;
            s = &s[consumed..];
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let r = skp_next_bytes(s, iso);
            p_ch = r.0;
            consumed = r.1;
            s = &s[consumed..];
        }
    }
    false
}

/// Returns true if `ch` is one of the characters in `set`. The `iso` flag is used for encoding.
pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    is_oneof_bytes(ch, set.as_bytes(), iso)
}

// Internal byte-based is_string returning the matched length on the source side
fn is_string_bytes(s: &[u8], p: &[u8], len: i32, flg: i32) -> i32 {
    let mut len = len;
    let start = s;
    let mut s = s;
    let mut p = p;
    let mut mlen: i32 = 0;
    while len > 0 {
        if !p.is_empty() && p[0] == 0x0E {
            return mlen;
        }

        let (p_chr, p_n) = skp_next_bytes(p, flg & 2);
        let (s_chr, s_n) = skp_next_bytes(s, flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += s_n as i32;
            len -= p_n as i32;
            p = &p[p_n..];
            s = &s[s_n..];
        } else {
            // search for an alternative '\xE'
            // Mirror C: while (len>0 && *p++ != '\xE') len--;
            // -> advance p, only decrement len when char != \xE.
            loop {
                if len <= 0 {
                    break;
                }
                let ch = if !p.is_empty() { p[0] } else { 0 };
                p = if !p.is_empty() { &p[1..] } else { p };
                if ch == 0x0E {
                    break;
                }
                len -= 1;
            }
            // Mirror C: if (len-- <= 0) return 0;
            if len <= 0 {
                return 0;
            }
            len -= 1;
            s = start;
            mlen = 0;
        }
    }
    mlen
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_bytes(s.as_bytes(), p.as_bytes(), len, flg)
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

// Internal byte-level match function. Returns
// (ret, p_consumed_bytes, s_consumed_bytes).
// `p_consumed_bytes` is the length of the prefix of `pat` that was consumed.
// `s_consumed_bytes` is the length of the prefix of `src` that was consumed.
fn match_bytes(pat: &[u8], src: &[u8], flg: &mut i32) -> (i32, usize, usize) {
    // Initial s_chr lookahead.
    let (mut s_chr, mut s_tmp_n) = skp_next_bytes(src, *flg & 2);
    let mut s_end_n: usize = 0; // bytes consumed from src so far

    let mut p_idx: usize = 0;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    if p_idx < pat.len() {
        match pat[p_idx] {
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
    if p_idx < pat.len() && pat[p_idx] == b'!' {
        match_not = 1;
        p_idx += 1;
    }

    let mut ret: i32 = MATCHED_FAIL;

    // Helper closure to handle the "W(x)" macro: counts how many times the
    // character class predicate succeeds.
    macro_rules! W {
        ($pred:expr) => {{
            let mut match_cnt: u32 = 0;
            while match_cnt < match_max && (s_chr != 0 && (($pred) != (match_not != 0))) {
                s_end_n += s_tmp_n;
                let (c, n) = skp_next_bytes(&src[s_end_n..], *flg & 2);
                s_chr = c;
                s_tmp_n = n;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    // get_next_s_chr: byte-by-byte advance (used for some patterns)
    macro_rules! get_next_s_chr {
        () => {{
            // s_end <- s_tmp;  s_chr = *s_end ; s_tmp++
            // s_tmp_n captures how far s_tmp is past s_end.
            s_end_n += s_tmp_n;
            s_chr = if s_end_n < src.len() { src[s_end_n] as u32 } else { 0 };
            s_tmp_n = 1;
        }};
    }

    if p_idx >= pat.len() {
        return (MATCHED_FAIL, p_idx, 0);
    }

    let cur = pat[p_idx];
    p_idx += 1;

    let mut handle_n = false;
    match cur {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                W!(s_chr != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                handle_n = true;
            }
        }
        b'n' => {
            handle_n = true;
        }
        b'd' => W!(is_digit(s_chr)),
        b'x' => W!(is_xdigit(s_chr)),
        b'a' => W!(is_alpha(s_chr)),
        b'u' => W!(is_upper(s_chr)),
        b'l' => W!(is_lower(s_chr)),
        b's' => W!(is_space(s_chr)),
        b'w' => W!(is_blank(s_chr)),
        b'c' => W!(is_ctrl(s_chr)),
        b'i' => W!(is_idchr(s_chr)),
        b'&' | b'@' => {
            // Both '&' and '@' set a goal marker. '!&' / '!@' set a negative goal.
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'[' => {
            // The set pattern starts at p_idx
            W!(is_oneof_bytes(s_chr, &pat[p_idx..], *flg & 2));
            // Advance past the set, including ']'
            if p_idx < pat.len() && pat[p_idx] == b']' {
                p_idx += 1;
            }
            while p_idx < pat.len() && pat[p_idx] != b']' {
                p_idx += 1;
            }
            if p_idx < pat.len() {
                p_idx += 1; // consume ']'
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = cur;
            let mut l: usize = 0;
            while p_idx + l < pat.len() && pat[p_idx + l] != quote {
                l += 1;
            }
            let mlen = if l > 0 {
                is_string_bytes(&src[s_end_n..], &pat[p_idx..p_idx + l], l as i32, *flg)
            } else {
                0
            };
            if l > 0 && mlen > 0 {
                if match_not == 0 {
                    s_end_n += mlen as usize;
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            }
            p_idx += l + 1;
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
                get_next_s_chr!();
            }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) {
                get_next_s_chr!();
            }
            ret = MATCHED;
        }
        b'N' => {
            // up to end of line
            while s_chr != 0 && !is_break(s_chr) {
                get_next_s_chr!();
            }
            if s_chr != 0 {
                get_next_s_chr!();
            }
            ret = MATCHED;
        }
        b'I' => {
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next_s_chr!();
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            if p_idx >= pat.len() || pat[p_idx] != b')' || s_chr != b'(' as u32 {
                // not matching
            } else {
                p_idx += 1;
                handle_balanced(&src, &mut s_end_n, &mut s_chr, &mut s_tmp_n, &mut ret);
            }
        }
        b'B' => {
            handle_balanced(&src, &mut s_end_n, &mut s_chr, &mut s_tmp_n, &mut ret);
        }
        b'Q' => {
            // Quoted string
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next_s_chr!();
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == b'\\' as u32 {
                        get_next_s_chr!();
                    }
                }
                if s_chr != 0 {
                    get_next_s_chr!();
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            if s_chr == b'0' as u32
                && s_end_n + 1 < src.len()
                && (src[s_end_n + 1] == b'x' || src[s_end_n + 1] == b'X')
                && s_end_n + 2 < src.len()
                && is_xdigit(src[s_end_n + 2] as u32)
            {
                get_next_s_chr!();
                get_next_s_chr!();
                get_next_s_chr!();
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                get_next_s_chr!();
            }
        }
        b'D' | b'F' => {
            if cur == b'D' {
                intnumber = true;
            }
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next_s_chr!();
                    if !is_space(s_chr) {
                        break;
                    }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next_s_chr!();
            }
            if !intnumber {
                if s_chr == b'.' as u32 {
                    get_next_s_chr!();
                }
                while is_digit(s_chr) {
                    ret = MATCHED;
                    get_next_s_chr!();
                }
                if ret == MATCHED && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                    get_next_s_chr!();
                    if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                        get_next_s_chr!();
                    }
                    while is_digit(s_chr) {
                        get_next_s_chr!();
                    }
                    if s_chr == b'.' as u32 {
                        get_next_s_chr!();
                    }
                    while is_digit(s_chr) {
                        get_next_s_chr!();
                    }
                }
            }
        }
        _ => {
            ret = MATCHED_FAIL;
            p_idx -= 1;
        }
    }

    if handle_n {
        W!(is_break(s_chr));
    }

    if ret != MATCHED_FAIL {
        (ret, p_idx, s_end_n)
    } else {
        (MATCHED_FAIL, p_idx, 0)
    }
}

fn handle_balanced(
    src: &[u8],
    s_end_n: &mut usize,
    s_chr: &mut u32,
    s_tmp_n: &mut usize,
    ret: &mut i32,
) {
    let open = *s_chr;
    let close = get_close(open);
    if close != 0 {
        let mut count: i32 = 1;
        while *s_chr != 0 && count > 0 {
            // get_next_s_chr
            *s_end_n += *s_tmp_n;
            *s_chr = if *s_end_n < src.len() {
                src[*s_end_n] as u32
            } else {
                0
            };
            *s_tmp_n = 1;
            if *s_chr == open {
                count += 1;
            }
            if *s_chr == close {
                count -= 1;
            }
        }
        if count == 0 {
            *s_end_n += *s_tmp_n;
            *s_chr = if *s_end_n < src.len() {
                src[*s_end_n] as u32
            } else {
                0
            };
            *s_tmp_n = 1;
            *ret = MATCHED;
        }
    }
}

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`. The flag parameter is passed by mutable reference.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let (ret, p_n, s_n) = match_bytes(pat.as_bytes(), src.as_bytes(), flg);
    if ret != MATCHED_FAIL {
        (ret, &src[s_n..], &pat[p_n..])
    } else {
        (ret, src, pat)
    }
}

/// The core scanning function from the C header.
///
/// This corresponds to:
/// ```c
/// int skp_(char *src, char *pat, char **to, char **end);
/// ```
/// In Rust we take `&str` for both source and pattern and return a tuple:
/// `(match_code, to, end)`.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    if pat.is_empty() && src.is_empty() {
        return (0, src, src);
    }

    let src_bytes = src.as_bytes();
    let pat_bytes = pat.as_bytes();

    let mut start_n: usize = 0; // index into src_bytes for current "start"
    let mut s_n: usize = 0; // index into src_bytes for current scan position
    let mut p_idx: usize = 0;
    let mut skp_to = 0;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    if !pat_bytes.is_empty() && pat_bytes[0] == b'>' {
        skp_to = 1;
        p_idx += 1;
    }

    // Skip leading whitespace in pattern (ASCII space chars per is_space treats <0xFF first)
    while p_idx < pat_bytes.len() && is_space(pat_bytes[p_idx] as u32) {
        p_idx += 1;
    }

    while p_idx < pat_bytes.len() && pat_bytes[p_idx] > 7 {
        let (m, p_consumed, s_consumed) =
            match_bytes(&pat_bytes[p_idx..], &src_bytes[s_n..], &mut flg);
        if m != 0 {
            s_n += s_consumed;
            p_idx += p_consumed;
            if m == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s_n);
                // Goal does not change `matched` - keep prior real-match state.
            } else if m == MATCHED_GOALNOT {
                goalnot = Some(s_n);
                // Negative goal does not change `matched`.
            } else {
                matched = m;
            }
        } else {
            matched = 0;
            // Skip past the rest of the failed alternative (until next byte <= 7)
            while p_idx < pat_bytes.len() && pat_bytes[p_idx] > 7 {
                p_idx += 1;
            }
            if p_idx < pat_bytes.len()
                && pat_bytes[p_idx] > 0
                && p_idx + 1 < pat_bytes.len()
                && pat_bytes[p_idx + 1] > 0
            {
                // Try a new pattern alternative
                s_n = start_n;
                p_idx += 1;
            } else if skp_to != 0 {
                goal = None;
                goalnot = None;
                // Reset pattern to its start (after the leading '>' if any)
                p_idx = if pat_bytes.first() == Some(&b'>') { 1 } else { 0 };
                while p_idx < pat_bytes.len() && is_space(pat_bytes[p_idx] as u32) {
                    p_idx += 1;
                }
                start_n += 1;
                s_n = start_n;
                if start_n >= src_bytes.len() {
                    break;
                }
            } else {
                break;
            }
        }
        // skip whitespace
        while p_idx < pat_bytes.len() && is_space(pat_bytes[p_idx] as u32) {
            p_idx += 1;
        }
    }

    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        // p="" — make sure we treat the remaining pattern byte as terminator
        // by setting p_idx to pattern length (so the next check `*p <= '\7'` is true).
        p_idx = pat_bytes.len();
    }

    if let Some(g) = goal {
        s_n = g;
    }

    let pat_terminator_byte = if p_idx < pat_bytes.len() {
        pat_bytes[p_idx]
    } else {
        0
    };

    if matched != 0 && pat_terminator_byte <= 7 {
        let ret = if pat_terminator_byte > 0 {
            pat_terminator_byte as i32
        } else {
            1
        };
        if skp_to != 0 {
            // For the `>` (skip-to) operator, the returned `to` slice is the
            // matched substring, and the returned `end` slice is the full
            // source. This mirrors the C convention `len = end - to` while
            // keeping `&to[..len]` a valid slice.
            return (ret, &src[start_n..s_n], src);
        }
        return (ret, &src[s_n..], &src[s_n..]);
    }

    (0, src, src)
}

/// In the C header a set of macros provides variants:
///   skp(src, pat), skp(src, pat, end) and skp(src, pat, to, end).
///
/// The following functions mimic those overloads.
pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    // We need `to` and `end` to be slices borrowed from `src`. To do this, we
    // call the underlying skp_ which returns slices of `src` and then store them
    // through the provided mutable references.
    // SAFETY: the lifetimes of the references stored in `to`/`end` are bounded
    // by the lifetime of `src`, which the caller controls.
    let (alt, t, e) = skp_(src, pat);
    if let Some(slot) = to {
        // Transmute lifetime: the &str stored in `*slot` has the lifetime of
        // whatever the caller bound it to. Since `t` is a slice of `src`, this
        // is sound as long as the caller keeps `src` alive.
        let t_static: &str = unsafe { std::mem::transmute::<&str, &str>(t) };
        *slot = t_static;
    }
    if let Some(slot) = end {
        let e_static: &str = unsafe { std::mem::transmute::<&str, &str>(e) };
        *slot = e_static;
    }
    alt
}

pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    let (alt, _t, e) = skp_(src, pat);
    if let Some(slot) = end {
        let e_static: &str = unsafe { std::mem::transmute::<&str, &str>(e) };
        *slot = e_static;
    }
    alt
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_(src, pat).0
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

pub const ASTNULL: i32 = -1;

/// Parses the source string `src` using a given parsing rule.
/// (Corresponds to `ast_t skp_parse(char *src, skprule_t rule, char *rulename, int debug);`)
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
            let info = ast.lastinfo;
            ast_setinfo(&mut ast, info, ASTNULL);
        }
    }
    Some(ast)
}

/// Debug function for AST.
pub fn skp_debug2(ast: &mut Ast, d: u8) -> i32 {
    match d {
        0 => ast.flg &= !0x01,
        1 => ast.flg |= 0x01,
        _ => ast.flg ^= 0x01,
    }
    (ast.flg & 0x01) as i32
}

/// Returns the rule name at which an error occurred.
pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return None;
    }
    ast.err_rule.as_deref()
}

/// Returns the error position as a string pointer.
pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return None;
    }
    let pos = ast.err_pos as usize;
    if pos <= ast.start.len() {
        Some(&ast.start[pos..])
    } else {
        None
    }
}

/// Returns the start of the error line.
pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let bytes = ast.start.as_bytes();
    let mut ln = ast.err_pos as usize;
    if ln > bytes.len() {
        return "";
    }
    while ln > 0 {
        let prev = bytes[ln - 1];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        ln -= 1;
    }
    &ast.start[ln..]
}

/// Returns the error column number.
pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let pos = ast.err_pos as usize;
    let line_start = ast.start.len() - line.len();
    (pos - line_start) as i32
}

/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    let mut ast = Ast::default();
    ast.nodes_cnt = 0;
    ast.nodes_max = 8;
    ast.nodes = Vec::with_capacity(ast.nodes_max as usize);
    ast.par_cnt = 0;
    ast.par_max = 16;
    ast.par = Vec::with_capacity(ast.par_max as usize);
    ast.mmz_cnt = 0;
    ast.mmz_max = 64;
    ast.mmz = Vec::with_capacity(ast.mmz_max as usize);
    ast.lastpos = 0;
    ast.pos = 0;
    ast.fail = 0;
    ast.depth = 0;
    ast.err_msg = Some(String::new());
    ast.err_pos = -1;
    ast.err_rule = None;
    ast.cur_node = ASTNULL;
    ast.cur_rule = None;
    ast.auxptr = None;
    Some(ast)
}

/// Frees an AST.
pub fn astfree(_ast: Ast) -> Option<Ast> {
    // Memory is freed when the Ast goes out of scope; mirror C signature.
    None
}

/// Opens a new AST node starting at position `from` with the given rule.
pub fn ast_open(ast: &mut Ast, from: i32, rule: &str) -> i32 {
    if ast.fail != 0 {
        return -1;
    }
    let par = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;
    let node = ast.nodes_cnt;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from,
        to: 0,
        delta: 0,
        tag: 0,
    });
    ast.nodes_cnt += 1;
    ast.par[par as usize] = node;
    par
}

/// Closes the current AST node at position `to`, linking with the open node `open`.
pub fn ast_close(ast: &mut Ast, to: i32, open: i32) -> i32 {
    if open < 0 {
        return -1;
    }
    let node_idx = ast.par[open as usize] as usize;
    if ast.fail != 0 {
        let from = ast.nodes[node_idx].from;
        ast.pos = from;
        ast.nodes_cnt = ast.par[open as usize];
        ast.par_cnt = open;
        ast.nodes.truncate(ast.nodes_cnt as usize);
        ast.par.truncate(ast.par_cnt as usize);
        return -1;
    }
    let par = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;
    let delta = par - open;
    ast.nodes[node_idx].to = to;
    ast.nodes[node_idx].delta = delta;
    ast.nodes[node_idx].tag = 0;
    ast.par[par as usize] = -delta;
    let rule = ast.nodes[node_idx].rule.clone();
    ast.cur_node = par;
    ast.cur_rule = Some(rule);
    par
}

/// Aborts parsing with the given message and rule.
pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    ast.err_msg = Some(msg.to_string());
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
}

/// Records memoization of AST nodes (for left recursion etc.).
pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // Simplified: we do not implement memoization in the Rust port.
}

/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

/// Sets AST node information.
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    let mut node = node;
    if ast.par_cnt <= node {
        return;
    }
    if node == ASTNULL {
        node = ast.par_cnt - 1;
    }
    if node < 0 {
        return;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    ast.nodes[nidx].tag = info;
}

/// Records a new AST info node.
pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail != 0 {
        return;
    }
    let pos = ast.pos;
    let par = ast_open(ast, pos, "#");
    ast_close(ast, pos, par);
    let nidx = ast.par[par as usize] as usize;
    ast.nodes[nidx].tag = info;
    ast.lastinfo = info;
}

/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    ast.nodes[nidx].tag
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
    let span2: Vec<i32> = ast.par[o2 as usize..=c2 as usize].to_vec();
    let span1: Vec<i32> = ast.par[o1 as usize..=c1 as usize].to_vec();
    let mut new_seq = Vec::with_capacity(span1.len() + span2.len());
    new_seq.extend(span1);
    new_seq.extend(span2);
    for (i, v) in new_seq.into_iter().enumerate() {
        ast.par[o2 as usize + i] = v;
    }
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
    let l_idx = ast.par[lft as usize] as usize;
    let r_idx = ast.par[rgt as usize] as usize;
    let node_from = ast.nodes[l_idx].from;
    let node_to = ast.nodes[r_idx].to;
    rgt += ast.nodes[r_idx].delta;
    let node = ast.nodes_cnt;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta: rgt - lft + 2,
        tag: 0,
    });
    ast.nodes_cnt += 1;
    let delta = rgt - lft + 2;
    // Insert two new par slots: one before lft and one after rgt
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
    let nidx = ast.par[o1 as usize] as usize;
    if ast.nodes[nidx].tag == 0 {
        // Remove o1 and c1 (the outer pair) - shift the inner contents up by 1
        // The C version uses memmove(par+o1, par+o2, (c2-o2+1)*sizeof) and then par_cnt -= 2
        // Effectively: remove indexes o1 and ... actually it overwrites o1..(o1+(c2-o2+1)) with par[o2..c2+1]
        // and then drops the last two par entries.
        let block: Vec<i32> = ast.par[o2 as usize..=c2 as usize].to_vec();
        for (i, v) in block.into_iter().enumerate() {
            ast.par[o1 as usize + i] = v;
        }
        ast.par_cnt -= 2;
        ast.par.truncate(ast.par_cnt as usize);
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
        ast.par.truncate(ast.par_cnt as usize);
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
    let nidx = ast.par[o1 as usize] as usize;
    if ast.nodes[nidx].from != ast.nodes[nidx].to {
        return;
    }
    ast.par_cnt -= 2;
    ast.par.truncate(ast.par_cnt as usize);
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
    let nidx = ast.par[node as usize] as usize;
    ast.nodes[nidx].from == ast.nodes[nidx].to
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
    ast.par.truncate(ast.par_cnt as usize);
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
        let nidx = ast.par[node as usize] as usize;
        node += ast.nodes[nidx].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        return ASTNULL;
    }
    node
}

/// Returns the parent of a node.
pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut node = astfirst(ast, node);
    if node == ASTNULL {
        return ASTNULL;
    }
    node -= 1;
    if node < 0 || ast.par[node as usize] < 0 {
        return ASTNULL;
    }
    node
}

/// Returns the first child of a node.
pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let node = node + 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        return ASTNULL;
    }
    node
}

/// Returns the leftmost sibling (first child) of a node.
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
    let ndx = node + 1;
    if ndx < 0 {
        return 0;
    }
    if ndx >= ast.par_cnt {
        return ASTNULL;
    }
    ndx
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
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    &ast.nodes[nidx].rule
}

/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    let from = ast.nodes[nidx].from as usize;
    if from <= ast.start.len() {
        &ast.start[from..]
    } else {
        ""
    }
}

/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    let to = ast.nodes[nidx].to as usize;
    if to <= ast.start.len() {
        &ast.start[to..]
    } else {
        ""
    }
}

/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    ast.nodes[nidx].to - ast.nodes[nidx].from
}

/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 {
        return false;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    ast.nodes[nidx].delta == 1
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
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let nidx = ast.par[idx] as usize;
    if ast.nodes[nidx].rule == rulename {
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
pub fn astprintsexpr(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: i32 = ASTNULL;
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
                    let len = from_s.len() - to_s.len();
                    let slice = &from_s.as_bytes()[..len];
                    for &b in slice {
                        if b == b'\'' {
                            let _ = f.write_all(b"\\");
                        }
                        let _ = f.write_all(&[b]);
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
    let mut node: i32 = ASTNULL;
    let mut levl: i32 = 0;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL {
            break;
        }
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
                let from_s = astnodefrom(ast, node);
                let to_s = astnodeto(ast, node);
                let len = from_s.len() - to_s.len();
                let slice = &from_s.as_bytes()[..len];
                for &b in slice {
                    if b == b'\'' {
                        let _ = f.write_all(b"\\");
                    }
                    let _ = f.write_all(&[b]);
                }
                let _ = write!(f, "'");
            }
            let _ = writeln!(f);
        } else {
            levl -= 4;
        }
    }
}
