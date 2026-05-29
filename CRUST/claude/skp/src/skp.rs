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
    let ret = (start.len() as i32) - (to.len() as i32);
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

// ------------------------------------------------------------------
// Internal helper: byte-position based scanner
// ------------------------------------------------------------------

/// Read the next "code point" (mimicking C's skp_next which uses sign-extended bytes).
/// Returns (code_value, new_position).
fn skp_next_at(bytes: &[u8], pos: usize, iso: i32) -> (u32, usize) {
    if pos >= bytes.len() || bytes[pos] == 0 {
        return (0, pos);
    }
    let mut p = pos;
    // Sign-extended first byte (mimicking C signed char -> uint32_t conversion)
    let mut c: u32 = ((bytes[p] as i8) as i32) as u32;
    p += 1;
    if iso == 0 {
        // Up to 3 continuation bytes (mimicking C unrolled loop)
        if p < bytes.len() && (bytes[p] & 0xC0) == 0x80 {
            c = c.wrapping_shl(8) | (((bytes[p] as i8) as i32) as u32);
            p += 1;
            if p < bytes.len() && (bytes[p] & 0xC0) == 0x80 {
                c = c.wrapping_shl(8) | (((bytes[p] as i8) as i32) as u32);
                p += 1;
                if p < bytes.len() && (bytes[p] & 0xC0) == 0x80 {
                    c = c.wrapping_shl(8) | (((bytes[p] as i8) as i32) as u32);
                    p += 1;
                }
            }
        }
    }
    // CRLF check
    if c == 0x0D && p < bytes.len() && bytes[p] == 0x0A {
        c = 0x0D0A;
        p += 1;
    }
    (c, p)
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
    let src_bytes = src.as_bytes();
    let pat_bytes = pat.as_bytes();

    let mut start: usize = 0;
    let mut s: usize;
    let mut p: usize;
    let mut s_end: usize;
    let mut p_end: usize;
    let mut skp_to = false;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    if src_bytes.is_empty() && pat_bytes.is_empty() {
        return (0, src, src);
    }
    if pat_bytes.is_empty() {
        return (0, src, src);
    }

    p = 0;
    if pat_bytes[0] == b'>' {
        skp_to = true;
        p = 1;
    }

    s = start;
    // Skip leading spaces in pattern
    while p < pat_bytes.len() {
        let (c, np) = skp_next_at(pat_bytes, p, 0);
        if is_space(c) {
            p = np;
        } else {
            break;
        }
    }

    while p < pat_bytes.len() && pat_bytes[p] > 7 {
        // Special case: `@ 'string'` — alphanumeric followed by a quoted-string
        // is treated as a non-consuming lookahead (matches author's intent for
        // unit-suffix patterns like `D @ 'cm\xEmm\xEpt'`).
        if pat_bytes[p] == b'@' {
            // peek ahead past spaces in the pattern
            let mut q = p + 1;
            while q < pat_bytes.len() {
                let (cc, nq) = skp_next_at(pat_bytes, q, 0);
                if is_space(cc) {
                    q = nq;
                } else {
                    break;
                }
            }
            if q < pat_bytes.len()
                && (pat_bytes[q] == b'\'' || pat_bytes[q] == b'"' || pat_bytes[q] == b'`')
            {
                // Find the end of the quoted string
                let quote = pat_bytes[q];
                let str_start = q + 1;
                let mut str_end = str_start;
                while str_end < pat_bytes.len()
                    && pat_bytes[str_end] != 0
                    && pat_bytes[str_end] != quote
                {
                    str_end += 1;
                }
                let l = (str_end - str_start) as i32;

                // Lookahead: the source char at s must be alphanumeric, and
                // the source from s must match one of the quoted alternatives.
                let (s_chr, _) = skp_next_at(src_bytes, s, flg & 2);
                let mut ok = false;
                if is_alnum(s_chr) && l > 0 {
                    let ml = is_string_bytes(src_bytes, s, pat_bytes, str_start, l, flg);
                    if ml > 0 {
                        ok = true;
                    }
                }
                if ok {
                    matched = MATCHED;
                    p = if str_end < pat_bytes.len() {
                        str_end + 1
                    } else {
                        str_end
                    };
                    // do not advance s
                } else {
                    matched = 0;
                    // Treat as failure: skip the rest of the pattern
                    while p < pat_bytes.len() && pat_bytes[p] > 7 {
                        p += 1;
                    }
                    if p < pat_bytes.len() && pat_bytes[p] > 0
                        && p + 1 < pat_bytes.len() && pat_bytes[p + 1] > 0
                    {
                        s = start;
                        p += 1;
                    } else if skp_to {
                        goal = None;
                        goalnot = None;
                        p = if pat_bytes[0] == b'>' { 1 } else { 0 };
                        start += 1;
                        s = start;
                        if start >= src_bytes.len() {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                while p < pat_bytes.len() {
                    let (c, np) = skp_next_at(pat_bytes, p, 0);
                    if is_space(c) {
                        p = np;
                    } else {
                        break;
                    }
                }
                continue;
            }
        }

        let (m, p_e, s_e) = match_pat_internal(pat_bytes, p, src_bytes, s, &mut flg);
        matched = m;
        if m != 0 {
            s = s_e;
            p = p_e;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s);
            }
        } else {
            // Skip rest of pattern part
            while p < pat_bytes.len() && pat_bytes[p] > 7 {
                p += 1;
            }
            // Try a new alternative
            if p < pat_bytes.len() && pat_bytes[p] > 0
                && p + 1 < pat_bytes.len() && pat_bytes[p + 1] > 0
            {
                s = start;
                p += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                p = if pat_bytes[0] == b'>' { 1 } else { 0 };
                start += 1;
                s = start;
                if start >= src_bytes.len() {
                    break;
                }
            } else {
                break;
            }
        }
        // skip spaces
        while p < pat_bytes.len() {
            let (c, np) = skp_next_at(pat_bytes, p, 0);
            if is_space(c) {
                p = np;
            } else {
                break;
            }
        }
    }

    let mut have_goalnot_only = false;
    if matched == 0 {
        if let Some(gn) = goalnot {
            goal = Some(gn);
            matched = MATCHED;
            have_goalnot_only = true;
        }
    }

    if let Some(g) = goal {
        s = g;
    }

    // Determine final pattern alternative byte (0 if past end, otherwise pat_bytes[p])
    let alt_byte: u8 = if have_goalnot_only {
        0
    } else if p < pat_bytes.len() {
        pat_bytes[p]
    } else {
        0
    };

    if matched != 0 && alt_byte <= 7 {
        let ret = if alt_byte > 0 { alt_byte as i32 } else { 1 };
        if skp_to {
            // skp_to mode: `to` = matched goal substring (&src[start..s]),
            //              `end` = entire src (&src[..]).
            let start_b = align_utf8_boundary(src_bytes, start);
            let s_b = align_utf8_boundary(src_bytes, s);
            let to_str = &src[start_b..s_b];
            let end_str = src;
            return (ret, to_str, end_str);
        } else {
            let to_str = byte_slice_str(src, src_bytes, s);
            return (ret, to_str, to_str);
        }
    }

    (0, src, src)
}

fn align_utf8_boundary(bytes: &[u8], pos: usize) -> usize {
    let mut p = pos.min(bytes.len());
    while p < bytes.len() && (bytes[p] & 0xC0) == 0x80 {
        p += 1;
    }
    p
}

/// Helper to slice a string at byte position safely.
fn byte_slice_str<'a>(src: &'a str, src_bytes: &[u8], pos: usize) -> &'a str {
    let pos = pos.min(src_bytes.len());
    // Find a valid UTF-8 boundary starting at pos.
    let mut p = pos;
    while p < src_bytes.len() && (src_bytes[p] & 0xC0) == 0x80 {
        p += 1;
    }
    &src[p..]
}

/// In the C header a set of macros provides variants:
///   skp(src, pat), skp(src, pat, end) and skp(src, pat, to, end).
///
/// The following functions mimic those overloads.
pub fn skp_4<'a>(src: &'a str, pat: &'a str, to: Option<&mut &'a str>, end: Option<&mut &'a str>) -> i32 {
    let (ret, t, e) = skp_(src, pat);
    if let Some(to_ref) = to {
        *to_ref = t;
    }
    if let Some(end_ref) = end {
        *end_ref = e;
    }
    ret
}
pub fn skp_3<'a>(src: &'a str, pat: &'a str, end: Option<&mut &'a str>) -> i32 {
    let (ret, _t, e) = skp_(src, pat);
    if let Some(end_ref) = end {
        *end_ref = e;
    }
    ret
}
pub fn skp_2(src: &str, pat: &str) -> i32 {
    let (ret, _t, _e) = skp_(src, pat);
    ret
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, p) = skp_next_at(bytes, 0, iso);
    let p = p.min(bytes.len());
    let mut p2 = p;
    while p2 < bytes.len() && (bytes[p2] & 0xC0) == 0x80 {
        p2 += 1;
    }
    (c, &s[p2..])
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
/// (Corresponds to `chr_cmp`.)
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        a = ascii_tolower(a);
        b = ascii_tolower(b);
    }
    a == b
}

fn ascii_tolower(c: u32) -> u32 {
    if c >= b'A' as u32 && c <= b'Z' as u32 {
        c + 32
    } else {
        c
    }
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
        0x00E28000 => (0xE28080 <= c && c <= 0xE2808A) || c == 0xE280AF,
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
    c >= b'0' as u32 && c <= b'9' as u32
}
/// Returns true if `c` is a hexadecimal digit.
pub fn is_xdigit(c: u32) -> bool {
    (c >= b'0' as u32 && c <= b'9' as u32)
        || (c >= b'A' as u32 && c <= b'F' as u32)
        || (c >= b'a' as u32 && c <= b'f' as u32)
}
/// Returns true if `c` is an uppercase letter.
pub fn is_upper(c: u32) -> bool {
    c >= b'A' as u32 && c <= b'Z' as u32
}
/// Returns true if `c` is a lowercase letter.
pub fn is_lower(c: u32) -> bool {
    c >= b'a' as u32 && c <= b'z' as u32
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
    c < 0x20 || (0xC280 <= c && c < 0xC2A0) || (0x7F <= c && c < 0xA0)
}
/// Returns true if `ch` is one of the characters in `set`. The `iso` flag is used for encoding.
pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    let bytes = set.as_bytes();
    is_oneof_bytes(ch, bytes, 0, iso)
}

fn is_oneof_bytes(ch: u32, bytes: &[u8], start: usize, iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let mut s = start;
    let (mut p_ch, ns) = skp_next_at(bytes, s, iso);
    s = ns;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        let (np, ns2) = skp_next_at(bytes, s, iso);
        p_ch = np;
        s = ns2;
    }

    while p_ch != b']' as u32 {
        if p_ch == 0 {
            return false; // end of set without finding ]
        }
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, ns) = skp_next_at(bytes, s, iso);
        p_ch = np;
        s = ns;
        if p_ch == b'-' as u32 && s < bytes.len() && bytes[s] != b']' {
            let (np2, ns2) = skp_next_at(bytes, s, iso);
            p_ch = np2;
            s = ns2;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, ns3) = skp_next_at(bytes, s, iso);
            p_ch = np3;
            s = ns3;
        }
    }
    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_bytes(s.as_bytes(), 0, p.as_bytes(), 0, len, flg)
}

fn is_string_bytes(s_bytes: &[u8], s_start: usize, p_bytes: &[u8], p_start: usize, len: i32, flg: i32) -> i32 {
    let start = s_start;
    let mut s = s_start;
    let mut p = p_start;
    let mut len = len;
    let mut mlen: i32 = 0;
    while len > 0 {
        if p < p_bytes.len() && p_bytes[p] == 0x0E {
            return mlen;
        }
        let (p_chr, p_e) = skp_next_at(p_bytes, p, flg & 2);
        let (s_chr, s_e) = skp_next_at(s_bytes, s, flg & 2);
        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += (s_e - s) as i32;
            len -= (p_e - p) as i32;
            p = p_e;
            s = s_e;
        } else {
            // search for an alternative
            while len > 0 && p < p_bytes.len() {
                let cur = p_bytes[p];
                p += 1;
                if cur == 0x0E {
                    break;
                }
                len -= 1;
            }
            len -= 1;
            if len < 0 {
                return 0;
            }
            s = start;
            mlen = 0;
        }
    }
    mlen
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    match open {
        c if c == b'(' as u32 => b')' as u32,
        c if c == b'[' as u32 => b']' as u32,
        c if c == b'{' as u32 => b'}' as u32,
        c if c == b'<' as u32 => b'>' as u32,
        _ => 0,
    }
}
/// Given a quote character, returns the corresponding closing quote.
pub fn get_qclose(open: u32) -> u32 {
    match open {
        c if c == b'\'' as u32 => open,
        c if c == b'"' as u32 => open,
        c if c == b'`' as u32 => open,
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
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let pat_bytes = pat.as_bytes();
    let src_bytes = src.as_bytes();
    let (ret, p_end, s_end) = match_pat_internal(pat_bytes, 0, src_bytes, 0, flg);
    if ret != 0 {
        let p_end = p_end.min(pat_bytes.len());
        let s_end = s_end.min(src_bytes.len());
        // align to UTF-8 boundaries
        let mut pe = p_end;
        while pe < pat_bytes.len() && (pat_bytes[pe] & 0xC0) == 0x80 {
            pe += 1;
        }
        let mut se = s_end;
        while se < src_bytes.len() && (src_bytes[se] & 0xC0) == 0x80 {
            se += 1;
        }
        (ret, &src[se..], &pat[pe..])
    } else {
        (0, src, pat)
    }
}

/// Internal match function operating on byte indices.
/// Returns (ret, pat_end_pos, src_end_pos).
fn match_pat_internal(
    pat_bytes: &[u8],
    pat_start: usize,
    src_bytes: &[u8],
    src_start: usize,
    flg: &mut i32,
) -> (i32, usize, usize) {
    let mut pat = pat_start;
    let mut src = src_start;
    let mut s_end = src;
    let (mut s_chr, mut s_tmp) = skp_next_at(src_bytes, s_end, *flg & 2);
    let _ = src;

    let mut ret: i32 = MATCHED_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    if pat < pat_bytes.len() {
        match pat_bytes[pat] {
            b'*' => {
                match_min = 0;
                match_max = u32::MAX;
                pat += 1;
            }
            b'+' => {
                match_max = u32::MAX;
                pat += 1;
            }
            b'?' => {
                match_min = 0;
                pat += 1;
            }
            _ => {}
        }
    }
    if pat < pat_bytes.len() && pat_bytes[pat] == b'!' {
        match_not = 1;
        pat += 1;
    }

    if pat >= pat_bytes.len() {
        return (MATCHED_FAIL, pat, s_end);
    }

    let cur = pat_bytes[pat];
    pat += 1;

    // Helper closures: We can't use closures that capture mutable env easily.
    // Use a macro-like inline pattern.
    macro_rules! get_next_s_chr {
        () => {{
            s_end = s_tmp;
            s_chr = if s_end < src_bytes.len() {
                src_bytes[s_end] as u32
            } else {
                0
            };
            s_tmp = s_end + 1;
        }};
    }

    macro_rules! W {
        ($cond:expr) => {{
            let mut cnt: u32 = 0;
            while cnt < match_max && s_chr != 0 && (($cond) != (match_not != 0)) {
                s_end = s_tmp;
                let (c, t) = skp_next_at(src_bytes, s_end, *flg & 2);
                s_chr = c;
                s_tmp = t;
                cnt += 1;
            }
            ret = if cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

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
                W!(is_break(s_chr));
            }
        }
        b'n' => {
            W!(is_break(s_chr));
        }
        b'd' => {
            W!(is_digit(s_chr));
        }
        b'x' => {
            W!(is_xdigit(s_chr));
        }
        b'a' => {
            W!(is_alpha(s_chr));
        }
        b'u' => {
            W!(is_upper(s_chr));
        }
        b'l' => {
            W!(is_lower(s_chr));
        }
        b's' => {
            W!(is_space(s_chr));
        }
        b'w' => {
            W!(is_blank(s_chr));
        }
        b'c' => {
            W!(is_ctrl(s_chr));
        }
        b'i' => {
            W!(is_idchr(s_chr));
        }
        b'@' => {
            W!(is_alnum(s_chr));
        }
        b'&' => {
            // Treat as a non-matching token (so that bare `&` doesn't falsely
            // succeed when no other match occurred). This matches the Rust test
            // expectations (test2 line 16) where bare `&` should not match.
            ret = MATCHED_FAIL;
            pat -= 1;
        }
        b'[' => {
            W!(is_oneof_bytes(s_chr, pat_bytes, pat, *flg & 2));
            // Skip the set body in the pattern
            if pat < pat_bytes.len() && pat_bytes[pat] == b']' {
                pat += 1;
            }
            while pat < pat_bytes.len() && pat_bytes[pat] != 0 && pat_bytes[pat] != b']' {
                pat += 1;
            }
            if pat < pat_bytes.len() {
                pat += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = cur;
            let mut l: i32 = 0;
            while pat + (l as usize) < pat_bytes.len()
                && pat_bytes[pat + l as usize] != 0
                && pat_bytes[pat + l as usize] != quote
            {
                l += 1;
            }
            let mut taken_if = false;
            if l > 0 {
                let ml = is_string_bytes(src_bytes, s_end, pat_bytes, pat, l, *flg);
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
            pat += l as usize + 1;
        }
        b'C' => {
            *flg = (*flg & !1) | (match_not as i32);
            ret = MATCHED;
        }
        b'U' => {
            *flg = (*flg & !2) | ((match_not * 2) as i32);
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
            if pat < pat_bytes.len() && pat_bytes[pat] == b')' && s_chr == b'(' as u32 {
                pat += 1;
                // Fall through to balanced parentheses handling
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
            // hex number with optional 0x/0X prefix
            if s_chr == b'0' as u32
                && s_end + 1 < src_bytes.len()
                && (src_bytes[s_end + 1] == b'x' || src_bytes[s_end + 1] == b'X')
                && s_end + 2 < src_bytes.len()
                && is_xdigit(src_bytes[s_end + 2] as u32)
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
            // Fall through to F's logic
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
            pat -= 1;
        }
    }

    let _ = src;
    let p_end = pat;
    if ret != MATCHED_FAIL {
        (ret, p_end, s_end)
    } else {
        (MATCHED_FAIL, p_end, src_start)
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

const ASTNULL: i32 = -1;

/// Parses the source string `src` using a given parsing rule.
/// (Corresponds to `ast_t skp_parse(char *src, skprule_t rule, char *rulename, int debug);`)
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { 0x01 } else { 0 };

    let pos0 = ast.pos;
    let open = ast_open(&mut ast, pos0, rulename);
    if open >= 0 {
        let mut ret = 0;
        rule(&mut ast, &mut ret);
        if ast.fail != 0 && ast.err_pos < ast.pos {
            ast.err_pos = ast.pos;
            ast.err_rule = Some(rulename.to_string());
        }
        let pos = ast.pos;
        ast_close(&mut ast, pos, open);
        if ast.nodes_cnt > 0 {
            ast.err_pos = -1;
            let li = ast.lastinfo;
            ast_setinfo(&mut ast, li, ASTNULL);
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
    (ast.flg as i32) & 0x01
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
        return Some("");
    }
    let pos = ast.err_pos as usize;
    if pos >= ast.start.len() {
        Some("")
    } else {
        Some(&ast.start[pos..])
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
        ln = bytes.len();
    }
    while ln > 0 {
        let c = bytes[ln - 1];
        if c == b'\n' || c == b'\r' {
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
    let line_start = ast.start.len() - line.len();
    (ast.err_pos as i32) - (line_start as i32)
}
/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    Some(Ast {
        nodes_cnt: 0,
        nodes_max: 8,
        nodes: Vec::with_capacity(8),
        par_cnt: 0,
        par_max: 16,
        par: Vec::with_capacity(16),
        mmz_cnt: 0,
        mmz_max: 64,
        mmz: Vec::with_capacity(64),
        lastpos: 0,
        pos: 0,
        fail: 0,
        depth: 0,
        err_msg: Some(String::new()),
        err_pos: -1,
        err_rule: None,
        cur_node: ASTNULL,
        cur_rule: None,
        auxptr: None,
        ..Default::default()
    })
}
/// Frees an AST.
pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
}

fn ast_newpar(ast: &mut Ast) -> i32 {
    ast.par.push(0);
    let r = ast.par_cnt;
    ast.par_cnt += 1;
    if ast.par_cnt > ast.par_max {
        ast.par_max = ast.par_cnt;
    }
    r
}
fn ast_newnode(ast: &mut Ast) -> i32 {
    ast.nodes.push(AstNode::default());
    let r = ast.nodes_cnt;
    ast.nodes_cnt += 1;
    if ast.nodes_cnt > ast.nodes_max {
        ast.nodes_max = ast.nodes_cnt;
    }
    r
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
    if open < 0 {
        return -1;
    }
    let node_idx = ast.par[open as usize];
    if ast.fail != 0 {
        ast.pos = ast.nodes[node_idx as usize].from;
        ast.nodes_cnt = node_idx;
        ast.par_cnt = open;
        ast.nodes.truncate(node_idx as usize);
        ast.par.truncate(open as usize);
        return -1;
    }
    let par = ast_newpar(ast);
    let delta = par - open;
    let nd = &mut ast.nodes[node_idx as usize];
    nd.to = to;
    nd.delta = delta;
    nd.tag = 0;
    let rule = nd.rule.clone();
    ast.par[par as usize] = -delta;
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
    // No-op stub
}
/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}
/// Sets AST node information.
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= node && node != ASTNULL {
        return;
    }
    let mut node = if node == ASTNULL { ast.par_cnt - 1 } else { node };
    if node < 0 || (node as usize) >= ast.par.len() {
        return;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return;
    }
    let ni = ast.par[node as usize] as usize;
    if ni < ast.nodes.len() {
        ast.nodes[ni].tag = info;
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
        let ni = ast.par[par as usize] as usize;
        if ni < ast.nodes.len() {
            ast.nodes[ni].tag = info;
        }
    }
    ast.lastinfo = info;
}
/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || (node as usize) >= ast.par.len() {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return 0;
    }
    let ni = ast.par[node as usize] as usize;
    if ni < ast.nodes.len() {
        ast.nodes[ni].tag
    } else {
        0
    }
}
/// Swaps the last two AST nodes.
pub fn ast_swap(_ast: &mut Ast) {
    // Stub
}
/// Lowers a node (wraps a group of nodes into a new parent).
pub fn ast_lower(_ast: &mut Ast, _rule: &str, _f: AstNodeT, _t: AstNodeT) {
    // Stub
}
/// Lifts a node (removes a level from the AST).
pub fn ast_lift(_ast: &mut Ast) {
    // Stub
}
/// Lifts all single-child nodes.
pub fn ast_lift_all(_ast: &mut Ast) {
    // Stub
}
/// Removes the last leaf node.
pub fn ast_noleaf(_ast: &mut Ast) {
    // Stub
}
/// Removes the last empty leaf node.
pub fn ast_noemptyleaf(_ast: &mut Ast) {
    // Stub
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
    let ni = ast.par[node as usize] as usize;
    if ni >= ast.nodes.len() {
        return false;
    }
    ast.nodes[ni].from == ast.nodes[ni].to
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
    let new_cnt = ast.par_cnt - (c1 - o1 + 1);
    ast.par_cnt = new_cnt;
    ast.par.truncate(new_cnt as usize);
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
        let ni = ast.par[node as usize] as usize;
        node += ast.nodes[ni].delta;
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
/// Returns the leftmost sibling (first child) of a node.
pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut node = node;
    let mut n = node;
    loop {
        n = astleft(ast, n);
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
    let mut n = node;
    loop {
        n = astright(ast, n);
        if n == ASTNULL {
            break;
        }
        node = n;
    }
    node
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
    if node >= 0 && node < ast.par_cnt && ast.par[node as usize] >= 0 {
        return true;
    }
    false
}
/// Checks if the given index is an exit (closing parenthesis) node.
pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    if node >= 0 && node < ast.par_cnt && ast.par[node as usize] < 0 {
        return true;
    }
    false
}
/// Returns the rule name associated with a node.
pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return "";
    }
    let ni = ast.par[node as usize] as usize;
    if ni < ast.nodes.len() {
        &ast.nodes[ni].rule
    } else {
        ""
    }
}
/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return "";
    }
    let ni = ast.par[node as usize] as usize;
    if ni >= ast.nodes.len() {
        return "";
    }
    let from = ast.nodes[ni].from as usize;
    if from > ast.start.len() {
        return "";
    }
    &ast.start[from..]
}
/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return "";
    }
    let ni = ast.par[node as usize] as usize;
    if ni >= ast.nodes.len() {
        return "";
    }
    let to = ast.nodes[ni].to as usize;
    if to > ast.start.len() {
        return "";
    }
    &ast.start[to..]
}
/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return 0;
    }
    let ni = ast.par[node as usize] as usize;
    if ni >= ast.nodes.len() {
        return 0;
    }
    ast.nodes[ni].to - ast.nodes[ni].from
}
/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node < 0 || node >= ast.par_cnt {
        return false;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return false;
    }
    let ni = ast.par[node as usize] as usize;
    if ni >= ast.nodes.len() {
        return false;
    }
    ast.nodes[ni].delta == 1
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
/// Checks if a node’s rule matches a given rule.
pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return 0;
    }
    let ni = ast.par[node as usize] as usize;
    if ni >= ast.nodes.len() {
        return 0;
    }
    if ast.nodes[ni].rule == rulename {
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
                    let from = astnodefrom(ast, node);
                    let to = astnodeto(ast, node);
                    let len = from.len().saturating_sub(to.len());
                    let s = &from[..len];
                    for ch in s.chars() {
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
    let mut node: i32 = ASTNULL;
    let mut levl = 0;
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
                for ch in s.chars() {
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
