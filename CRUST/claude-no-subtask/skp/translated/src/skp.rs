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
    if 0 <= ret && ret <= (1 << 16) {
        ret
    } else {
        0
    }
}
/// Global variable used in the C code.
pub static mut SKP_ZERO: i32 = 0;
/// Trace function (corresponds to the C macro skptrace).
pub fn skptrace(args: std::fmt::Arguments) {
    eprintln!("TRCE: {}", args);
}

// =============================================================================
// Core scanning helpers (work on byte indices internally for C-equivalent
// behavior). All public functions take/return `&str` since the inputs are
// guaranteed to be valid UTF-8 and we always return suffixes.
// =============================================================================

/// Returns true if `c` is a blank character.
pub fn is_blank(c: u32) -> bool {
    if c < 0xFF {
        return c == 0x20 || c == 0x09;
    }
    match c & 0xFFFFFF00 {
        0x00000000 => c == 0xA0,
        0x0000C200 => c == 0xC2A0,
        0x00E19A00 => c == 0xE19A80,
        0x00E28000 => (0xE28080 <= c && c <= 0xE2808A) || c == 0xE280AF,
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

pub fn is_space(c: u32) -> bool {
    is_blank(c) || is_break(c)
}

pub fn is_digit(c: u32) -> bool {
    c >= b'0' as u32 && c <= b'9' as u32
}

pub fn is_xdigit(c: u32) -> bool {
    (c >= b'0' as u32 && c <= b'9' as u32)
        || (c >= b'A' as u32 && c <= b'F' as u32)
        || (c >= b'a' as u32 && c <= b'f' as u32)
}

pub fn is_upper(c: u32) -> bool {
    c >= b'A' as u32 && c <= b'Z' as u32
}

pub fn is_lower(c: u32) -> bool {
    c >= b'a' as u32 && c <= b'z' as u32
}

pub fn is_alpha(c: u32) -> bool {
    is_upper(c) || is_lower(c)
}

pub fn is_idchr(c: u32) -> bool {
    is_alpha(c) || is_digit(c) || c == b'_' as u32
}

pub fn is_alnum(c: u32) -> bool {
    is_alpha(c) || is_digit(c)
}

pub fn is_ctrl(c: u32) -> bool {
    c < 0x20 || (0xC280 <= c && c < 0xC2A0) || (0x7F <= c && c < 0xA0)
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let (mut a, mut b) = (a, b);
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        if (b'A' as u32) <= a && a <= (b'Z' as u32) {
            a += 0x20;
        }
        if (b'B' as u32 - 1) < b && b <= (b'Z' as u32) {
            // (Compute b's lowercase too)
        }
        if (b'A' as u32) <= b && b <= (b'Z' as u32) {
            b += 0x20;
        }
    }
    a == b
}

// -----------------------------------------------------------------------------
// Byte-level helper: read next UTF-8 char (or single byte if iso) starting at
// position `i` in `s`. Returns `(packed_code, new_i)`.
// -----------------------------------------------------------------------------
fn skp_next_bytes(s: &[u8], i: usize, iso: i32) -> (u32, usize) {
    if i >= s.len() || s[i] == 0 {
        return (0, i);
    }
    let mut idx = i;
    let mut c: u32 = s[idx] as u32;
    idx += 1;
    if iso == 0 {
        // up to 3 continuation bytes
        if idx < s.len() && (s[idx] & 0xC0) == 0x80 {
            c = (c << 8) | (s[idx] as u32);
            idx += 1;
            if idx < s.len() && (s[idx] & 0xC0) == 0x80 {
                c = (c << 8) | (s[idx] as u32);
                idx += 1;
                if idx < s.len() && (s[idx] & 0xC0) == 0x80 {
                    c = (c << 8) | (s[idx] as u32);
                    idx += 1;
                }
            }
        }
    }
    if c == 0x0D && idx < s.len() && s[idx] == 0x0A {
        c = 0x0D0A;
        idx += 1;
    }
    (c, idx)
}

/// Returns the next Unicode code point from the string `s`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, idx) = skp_next_bytes(bytes, 0, iso);
    // Safety: the input is valid UTF-8 and idx falls on a UTF-8 boundary
    // because skp_next_bytes either consumes a full UTF-8 char or stops at
    // an ASCII boundary.
    let rest = &s[idx..];
    (c, rest)
}

/// `is_oneof` operating on bytes.
fn is_oneof_bytes(ch: u32, set: &[u8], iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let mut i = 0;
    let (mut p_ch, ni) = skp_next_bytes(set, i, iso);
    i = ni;
    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let (c, ni2) = skp_next_bytes(set, i, iso);
            p_ch = c;
            i = ni2;
        }
    }
    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (c2, ni3) = skp_next_bytes(set, i, iso);
        p_ch = c2;
        i = ni3;
        // Check for range "x-y"
        let next_byte = set.get(i).copied().unwrap_or(0);
        if p_ch == b'-' as u32 && next_byte != b']' && next_byte != 0 {
            let (c3, ni4) = skp_next_bytes(set, i, iso);
            p_ch = c3;
            i = ni4;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (c4, ni5) = skp_next_bytes(set, i, iso);
            p_ch = c4;
            i = ni5;
        }
    }
    false
}

pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    is_oneof_bytes(ch, set.as_bytes(), iso)
}

/// Pattern alternative-aware string match (operating on bytes).
fn is_string_bytes(s: &[u8], p: &[u8], len_in: i32, flg: i32) -> i32 {
    let start = 0usize;
    let mut s_i = start;
    let mut p_i = 0usize;
    let mut len = len_in;
    let mut mlen: i32 = 0;
    while len > 0 {
        if p_i < p.len() && p[p_i] == 0x0E {
            return mlen;
        }
        let (p_chr, p_end) = skp_next_bytes(p, p_i, flg & 2);
        let (s_chr, s_end) = skp_next_bytes(s, s_i, flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += (s_end - s_i) as i32;
            len -= (p_end - p_i) as i32;
            p_i = p_end;
            s_i = s_end;
        } else {
            // search for an alternative
            while len > 0 && p_i < p.len() {
                let b = p[p_i];
                p_i += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
            len -= 1;
            if len < 0 {
                return 0;
            }
            s_i = start;
            mlen = 0;
        }
    }
    mlen
}

pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_bytes(s.as_bytes(), p.as_bytes(), len, flg)
}

pub fn get_close(open: u32) -> u32 {
    match open as u8 as char {
        '(' => ')' as u32,
        '[' => ']' as u32,
        '{' => '}' as u32,
        '<' => '>' as u32,
        _ => 0,
    }
}

pub fn get_qclose(open: u32) -> u32 {
    match open as u8 as char {
        '\'' | '"' | '`' => open,
        _ => 0,
    }
}

pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

// -----------------------------------------------------------------------------
// Internal byte-based match function. Takes pattern byte slice + index,
// source byte slice + index, and returns (ret, src_end_idx, pat_end_idx).
// -----------------------------------------------------------------------------
fn match_bytes(
    pat: &[u8],
    pat_start: usize,
    src: &[u8],
    src_start: usize,
    flg: &mut i32,
) -> (i32, usize, usize) {
    let mut p_i = pat_start;
    let mut ret: i32 = MATCHED_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    let mut s_end = src_start;
    let (mut s_chr, mut s_tmp) = skp_next_bytes(src, s_end, *flg & 2);

    // Quantifiers
    if p_i < pat.len() {
        match pat[p_i] {
            b'*' => {
                match_min = 0;
                match_max = u32::MAX;
                p_i += 1;
            }
            b'+' => {
                match_max = u32::MAX;
                p_i += 1;
            }
            b'?' => {
                match_min = 0;
                p_i += 1;
            }
            _ => {}
        }
    }
    if p_i < pat.len() && pat[p_i] == b'!' {
        match_not = 1;
        p_i += 1;
    }

    if p_i >= pat.len() {
        return (MATCHED_FAIL, src_start, p_i);
    }

    // Helper macros translated to closures-in-flow:
    // W(x): Match the predicate x repeatedly between match_min and match_max
    // get_next_s_chr(): Move s by 1 byte (single-byte get next)

    // We use small inline helpers via a macro-like pattern:
    // For W: takes a `predicate` evaluated against current s_chr.
    let chr_byte = pat[p_i];
    p_i += 1;

    intnumber = false;

    macro_rules! do_w {
        ($cond:expr) => {{
            let mut match_cnt: u32 = 0;
            while match_cnt < match_max && (s_chr != 0 && (($cond) != (match_not != 0))) {
                s_end = s_tmp;
                let (c, t) = skp_next_bytes(src, s_end, *flg & 2);
                s_chr = c;
                s_tmp = t;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    macro_rules! get_next_s_chr {
        () => {{
            s_end = s_tmp;
            s_chr = if s_end < src.len() { src[s_end] as u32 } else { 0 };
            s_tmp = s_end + 1;
        }};
    }

    match chr_byte {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                do_w!(s_chr != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                do_w!(is_break(s_chr));
            }
        }
        b'n' => {
            do_w!(is_break(s_chr));
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
            ret = if match_not != 0 { MATCHED_GOALNOT } else { MATCHED_GOAL };
        }

        b'[' => {
            do_w!(is_oneof_bytes(s_chr, &pat[p_i..], *flg & 2));
            // Skip past the set in the pattern
            if p_i < pat.len() && pat[p_i] == b']' {
                p_i += 1;
            }
            while p_i < pat.len() && pat[p_i] != 0 && pat[p_i] != b']' {
                p_i += 1;
            }
            if p_i < pat.len() {
                p_i += 1;
            }
        }

        q @ (b'"' | b'\'' | b'`') => {
            let quote = q;
            let mut l: i32 = 0;
            while p_i + (l as usize) < pat.len()
                && pat[p_i + l as usize] != 0
                && pat[p_i + l as usize] != quote
            {
                l += 1;
            }
            let ml;
            if l > 0
                && {
                    ml = is_string_bytes(&src[s_end..], &pat[p_i..], l, *flg);
                    ml > 0
                }
            {
                if match_not == 0 {
                    s_end += ml as usize;
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            }
            p_i += (l as usize) + 1;
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
            // Followed by ')'? Then it's the parenthesis-only case: "()", which
            // requires source to start with '('. After that fall through to
            // balanced-parenthesis logic.
            if !(p_i < pat.len() && pat[p_i] == b')' && s_chr == b'(' as u32) {
                // No match
            } else {
                p_i += 1;
                // Fall-through to balanced parenthesis ('B' case) logic:
                let open = s_chr;
                let close = get_close(open);
                if close != 0 {
                    let mut count: i32 = 1;
                    while s_chr != 0 && count > 0 {
                        get_next_s_chr!();
                        if s_chr == open {
                            count += 1;
                        }
                        if s_chr == close {
                            count -= 1;
                        }
                    }
                    if count == 0 {
                        get_next_s_chr!();
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
                    get_next_s_chr!();
                    if s_chr == open {
                        count += 1;
                    }
                    if s_chr == close {
                        count -= 1;
                    }
                }
                if count == 0 {
                    get_next_s_chr!();
                    ret = MATCHED;
                }
            }
        }

        b'Q' => {
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
            // hex number
            if s_chr == b'0' as u32
                && s_end + 2 < src.len()
                && (src[s_end + 1] == b'x' || src[s_end + 1] == b'X')
                && is_xdigit(src[s_end + 2] as u32)
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

        b'D' => {
            intnumber = true;
            // sign
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

        b'F' => {
            // sign
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

        _ => {
            ret = MATCHED_FAIL;
            p_i -= 1;
        }
    }

    let _ = intnumber;
    if ret != MATCHED_FAIL {
        (ret, s_end, p_i)
    } else {
        // Even on fail, return p_i so caller can proceed
        (MATCHED_FAIL, src_start, p_i)
    }
}

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`. The flag parameter is passed by mutable reference.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let (ret, s_end, p_end) = match_bytes(pat.as_bytes(), 0, src.as_bytes(), 0, flg);
    if ret != MATCHED_FAIL {
        (ret, &src[s_end..], &pat[p_end..])
    } else {
        (ret, src, &pat[p_end..])
    }
}

// -----------------------------------------------------------------------------
// skp_ implementation
// -----------------------------------------------------------------------------
/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    let (alt, to_idx, end_idx) = skp_inner(src.as_bytes(), pat.as_bytes());
    // Slice the input by byte indices. Because src is valid UTF-8 and we only
    // ever advance on single-byte boundaries (ASCII) or full UTF-8 char
    // boundaries (per skp_next), the indices fall on char boundaries.
    let to = &src[to_idx..];
    let end = &src[end_idx..];
    (alt, to, end)
}

fn skp_inner(src: &[u8], pat: &[u8]) -> (i32, usize, usize) {
    if pat.is_empty() {
        return (0, 0, 0);
    }
    let start_i: usize = 0;
    let mut start = start_i;

    let mut p_i = 0usize;
    let mut s_i = start;

    let mut skp_to = 0;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    // Handle leading '>'
    if !pat.is_empty() && pat[0] == b'>' {
        skp_to = 1;
        p_i = 1;
    }
    let pat_orig_start = p_i;

    // Skip leading spaces
    while p_i < pat.len() && {
        let c = pat[p_i] as u32;
        is_space(c)
    } {
        p_i += 1;
    }

    while p_i < pat.len() && pat[p_i] > b'\x07' {
        let (m, s_end, p_end) = match_bytes(pat, p_i, src, s_i, &mut flg);
        if m != 0 {
            matched = m;
            s_i = s_end;
            p_i = p_end;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s_i);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s_i);
            }
        } else {
            // Scan to next alternative or end
            p_i = p_end;
            while p_i < pat.len() && pat[p_i] > b'\x07' {
                p_i += 1;
            }
            // Try a new alternative pattern
            if p_i + 1 < pat.len() && pat[p_i] > 0 && pat[p_i + 1] > 0 {
                s_i = start;
                p_i += 1;
            } else if skp_to != 0 {
                goal = None;
                goalnot = None;
                p_i = pat_orig_start;
                start += 1;
                s_i = start;
                if start >= src.len() || src[start] == 0 {
                    break;
                }
            } else {
                break;
            }
        }
        // Skip spaces
        while p_i < pat.len() && {
            let c = pat[p_i] as u32;
            is_space(c)
        } {
            p_i += 1;
        }
    }

    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        // emulate p="": treat as p_i pointing past end (i.e. byte "0")
    }

    if let Some(g) = goal {
        s_i = g;
    }

    // p[*p<=7] check
    let p_byte = if matched != 0 && goalnot.is_some() && goal.is_some() && {
        // when we set p="" above (because goalnot caused match), we should treat
        // as success regardless of the pattern. Use an explicit synthetic 0.
        true
    } && goal == goalnot
    {
        0u8
    } else if p_i < pat.len() {
        pat[p_i]
    } else {
        0u8
    };

    if matched != 0 && p_byte <= b'\x07' {
        let ret = if p_byte > 0 { p_byte as i32 } else { 1 };
        let to = if skp_to != 0 { start_i } else { s_i };
        let end = s_i;
        return (ret, to, end);
    }

    (0, 0, 0)
}

// =============================================================================
// Wrappers for skp variants
// =============================================================================

pub fn skp_4<'a>(
    src: &'a str,
    pat: &'a str,
    to: Option<&mut &'a str>,
    end: Option<&mut &'a str>,
) -> i32 {
    let (alt, t, e) = skp_(src, pat);
    if let Some(out) = to {
        *out = t;
    }
    if let Some(out) = end {
        *out = e;
    }
    alt
}

pub fn skp_3<'a>(src: &'a str, pat: &'a str, end: Option<&mut &'a str>) -> i32 {
    let (alt, _t, e) = skp_(src, pat);
    if let Some(out) = end {
        *out = e;
    }
    alt
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    let (alt, _t, _e) = skp_(src, pat);
    alt
}

// =============================================================================
// AST data structures and functions
// =============================================================================

/// In C: `typedef int32_t astnode_t;`
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

pub const SKP_DEBUG: i8 = 0x01;
pub const ASTNULL: i32 = -1;

pub fn ast_new() -> Option<Ast> {
    let mut ast = Ast::default();
    ast.nodes_max = 8;
    ast.par_max = 16;
    ast.mmz_max = 64;
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

pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
}

pub fn ast_newpar(ast: &mut Ast) -> i32 {
    let i = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;
    i
}

pub fn ast_newnode(ast: &mut Ast) -> i32 {
    let i = ast.nodes_cnt;
    ast.nodes.push(AstNode::default());
    ast.nodes_cnt += 1;
    i
}

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
        ast.nodes.truncate(ast.nodes_cnt as usize);
        ast.par.truncate(ast.par_cnt as usize);
        return -1;
    }
    let par = ast_newpar(ast);
    let delta = par - open;
    {
        let nd = &mut ast.nodes[node_idx as usize];
        nd.to = to;
        nd.delta = delta;
        nd.tag = 0;
    }
    ast.par[par as usize] = -delta;
    ast.cur_node = par;
    ast.cur_rule = Some(ast.nodes[node_idx as usize].rule.clone());
    par
}

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
    Some(ast)
}

pub fn skp_debug2(ast: &mut Ast, d: u8) -> i32 {
    match d {
        0 => ast.flg &= !SKP_DEBUG,
        1 => ast.flg |= SKP_DEBUG,
        _ => ast.flg ^= SKP_DEBUG,
    }
    (ast.flg & SKP_DEBUG) as i32
}

pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return None;
    }
    ast.err_rule.as_deref()
}

pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return None;
    }
    Some(&ast.start[ast.err_pos as usize..])
}

pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let bytes = ast.start.as_bytes();
    let mut ln = ast.err_pos as usize;
    while ln > 0 {
        let prev = bytes[ln - 1];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        ln -= 1;
    }
    &ast.start[ln..]
}

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let line_start_offset = ast.start.len() - line.len();
    (ast.err_pos as usize - line_start_offset) as i32
}

pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    ast.err_msg = Some(msg.to_string());
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
}

pub fn skp_memoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str, old_pos: i32, start_par: i32) {
    let mut start_par = start_par;
    let mut end_par = ast.par_cnt;
    if ast.fail != 0 || end_par <= start_par {
        start_par = -1;
        end_par = -1;
    }
    let numnodes = if start_par >= 0 {
        (end_par - start_par) / 2
    } else {
        0
    };
    mmz.pos = old_pos;
    mmz.endpos = ast.pos;
    mmz.numnodes = if ast.fail != 0 { -1 } else { numnodes };
    mmz.lastinfo = ast.lastinfo;
    mmz.nodes.clear();
    if start_par >= 0 {
        for k in start_par..end_par {
            if ast.par[k as usize] >= 0 {
                let nd = &ast.nodes[ast.par[k as usize] as usize];
                mmz.nodes.push(nd.clone());
            }
        }
    }
    mmz.maxnodes = mmz.nodes.len() as i32;
}

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
        // Rebuild the parenthesis structure
        let mut par_indices = Vec::with_capacity((2 * numnodes) as usize);
        for _ in 0..(2 * numnodes) {
            par_indices.push(i32::MAX);
        }
        let base = ast.par_cnt;
        for ent in &par_indices {
            ast.par.push(*ent);
        }
        ast.par_cnt += 2 * numnodes;

        let mut cur_par = base;
        for k in 0..numnodes {
            // Copy node
            let node = mmz.nodes[k as usize].clone();
            let delta = node.delta;
            let nidx = ast.nodes_cnt;
            ast.nodes.push(node);
            ast.nodes_cnt += 1;
            // Find next free par slot
            while ast.par[cur_par as usize] != i32::MAX {
                cur_par += 1;
            }
            ast.par[cur_par as usize] = nidx;
            ast.par[(cur_par + delta) as usize] = -delta;
        }
    }
    1
}

pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= node {
        return;
    }
    let mut node = node;
    if node == ASTNULL {
        node = ast.par_cnt - 1;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let nidx = ast.par[node as usize] as usize;
    ast.nodes[nidx].tag = info;
}

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

pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    ast.nodes[ast.par[node as usize] as usize].tag
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
    let mid: Vec<i32> = ast.par[(o2 as usize)..=(c2 as usize)].to_vec();
    let later: Vec<i32> = ast.par[(o1 as usize)..=(c1 as usize)].to_vec();
    let mut k = o2 as usize;
    for v in &later {
        ast.par[k] = *v;
        k += 1;
    }
    for v in &mid {
        ast.par[k] = *v;
        k += 1;
    }
}

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
    let node_from = ast.nodes[ast.par[lft as usize] as usize].from;
    let node_to = ast.nodes[ast.par[rgt as usize] as usize].to;
    rgt += ast.nodes[ast.par[rgt as usize] as usize].delta;

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
    ast_newpar(ast);
    ast_newpar(ast);

    // shift par[rgt+1 ..] by 2 positions
    let par_cnt = ast.par_cnt;
    if par_cnt - 1 - rgt > 2 {
        let mut k = (par_cnt - 1) as usize;
        while k >= (rgt + 3) as usize {
            ast.par[k] = ast.par[k - 2];
            if k == 0 {
                break;
            }
            k -= 1;
        }
    }
    // shift par[lft .. rgt] by 1 position (memmove right by 1)
    {
        let mut k = (rgt + 1) as usize;
        while k > lft as usize {
            ast.par[k] = ast.par[k - 1];
            k -= 1;
        }
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
        // memmove ast.par[o1..o1+(c2-o2+1)] = ast.par[o2..o2+(c2-o2+1)]
        let len = (c2 - o2 + 1) as usize;
        for k in 0..len {
            ast.par[(o1 as usize) + k] = ast.par[(o2 as usize) + k];
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
    let nidx = ast.par[node as usize] as usize;
    let nd = &ast.nodes[nidx];
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
    let mut current = node;
    loop {
        let n = astleft(ast, current);
        if n == ASTNULL {
            break;
        }
        current = n;
    }
    current
}

pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut current = node;
    loop {
        let n = astright(ast, current);
        if n == ASTNULL {
            break;
        }
        current = n;
    }
    current
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

pub fn astnext(ast: &Ast, node: AstNodeT) -> AstNodeT {
    astnextdf(ast, node)
}

pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && 0 <= node && ast.par[node as usize] >= 0
}

pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && 0 <= node && ast.par[node as usize] < 0
}

pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    &ast.nodes[ast.par[node as usize] as usize].rule
}

pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let from = ast.nodes[ast.par[node as usize] as usize].from as usize;
    if from <= ast.start.len() {
        &ast.start[from..]
    } else {
        ""
    }
}

pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let to = ast.nodes[ast.par[node as usize] as usize].to as usize;
    if to <= ast.start.len() {
        &ast.start[to..]
    } else {
        ""
    }
}

pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let nd = &ast.nodes[ast.par[node as usize] as usize];
    nd.to - nd.from
}

pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 {
        return false;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    ast.nodes[ast.par[node as usize] as usize].delta == 1
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

pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let nd = &ast.nodes[ast.par[node as usize] as usize];
    if nd.rule == rulename {
        1
    } else {
        0
    }
}

pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

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
                let from_s = astnodefrom(ast, node);
                let to_s = astnodeto(ast, node);
                let len = from_s.len().saturating_sub(to_s.len());
                for &c in from_s.as_bytes()[..len].iter() {
                    if c == b'\'' {
                        let _ = write!(f, "\\");
                    }
                    let _ = f.write_all(&[c]);
                }
                let _ = write!(f, "'");
            }
        } else {
            let _ = write!(f, ")");
        }
    }
}

pub fn astprinttree(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: i32 = ASTNULL;
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
                let from_s = astnodefrom(ast, node);
                let to_s = astnodeto(ast, node);
                let len = from_s.len().saturating_sub(to_s.len());
                for &c in from_s.as_bytes()[..len].iter() {
                    if c == b'\'' {
                        let _ = write!(f, "\\");
                    }
                    let _ = f.write_all(&[c]);
                }
                let _ = write!(f, "'");
            }
            let _ = writeln!(f);
        } else {
            levl -= 4;
        }
    }
}
