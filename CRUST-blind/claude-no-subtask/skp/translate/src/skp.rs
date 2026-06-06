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
    // Difference between to and start in bytes.
    // In the C version both are pointers into the same buffer, here we use byte
    // length difference: len(start) - len(to) (since `to` is the suffix after `start`).
    let s_len = start.len() as i64;
    let t_len = to.len() as i64;
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

// =========================================================================
// Internal byte-level helpers. These operate on byte slices like the C code.
// =========================================================================

/// Reads the next "code point" from byte slice `s`.
/// Returns (code_point, bytes_consumed). If `s` is empty, returns (0, 0).
/// If iso != 0, only the first byte is consumed (ISO-8859 / single-byte mode).
/// Otherwise, UTF-8 continuation bytes (0x80-0xBF) are concatenated into the
/// returned u32 (not as actual Unicode codepoints, but as raw byte values
/// shifted left by 8 each).
/// CRLF (0x0D 0x0A) is also returned as a single composite "code point" 0x0D0A.
fn skp_next_bytes(s: &[u8], iso: i32) -> (u32, usize) {
    if s.is_empty() {
        return (0, 0);
    }
    let mut c: u32 = s[0] as u32;
    let mut i: usize = 1;
    if iso == 0 {
        // Read up to 3 continuation bytes (matching the C code's #if 1 branch).
        if i < s.len() && (s[i] & 0xC0) == 0x80 {
            c = (c << 8) | s[i] as u32;
            i += 1;
            if i < s.len() && (s[i] & 0xC0) == 0x80 {
                c = (c << 8) | s[i] as u32;
                i += 1;
                if i < s.len() && (s[i] & 0xC0) == 0x80 {
                    c = (c << 8) | s[i] as u32;
                    i += 1;
                }
            }
        }
    }
    // CRLF handling: c == 0x0D and next byte is 0x0A
    if c == 0x0D && i < s.len() && s[i] == 0x0A {
        c = 0x0D0A;
        i += 1;
    }
    (c, i)
}

// =========================================================================
// Public scanning API
// =========================================================================

/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    let mut to: &str = src;
    let mut end: &str = src;
    let ret = skp_4(src, pat, Some(&mut to), Some(&mut end));
    (ret, to, end)
}

pub fn skp_4<'a>(src: &'a str, pat: &str, to: Option<&mut &'a str>, end: Option<&mut &'a str>) -> i32 {
    // Convert to bytes for processing (matching the C version's byte-pointer approach).
    let src_bytes = src.as_bytes();
    let pat_bytes = pat.as_bytes();

    if pat_bytes.is_empty() && src_bytes.is_empty() {
        // Both empty - no pattern to match.
        if let Some(t) = to { *t = src; }
        if let Some(e) = end { *e = src; }
        return 0;
    }

    let mut start: usize = 0; // byte offset into src
    let mut s: usize = start;
    let mut p: usize = 0;     // byte offset into pat

    let mut s_end_idx: usize = s;
    let mut p_end_idx: usize = p;

    let mut skp_to = false;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    if !pat_bytes.is_empty() && pat_bytes[0] == b'>' {
        skp_to = true;
        p += 1;
    }

    // Skip leading spaces (raw byte check via is_space on single byte)
    while p < pat_bytes.len() && is_space_byte(pat_bytes[p]) {
        p += 1;
    }

    // Loop while *p > '\7' (i.e., greater than 0x07).
    while p < pat_bytes.len() && pat_bytes[p] > 7 {
        matched = match_internal(pat_bytes, p, src_bytes, s, &mut p_end_idx, &mut s_end_idx, &mut flg);
        if matched != 0 {
            s = s_end_idx;
            p = p_end_idx;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s);
            }
        } else {
            // Skip to next alternative or end.
            while p < pat_bytes.len() && pat_bytes[p] > 7 {
                p += 1;
            }
            // If there's a next alt (*p > 0 && p[1] > 0), retry.
            if p < pat_bytes.len() && pat_bytes[p] > 0
                && p + 1 < pat_bytes.len() && pat_bytes[p + 1] > 0
            {
                s = start;
                p += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                p = if !pat_bytes.is_empty() && pat_bytes[0] == b'>' { 1 } else { 0 };
                start += 1;
                s = start;
                if start > src_bytes.len() || (start <= src_bytes.len() && start == src_bytes.len()) {
                    // Reached end of src
                    if start > src_bytes.len() { break; }
                    if src_bytes.is_empty() || start == src_bytes.len() {
                        // Test for *s == '\0' break
                        break;
                    }
                }
            } else {
                break;
            }
        }
        while p < pat_bytes.len() && is_space_byte(pat_bytes[p]) {
            p += 1;
        }
    }

    // After loop: if !matched && goalnot, set goal
    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        // p is set to "" - we don't really need to do anything since the
        // following check examines pat_bytes[p].
        // We need *p <= '\7'. Set p to past end (out of range).
        p = pat_bytes.len();
    }

    if let Some(g) = goal {
        s = g;
    }

    // matched && (*p <= '\7') condition
    let p_term_val: u8 = if p < pat_bytes.len() { pat_bytes[p] } else { 0 };

    if matched != 0 && p_term_val <= 7 {
        let ret = if p_term_val > 0 { p_term_val as i32 } else { 1 };
        let to_pos = if skp_to { start } else { s };
        if let Some(t) = to { *t = &src[to_pos..]; }
        if let Some(e) = end { *e = &src[s..]; }
        return ret;
    }

    if let Some(t) = to { *t = src; }
    if let Some(e) = end { *e = src; }
    0
}

/// Internal byte-level match function. Returns the match result.
/// `p_end_idx` and `s_end_idx` are output: indices into pat and src after the match.
fn match_internal(
    pat: &[u8],
    p_start: usize,
    src: &[u8],
    s_start: usize,
    p_end_idx: &mut usize,
    s_end_idx: &mut usize,
    flg: &mut i32,
) -> i32 {
    let mut p = p_start;
    let mut s_end = s_start;
    let mut s_tmp;
    let mut s_chr: u32;
    let mut ret: i32 = MATCHED_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_cnt: u32;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    // Read first character.
    let (ch0, consumed0) = skp_next_bytes(&src[s_end..], *flg & 2);
    s_chr = ch0;
    s_tmp = s_end + consumed0;

    if p < pat.len() {
        match pat[p] {
            b'*' => { match_min = 0; match_max = u32::MAX; p += 1; }
            b'+' => { match_max = u32::MAX; p += 1; }
            b'?' => { match_min = 0; p += 1; }
            _ => {}
        }
    }

    if p < pat.len() && pat[p] == b'!' {
        match_not = 1;
        p += 1;
    }

    // Closure-like macro substitutes
    macro_rules! W {
        ($pred:expr) => {{
            match_cnt = 0;
            while match_cnt < match_max && s_chr != 0 && (($pred as u32) != match_not) {
                s_end = s_tmp;
                let (c2, c2n) = skp_next_bytes(&src[s_end..], *flg & 2);
                s_chr = c2;
                s_tmp = s_end + c2n;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }}
    }

    macro_rules! get_next_s_chr {
        () => {{
            s_end = s_tmp;
            // *s_end ; s_tmp++  -- read single byte
            s_chr = if s_end < src.len() { src[s_end] as u32 } else { 0 };
            s_tmp = s_end + 1;
        }}
    }

    intnumber = false;

    if p >= pat.len() {
        // No pattern character; return fail.
        return MATCHED_FAIL;
    }

    let pat_char = pat[p];
    p += 1;
    match pat_char {
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
                // Fall through to 'n' case
                W!(is_break(s_chr));
            }
        }
        b'n' => { W!(is_break(s_chr)); }
        b'd' => { W!(is_digit(s_chr)); }
        b'x' => { W!(is_xdigit(s_chr)); }
        b'a' => { W!(is_alpha(s_chr)); }
        b'u' => { W!(is_upper(s_chr)); }
        b'l' => { W!(is_lower(s_chr)); }
        b's' => { W!(is_space(s_chr)); }
        b'w' => { W!(is_blank(s_chr)); }
        b'c' => { W!(is_ctrl(s_chr)); }
        b'i' => { W!(is_idchr(s_chr)); }
        b'@' => { W!(is_alnum(s_chr)); }
        b'&' => {
            ret = if match_not != 0 { MATCHED_GOALNOT } else { MATCHED_GOAL };
        }
        b'[' => {
            // is_oneof using set starting at p, going up to ']'
            let set_start = p;
            // Find end of set (matching the C semantics):
            // First ']' is part of set (skip it), then find next ']'.
            // But for is_oneof to work we pass the whole remaining set.
            let pred = is_oneof_bytes(s_chr, &pat[set_start..], *flg & 2);
            W!(pred);
            // Advance p past the set
            if p < pat.len() && pat[p] == b']' {
                p += 1;
            }
            while p < pat.len() && pat[p] != b']' {
                p += 1;
            }
            if p < pat.len() {
                p += 1; // skip ']'
            }
        }
        c @ (b'"' | b'\'' | b'`') => {
            let quote = c;
            let mut l: usize = 0;
            while p + l < pat.len() && pat[p + l] != quote {
                l += 1;
            }
            // Mimic C: if (l>0 && ((ml = is_string(s_end,pat,l,*flg)) > 0)) {
            //            if (!match_not) { s_end += ml; ret = MATCHED; }
            //          } else if (match_min == 0 || match_not) ret = MATCHED;
            let mut taken_if = false;
            if l > 0 {
                let ml = is_string_bytes(&src[s_end..], &pat[p..], l, *flg);
                if ml > 0 {
                    taken_if = true;
                    if match_not == 0 {
                        s_end += ml as usize;
                        ret = MATCHED;
                    }
                }
            }
            if !taken_if && (match_min == 0 || match_not != 0) {
                ret = MATCHED;
            }
            p += l + 1;
            if p > pat.len() { p = pat.len(); }
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
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) { break; }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            if p < pat.len() && pat[p] == b')' && s_chr == b'(' as u32 {
                p += 1;
                // Fall through to balanced parens
                let open = s_chr;
                let close = get_close(open);
                if close != 0 {
                    let mut count: i32 = 1;
                    while s_chr != 0 && count > 0 {
                        get_next_s_chr!();
                        if s_chr == open { count += 1; }
                        if s_chr == close { count -= 1; }
                    }
                    if count == 0 {
                        get_next_s_chr!();
                        ret = MATCHED;
                    }
                }
            }
            // Note: if condition fails, we just break (ret stays MATCHED_FAIL)
        }
        b'B' => {
            let open = s_chr;
            let close = get_close(open);
            if close != 0 {
                let mut count: i32 = 1;
                while s_chr != 0 && count > 0 {
                    get_next_s_chr!();
                    if s_chr == open { count += 1; }
                    if s_chr == close { count -= 1; }
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
                    if s_chr == qclose { break; }
                    if s_chr == b'\\' as u32 { get_next_s_chr!(); }
                }
                if s_chr != 0 {
                    get_next_s_chr!();
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            // Hex number
            let s1 = if s_end + 1 < src.len() { src[s_end + 1] } else { 0 };
            let s2 = if s_end + 2 < src.len() { src[s_end + 2] } else { 0 };
            if s_chr == b'0' as u32
                && (s1 == b'x' || s1 == b'X')
                && is_xdigit(s2 as u32)
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
            // Fall through to F-handling
            number_match(&mut s_chr, &mut s_end, &mut s_tmp, src, &mut ret, intnumber);
        }
        b'F' => {
            number_match(&mut s_chr, &mut s_end, &mut s_tmp, src, &mut ret, false);
        }
        _ => {
            ret = MATCHED_FAIL;
            p -= 1;
        }
    }

    let p_end = p;

    if ret != MATCHED_FAIL {
        *p_end_idx = p_end;
        *s_end_idx = s_end;
    }
    ret
}

fn number_match(
    s_chr: &mut u32,
    s_end: &mut usize,
    s_tmp: &mut usize,
    src: &[u8],
    ret: &mut i32,
    intnumber: bool,
) {
    macro_rules! get_next_s_chr {
        () => {{
            *s_end = *s_tmp;
            *s_chr = if *s_end < src.len() { src[*s_end] as u32 } else { 0 };
            *s_tmp = *s_end + 1;
        }}
    }

    if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
        loop {
            get_next_s_chr!();
            if !is_space(*s_chr) { break; }
        }
    }

    while is_digit(*s_chr) {
        *ret = MATCHED;
        get_next_s_chr!();
    }

    if intnumber { return; }

    if *s_chr == b'.' as u32 {
        get_next_s_chr!();
    }

    while is_digit(*s_chr) {
        *ret = MATCHED;
        get_next_s_chr!();
    }

    if *ret == MATCHED && (*s_chr == b'E' as u32 || *s_chr == b'e' as u32) {
        get_next_s_chr!();
        if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
            get_next_s_chr!();
        }
        while is_digit(*s_chr) { get_next_s_chr!(); }
        if *s_chr == b'.' as u32 { get_next_s_chr!(); }
        while is_digit(*s_chr) { get_next_s_chr!(); }
    }
}

pub fn skp_3<'a>(src: &'a str, pat: &str, end: Option<&mut &'a str>) -> i32 {
    skp_4(src, pat, None, end)
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_4(src, pat, None, None)
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, n) = skp_next_bytes(bytes, iso);
    (c, &s[n..])
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        // ASCII tolower
        if (b'A' as u32..=b'Z' as u32).contains(&a) { a += 32; }
        if (b'A' as u32..=b'Z' as u32).contains(&b) { b += 32; }
    }
    a == b
}

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

pub fn is_break(c: u32) -> bool {
    if c < 0x0F {
        return c == 0x0A || c == 0x0C || c == 0x0D;
    }
    if c < 0xFF {
        return c == 0x85;
    }
    c == 0x0D0A
        || c == 0xC285
        || c == 0xE280A8
        || c == 0xE280A9
}

pub fn is_space(c: u32) -> bool {
    is_blank(c) || is_break(c)
}

pub fn is_digit(c: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&c)
}

pub fn is_xdigit(c: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&c)
        || (b'A' as u32..=b'F' as u32).contains(&c)
        || (b'a' as u32..=b'f' as u32).contains(&c)
}

pub fn is_upper(c: u32) -> bool {
    (b'A' as u32..=b'Z' as u32).contains(&c)
}

pub fn is_lower(c: u32) -> bool {
    (b'a' as u32..=b'z' as u32).contains(&c)
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
    c < 0x20 || (0xC280..0xC2A0).contains(&c) || (0x7F..0xA0).contains(&c)
}

/// Helper: is a byte considered a "space" (single-byte test).
fn is_space_byte(b: u8) -> bool {
    is_space(b as u32)
}

pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    is_oneof_bytes(ch, set.as_bytes(), iso)
}

fn is_oneof_bytes(ch: u32, set: &[u8], iso: i32) -> bool {
    if ch == 0 { return false; }
    let mut idx: usize = 0;
    let (mut p_ch, n) = skp_next_bytes(&set[idx..], iso);
    idx += n;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 { return true; }
        let (pc, n) = skp_next_bytes(&set[idx..], iso);
        p_ch = pc;
        idx += n;
    }

    while p_ch != b']' as u32 {
        if p_ch == ch { return true; }
        let q_ch = p_ch;
        let (pc, n) = skp_next_bytes(&set[idx..], iso);
        p_ch = pc;
        idx += n;
        if p_ch == b'-' as u32 && idx < set.len() && set[idx] != b']' {
            let (pc2, n2) = skp_next_bytes(&set[idx..], iso);
            p_ch = pc2;
            idx += n2;
            if q_ch < ch && ch <= p_ch { return true; }
            let (pc3, n3) = skp_next_bytes(&set[idx..], iso);
            p_ch = pc3;
            idx += n3;
        }
    }
    false
}

pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_bytes(s.as_bytes(), p.as_bytes(), len as usize, flg)
}

fn is_string_bytes(s: &[u8], p: &[u8], len: usize, flg: i32) -> i32 {
    let mut s_idx: usize = 0;
    let mut p_idx: usize = 0;
    let start = s_idx;
    let mut mlen: i32 = 0;
    let mut len = len as i32;

    while len > 0 {
        if p_idx < p.len() && p[p_idx] == 0x0E {
            return mlen;
        }

        let (p_chr, p_n) = skp_next_bytes(&p[p_idx..], flg & 2);
        let (s_chr, s_n) = skp_next_bytes(&s[s_idx..], flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += s_n as i32;
            len -= p_n as i32;
            p_idx += p_n;
            s_idx += s_n;
        } else {
            // Search for an alternative.
            // Mimic C: while (len>0 && *p++ != '\xE') len--;
            // *p++ is post-increment: pointer always advances; len-- only when char != 0xE.
            while len > 0 && p_idx < p.len() {
                let ch = p[p_idx];
                p_idx += 1;
                if ch == 0x0E { break; }
                len -= 1;
            }
            // C: if (len-- <= 0) return 0;
            if len <= 0 {
                return 0;
            }
            len -= 1;
            s_idx = start;
            mlen = 0;
        }
    }
    mlen
}

pub fn get_close(open: u32) -> u32 {
    match open {
        c if c == b'(' as u32 => b')' as u32,
        c if c == b'[' as u32 => b']' as u32,
        c if c == b'{' as u32 => b'}' as u32,
        c if c == b'<' as u32 => b'>' as u32,
        _ => 0,
    }
}

pub fn get_qclose(open: u32) -> u32 {
    if open == b'\'' as u32 || open == b'"' as u32 || open == b'`' as u32 {
        open
    } else {
        0
    }
}

pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let mut p_end_idx: usize = 0;
    let mut s_end_idx: usize = 0;
    let ret = match_internal(
        pat.as_bytes(),
        0,
        src.as_bytes(),
        0,
        &mut p_end_idx,
        &mut s_end_idx,
        flg,
    );
    if ret != MATCHED_FAIL {
        (ret, &src[s_end_idx..], &pat[p_end_idx..])
    } else {
        (ret, src, pat)
    }
}

// =========================================================================
// AST Parsing
// =========================================================================

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

pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { 0x01 } else { 0 };
    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
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
            let li = ast.lastinfo;
            ast_setinfo(&mut ast, li, -1);
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
    if ast.err_pos < 0 { return Some(""); }
    ast.err_rule.as_deref()
}

pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return Some("");
    }
    let p = ast.err_pos as usize;
    if p > ast.start.len() { return Some(""); }
    Some(&ast.start[p..])
}

pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 { return ""; }
    let bytes = ast.start.as_bytes();
    let mut ln = ast.err_pos as usize;
    while ln > 0 {
        let prev = bytes[ln - 1];
        if prev == b'\n' || prev == b'\r' { break; }
        ln -= 1;
    }
    &ast.start[ln..]
}

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 { return 0; }
    let bytes = ast.start.as_bytes();
    let mut ln = ast.err_pos as usize;
    while ln > 0 {
        let prev = bytes[ln - 1];
        if prev == b'\n' || prev == b'\r' { break; }
        ln -= 1;
    }
    (ast.err_pos as usize - ln) as i32
}

pub fn ast_new() -> Option<Ast> {
    let mut ast = Ast::default();
    ast.nodes_cnt = 0;
    ast.nodes_max = 8;
    ast.nodes = Vec::with_capacity(8);
    ast.par_cnt = 0;
    ast.par_max = 16;
    ast.par = Vec::with_capacity(16);
    ast.mmz_cnt = 0;
    ast.mmz_max = 64;
    ast.mmz = Vec::new();
    ast.lastpos = 0;
    ast.pos = 0;
    ast.fail = 0;
    ast.depth = 0;
    ast.err_msg = Some(String::new());
    ast.err_pos = -1;
    ast.err_rule = None;
    ast.cur_node = -1;
    ast.cur_rule = None;
    ast.auxptr = None;
    Some(ast)
}

pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
}

fn skp_par_makeroom(ast: &mut Ast, needed: i32) -> bool {
    if ast.par_cnt + needed > ast.par_max {
        let mut new_max = ast.par_max;
        while ast.par_cnt + needed > new_max {
            new_max += new_max / 2;
            new_max += new_max & 1;
        }
        ast.par.reserve((new_max - ast.par_max) as usize);
        ast.par_max = new_max;
    }
    true
}

fn skp_nodes_makeroom(ast: &mut Ast, needed: i32) -> bool {
    if ast.nodes_cnt + needed > ast.nodes_max {
        let mut new_max = ast.nodes_max;
        while ast.nodes_cnt + needed > new_max {
            new_max += new_max / 2;
            new_max += new_max & 1;
        }
        ast.nodes.reserve((new_max - ast.nodes_max) as usize);
        ast.nodes_max = new_max;
    }
    true
}

fn ast_newpar(ast: &mut Ast) -> i32 {
    if !skp_par_makeroom(ast, 1) { return -1; }
    let v = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;
    v
}

fn ast_newnode(ast: &mut Ast) -> i32 {
    if !skp_nodes_makeroom(ast, 1) { return -1; }
    let v = ast.nodes_cnt;
    ast.nodes.push(AstNode::default());
    ast.nodes_cnt += 1;
    v
}

pub fn ast_open(ast: &mut Ast, from: i32, rule: &str) -> i32 {
    if ast.fail != 0 { return -1; }
    let par = ast_newpar(ast);
    if par < 0 { return -1; }
    let node = ast_newnode(ast);
    if node < 0 { return -1; }
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
    if open < 0 { return -1; }
    let node_idx = ast.par[open as usize];
    if ast.fail != 0 {
        let from = ast.nodes[node_idx as usize].from;
        ast.pos = from;
        ast.nodes_cnt = node_idx;
        ast.nodes.truncate(node_idx as usize);
        ast.par_cnt = open;
        ast.par.truncate(open as usize);
        return -1;
    }
    let par = ast_newpar(ast);
    if par < 0 { return -1; }
    let nd = &mut ast.nodes[node_idx as usize];
    nd.to = to;
    nd.delta = par - open;
    nd.tag = 0;
    let delta = nd.delta;
    let rule = nd.rule.clone();
    ast.par[par as usize] = -delta;
    ast.cur_node = par;
    ast.cur_rule = Some(rule);
    par
}

pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    if !msg.is_empty() {
        ast.err_msg = Some(msg.to_string());
    }
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
    let numnodes = (end_par - start_par) / 2;
    mmz.pos = old_pos;
    mmz.endpos = ast.pos;
    mmz.numnodes = if ast.fail != 0 { -1 } else { numnodes };
    mmz.lastinfo = ast.lastinfo;
    mmz.maxnodes = numnodes;
    mmz.nodes.clear();
    if start_par >= 0 {
        for k in start_par..end_par {
            let pk = ast.par[k as usize];
            if pk >= 0 {
                mmz.nodes.push(ast.nodes[pk as usize].clone());
            }
        }
    }
}

pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    // Simplified: never returns a memoized result (pretends it's first time).
    0
}

pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    let mut node = node;
    if ast.par_cnt <= node { return; }
    if node == -1 { node = ast.par_cnt - 1; }
    if node < 0 || (node as usize) >= ast.par.len() { return; }
    let p = ast.par[node as usize];
    let resolved = if p < 0 { node + p } else { node };
    if resolved < 0 || (resolved as usize) >= ast.par.len() { return; }
    let pidx = ast.par[resolved as usize];
    if pidx < 0 || (pidx as usize) >= ast.nodes.len() { return; }
    ast.nodes[pidx as usize].tag = info;
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail == 0 {
        let par = ast_open(ast, ast.pos, "#");
        ast_close(ast, ast.pos, par);
        if par >= 0 {
            let pidx = ast.par[par as usize];
            if pidx >= 0 && (pidx as usize) < ast.nodes.len() {
                ast.nodes[pidx as usize].tag = info;
            }
        }
        ast.lastinfo = info;
    }
}

pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 { return 0; }
    let mut node = node;
    let p = ast.par[node as usize];
    if p < 0 { node += p; }
    if node < 0 || (node as usize) >= ast.par.len() { return 0; }
    let pidx = ast.par[node as usize];
    if pidx < 0 || (pidx as usize) >= ast.nodes.len() { return 0; }
    ast.nodes[pidx as usize].tag
}

pub fn ast_swap(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    let c2 = o1 - 1;
    if c2 < 0 || ast.par[c2 as usize] >= 0 { return; }
    let o2 = c2 + ast.par[c2 as usize];
    if o2 < 0 || ast.par[o2 as usize] < 0 { return; }

    let block1: Vec<i32> = ast.par[(o2 as usize)..=(c2 as usize)].to_vec();
    let block2: Vec<i32> = ast.par[(o1 as usize)..=(c1 as usize)].to_vec();

    let mut idx = o2 as usize;
    for v in &block2 {
        ast.par[idx] = *v;
        idx += 1;
    }
    for v in &block1 {
        ast.par[idx] = *v;
        idx += 1;
    }
}

pub fn ast_lower(ast: &mut Ast, rule: &str, f: AstNodeT, t: AstNodeT) {
    let mut lft = f;
    let mut rgt = t;
    if ast.par_cnt <= lft || ast.par_cnt <= rgt || lft >= rgt { return; }

    if ast.par[lft as usize] < 0 { lft += ast.par[lft as usize]; }
    if ast.par[rgt as usize] < 0 { rgt += ast.par[rgt as usize]; }

    if lft < 0 || rgt < 0 { return; }

    let node_from = ast.nodes[ast.par[lft as usize] as usize].from;
    let node_to = ast.nodes[ast.par[rgt as usize] as usize].to;
    let rgt_delta = ast.nodes[ast.par[rgt as usize] as usize].delta;
    let new_rgt = rgt + rgt_delta;

    let node = ast_newnode(ast);
    if node < 0 { return; }

    let delta = new_rgt - lft + 2;
    ast.nodes[node as usize] = AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta,
        tag: 0,
    };

    if ast_newpar(ast) < 0 { return; }
    if ast_newpar(ast) < 0 { return; }

    // Save the elements from new_rgt+1 to old par_cnt-3 since they need to be shifted.
    let par_cnt = ast.par_cnt;
    // Move nodes after rgt: par[rgt+3] = par[rgt+1] for (par_cnt-1-rgt-2) elements
    let after_count = par_cnt - 1 - new_rgt - 2;
    if after_count > 0 {
        for i in (0..after_count).rev() {
            ast.par[(new_rgt + 3 + i) as usize] = ast.par[(new_rgt + 1 + i) as usize];
        }
    }
    // Move nodes from lft to new_rgt right by 1: par[lft+1] = par[lft] for (new_rgt - lft + 1) elements
    let move_count = new_rgt - lft + 1;
    for i in (0..move_count).rev() {
        ast.par[(lft + 1 + i) as usize] = ast.par[(lft + i) as usize];
    }

    ast.par[lft as usize] = node;
    ast.par[(new_rgt + 2) as usize] = -delta;
}

pub fn ast_lift(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 4 { return; }

    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }

    let c2 = c1 - 1;
    if c2 < 0 || ast.par[c2 as usize] >= 0 { return; }

    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }

    let o2 = c2 + ast.par[c2 as usize];
    if o2 < 0 || ast.par[o2 as usize] < 0 { return; }

    if o2 != o1 + 1 { return; }

    if ast.nodes[ast.par[o1 as usize] as usize].tag == 0 {
        // memmove(par+o1, par+o2, (c2-o2+1)*sizeof)
        let count = (c2 - o2 + 1) as usize;
        for i in 0..count {
            ast.par[o1 as usize + i] = ast.par[o2 as usize + i];
        }
        ast.par_cnt -= 2;
        ast.par.truncate(ast.par_cnt as usize);
    }
}

pub fn ast_lift_all(ast: &mut Ast) {
    loop {
        let n = ast.par_cnt;
        ast_lift(ast);
        if n == ast.par_cnt { break; }
    }
}

pub fn ast_noleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    if c1 == o1 + 1 {
        ast.par_cnt -= 2;
        ast.par.truncate(ast.par_cnt as usize);
    }
}

pub fn ast_noemptyleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    if c1 != o1 + 1 { return; }
    let nd = &ast.nodes[ast.par[o1 as usize] as usize];
    if nd.from != nd.to { return; }
    ast.par_cnt -= 2;
    ast.par.truncate(ast.par_cnt as usize);
}

pub fn ast_lastnode(ast: &Ast) -> AstNodeT {
    if ast.fail != 0 || ast.par_cnt < 2 { return -1; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return -1; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return -1; }
    o1
}

pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == -1 { return false; }
    if ast.par[node as usize] < 0 { return false; }
    let nd = &ast.nodes[ast.par[node as usize] as usize];
    nd.from == nd.to
}

pub fn ast_delete(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    ast.par_cnt -= c1 - o1 + 1;
    ast.par.truncate(ast.par_cnt as usize);
}

pub fn astleft(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut node = node;
    if node <= 0 || ast.par_cnt <= node { return -1; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    node -= 1;
    if node <= 0 || ast.par[node as usize] >= 0 { return -1; }
    node + ast.par[node as usize]
}

pub fn astright(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut node = node;
    if node <= 0 || ast.par_cnt <= node { return -1; }
    if ast.par[node as usize] > 0 {
        let pidx = ast.par[node as usize];
        node += ast.nodes[pidx as usize].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 { return -1; }
    node
}

pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut node = astfirst(ast, node);
    if node == -1 { return -1; }
    node -= 1;
    if node < 0 || ast.par[node as usize] < 0 { return -1; }
    node
}

pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node { return -1; }
    let n = node + 1;
    if n >= ast.par_cnt || ast.par[n as usize] < 0 { return -1; }
    n
}

pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node { return -1; }
    let mut result = node;
    let mut n = node;
    loop {
        let next = astleft(ast, n);
        if next == -1 { break; }
        result = next;
        n = next;
    }
    result
}

pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node { return -1; }
    let mut result = node;
    let mut n = node;
    loop {
        let next = astright(ast, n);
        if next == -1 { break; }
        result = next;
        n = next;
    }
    result
}

pub fn astnextdf(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let n = node + 1;
    if n < 0 { return 0; }
    if n >= ast.par_cnt { return -1; }
    n
}

pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && 0 <= node && ast.par[node as usize] >= 0
}

pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && 0 <= node && ast.par[node as usize] < 0
}

pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    let mut node = node;
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 || (node as usize) >= ast.par.len() { return ""; }
    let pidx = ast.par[node as usize];
    if pidx < 0 || (pidx as usize) >= ast.nodes.len() { return ""; }
    &ast.nodes[pidx as usize].rule
}

pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    let mut node = node;
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return ""; }
    let pidx = ast.par[node as usize];
    if pidx < 0 { return ""; }
    let from = ast.nodes[pidx as usize].from as usize;
    if from > ast.start.len() { return ""; }
    &ast.start[from..]
}

pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    let mut node = node;
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return ""; }
    let pidx = ast.par[node as usize];
    if pidx < 0 { return ""; }
    let to = ast.nodes[pidx as usize].to as usize;
    if to > ast.start.len() { return ""; }
    &ast.start[to..]
}

pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 { return 0; }
    let mut node = node;
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return 0; }
    let pidx = ast.par[node as usize];
    if pidx < 0 { return 0; }
    let nd = &ast.nodes[pidx as usize];
    nd.to - nd.from
}

pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 { return false; }
    let mut node = node;
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return false; }
    let pidx = ast.par[node as usize];
    if pidx < 0 { return false; }
    ast.nodes[pidx as usize].delta == 1
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
    if ast_is(ast, node, r1) != 0 { return 1; }
    if let Some(r) = r2 { if ast_is(ast, node, r) != 0 { return 1; } }
    if let Some(r) = r3 { if ast_is(ast, node, r) != 0 { return 1; } }
    if let Some(r) = r4 { if ast_is(ast, node, r) != 0 { return 1; } }
    if let Some(r) = r5 { if ast_is(ast, node, r) != 0 { return 1; } }
    0
}

pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == -1 || node >= ast.par_cnt || node < 0 { return 0; }
    let mut node = node;
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return 0; }
    let pidx = ast.par[node as usize];
    if pidx < 0 { return 0; }
    if ast.nodes[pidx as usize].rule == rulename { 1 } else { 0 }
}

pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

pub fn astprintsexpr(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: AstNodeT = -1;
    loop {
        node = astnextdf(ast, node);
        if node == -1 { break; }
        if astisnodeentry(ast, node) {
            let rule = astnoderule(ast, node).to_string();
            let _ = write!(f, "({} ", rule);
            if astisleaf(ast, node) {
                let _ = write!(f, "'");
                if rule == "#" {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from = astnodefrom(ast, node);
                    let to_str = astnodeto(ast, node);
                    let len = from.len() - to_str.len();
                    let bytes = &from.as_bytes()[..len];
                    for &b in bytes {
                        if b == b'\'' { let _ = write!(f, "\\"); }
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

pub fn astprinttree(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: AstNodeT = -1;
    let mut levl: i32 = 0;
    loop {
        node = astnextdf(ast, node);
        if node == -1 { break; }
        if astisnodeentry(ast, node) {
            let mut k = 0;
            while k < levl { let _ = write!(f, "    "); k += 4; }
            let rule = astnoderule(ast, node).to_string();
            let _ = write!(f, "[{}", rule);
            let tag = astnodeinfo(ast, node);
            if tag != 0 { let _ = write!(f, " ({})", tag); }
            let _ = write!(f, "]");
            levl += 4;
            if astisleaf(ast, node) {
                let _ = write!(f, " '");
                let from = astnodefrom(ast, node);
                let to_str = astnodeto(ast, node);
                let len = from.len() - to_str.len();
                let bytes = &from.as_bytes()[..len];
                for &b in bytes {
                    if b == b'\'' { let _ = write!(f, "\\"); }
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
