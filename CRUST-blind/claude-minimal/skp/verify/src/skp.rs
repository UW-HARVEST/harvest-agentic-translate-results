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
    // In the C code this is `to - start` (pointer arithmetic) clamped to [0, 65536].
    // We approximate with byte length difference of the two slices.
    let ret = start.len() as i32 - to.len() as i32;
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

// ----------------------------------------------------------------------------
// Helper: byte-level next code point (mirrors the C `skp_next`).
// Returns (code_point, bytes_consumed).
// ----------------------------------------------------------------------------
fn skp_next_bytes(s: &[u8], iso: i32) -> (u32, usize) {
    if s.is_empty() {
        return (0, 0);
    }
    let mut c: u32 = s[0] as u32;
    let mut i: usize = 1;
    if iso == 0 {
        // Treat as UTF-8 — collect continuation bytes (up to 3) into c.
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
    // Combine CRLF into 0x0D0A as in the C version.
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
    // We must return the rest as a &str — only safe if `n` lands on a UTF-8 boundary.
    if n == 0 || n > s.len() {
        return (c, s);
    }
    if s.is_char_boundary(n) {
        (c, &s[n..])
    } else {
        // Fall back: skip a single byte (still safe via char iteration).
        let mut chars = s.chars();
        let nc = chars.next().map(|ch| ch as u32).unwrap_or(0);
        (nc, chars.as_str())
    }
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
/// (Corresponds to `chr_cmp`.)
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
    c == 0x0D0A      // CRLF (combined)
        || c == 0xC285   // U+0085 NEL
        || c == 0xE280A8 // U+2028 LS
        || c == 0xE280A9 // U+2029 PS
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
    let (mut p_ch, mut idx) = skp_next_bytes(bytes, iso);
    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        let (np, n) = skp_next_bytes(&bytes[idx..], iso);
        p_ch = np;
        idx += n;
    }
    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, n) = skp_next_bytes(&bytes[idx..], iso);
        p_ch = np;
        idx += n;
        if p_ch == b'-' as u32 && idx < bytes.len() && bytes[idx] != b']' {
            let (np2, n2) = skp_next_bytes(&bytes[idx..], iso);
            p_ch = np2;
            idx += n2;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, n3) = skp_next_bytes(&bytes[idx..], iso);
            p_ch = np3;
            idx += n3;
        }
    }
    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    let s_bytes = s.as_bytes();
    let p_bytes = p.as_bytes();
    let start_s = s_bytes;
    let mut s_off: usize = 0;
    let mut p_off: usize = 0;
    let mut len = len;
    let mut mlen: i32 = 0;
    while len > 0 {
        if p_off >= p_bytes.len() {
            return mlen;
        }
        if p_bytes[p_off] == 0x0E {
            return mlen;
        }
        let (p_chr, p_n) = skp_next_bytes(&p_bytes[p_off..], flg & 2);
        let (s_chr, s_n) = skp_next_bytes(&s_bytes[s_off..], flg & 2);
        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += s_n as i32;
            len -= p_n as i32;
            p_off += p_n;
            s_off += s_n;
        } else {
            // Search for an alternative separator (0x0E)
            while len > 0 && p_off < p_bytes.len() {
                let b = p_bytes[p_off];
                p_off += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
            len -= 1;
            if len < 0 {
                return 0;
            }
            s_off = 0;
            mlen = 0;
            // ensure start_s remains valid
            let _ = start_s;
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
        c if c == b'\'' as u32 || c == b'"' as u32 || c == b'`' as u32 => open,
        _ => 0,
    }
}

/// Constants for match results.
pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

// ---------------------------------------------------------------------------
// Internal byte-oriented helpers used by `match_pat` and `skp_`.
// They operate on byte offsets to faithfully mirror the C code.
// ---------------------------------------------------------------------------

fn match_bytes(
    pat: &[u8],
    pat_off0: usize,
    src: &[u8],
    src_off0: usize,
    flg: &mut i32,
) -> (i32, usize, usize) {
    // Returns (match_result, src_end_off, pat_end_off) where pat_end_off is set
    // only when match_result != MATCHED_FAIL.
    let mut p_off = pat_off0;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_cnt: u32;
    let mut match_not: u32 = 0;
    let mut intnumber = false;
    let mut s_end = src_off0;
    let mut s_tmp = src_off0;
    let (mut s_chr, n0) = skp_next_bytes(&src[s_end..], *flg & 2);
    s_tmp = s_end + n0;
    let mut ret: i32 = MATCHED_FAIL;

    if p_off < pat.len() && pat[p_off] == b'*' {
        match_min = 0;
        match_max = u32::MAX;
        p_off += 1;
    } else if p_off < pat.len() && pat[p_off] == b'+' {
        match_max = u32::MAX;
        p_off += 1;
    } else if p_off < pat.len() && pat[p_off] == b'?' {
        match_min = 0;
        p_off += 1;
    }

    if p_off < pat.len() && pat[p_off] == b'!' {
        match_not = 1;
        p_off += 1;
    }

    // W(x) macro: loop while count < max && (s_chr && (!!x != match_not))
    macro_rules! w {
        ($cond:expr) => {{
            match_cnt = 0;
            while match_cnt < match_max && s_chr != 0 {
                let c = $cond;
                let cb: u32 = if c { 1 } else { 0 };
                if cb != match_not {
                    s_end = s_tmp;
                    let (nc, nn) = skp_next_bytes(&src[s_end..], *flg & 2);
                    s_chr = nc;
                    s_tmp = s_end + nn;
                    match_cnt += 1;
                } else {
                    break;
                }
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    macro_rules! get_next_s_chr {
        () => {{
            s_end = s_tmp;
            // C: s_chr = *s_end ; s_tmp++;  (single-byte)
            s_chr = if s_end < src.len() { src[s_end] as u32 } else { 0 };
            s_tmp = s_end + 1;
        }};
    }

    if p_off >= pat.len() {
        return (MATCHED_FAIL, s_end, p_off);
    }
    let c = pat[p_off];
    p_off += 1;

    match c {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { 1 } else { MATCHED_FAIL };
            } else {
                w!(s_chr != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = 1;
            } else {
                w!(is_break(s_chr));
            }
        }
        b'n' => w!(is_break(s_chr)),
        b'd' => w!(is_digit(s_chr)),
        b'x' => w!(is_xdigit(s_chr)),
        b'a' => w!(is_alpha(s_chr)),
        b'u' => w!(is_upper(s_chr)),
        b'l' => w!(is_lower(s_chr)),
        b's' => w!(is_space(s_chr)),
        b'w' => w!(is_blank(s_chr)),
        b'c' => w!(is_ctrl(s_chr)),
        b'i' => w!(is_idchr(s_chr)),
        b'@' => w!(is_alnum(s_chr)),
        b'&' => {
            ret = if match_not != 0 { MATCHED_GOALNOT } else { MATCHED_GOAL };
        }
        b'[' => {
            // Build a slice of pattern bytes starting at p_off as a &str (best effort).
            let set_bytes = &pat[p_off..];
            let set_str = std::str::from_utf8(set_bytes).unwrap_or("");
            w!(is_oneof(s_chr, set_str, *flg & 2));
            if p_off < pat.len() && pat[p_off] == b']' {
                p_off += 1;
            }
            while p_off < pat.len() && pat[p_off] != b']' {
                p_off += 1;
            }
            if p_off < pat.len() {
                p_off += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = c;
            let mut l: i32 = 0;
            while p_off + (l as usize) < pat.len() && pat[p_off + l as usize] != quote {
                l += 1;
            }
            let slice_str = std::str::from_utf8(
                &pat[p_off..p_off + l as usize],
            )
            .unwrap_or("");
            let s_end_str = std::str::from_utf8(&src[s_end..]).unwrap_or("");
            let ml = if l > 0 {
                is_string(s_end_str, slice_str, l, *flg)
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
            p_off += (l as usize) + 1;
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
            if !(p_off < pat.len() && pat[p_off] == b')' && s_chr == b'(' as u32) {
                // fall through to default fail
                ret = MATCHED_FAIL;
            } else {
                p_off += 1;
                // Fall through to 'B' logic
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
            // fall through to F
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
            if intnumber {
                // done
            } else {
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
            p_off -= 1;
        }
    }

    let p_end = p_off;
    if ret != MATCHED_FAIL {
        (ret, s_end, p_end)
    } else {
        (ret, s_end, p_end)
    }
}

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`. The flag parameter is passed by mutable reference.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let pb = pat.as_bytes();
    let sb = src.as_bytes();
    let (ret, s_end, p_end) = match_bytes(pb, 0, sb, 0, flg);
    let src_end = if s_end <= sb.len() && src.is_char_boundary(s_end) {
        &src[s_end..]
    } else {
        src
    };
    let pat_end = if p_end <= pb.len() && pat.is_char_boundary(p_end) {
        &pat[p_end..]
    } else {
        pat
    };
    (ret, src_end, pat_end)
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
    let pb = pat.as_bytes();
    let sb = src.as_bytes();
    let mut start: usize = 0;
    let mut s: usize = 0;
    let mut p: usize = 0;
    let mut skp_to = false;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    if pb.is_empty() || sb.is_empty() {
        return (0, src, src);
    }

    if p < pb.len() && pb[p] == b'>' {
        skp_to = true;
        p += 1;
    }

    while p < pb.len() && is_space(pb[p] as u32) {
        p += 1;
    }

    while p < pb.len() && pb[p] > 0x07 {
        let (m, s_end, p_end) = match_bytes(pb, p, sb, s, &mut flg);
        if m != 0 {
            matched = m;
            s = s_end;
            p = p_end;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s);
            }
        } else {
            // skip past current pattern characters > 0x07
            while p < pb.len() && pb[p] > 0x07 {
                p += 1;
            }
            if p < pb.len() && pb[p] > 0x00 && p + 1 < pb.len() && pb[p + 1] > 0x00 {
                s = start;
                p += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                p = if pb.first() == Some(&b'>') { 1 } else { 0 };
                start += 1;
                s = start;
                if start >= sb.len() {
                    break;
                }
            } else {
                break;
            }
        }
        while p < pb.len() && is_space(pb[p] as u32) {
            p += 1;
        }
    }

    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        p = pb.len(); // Force the "end of pattern" check below to treat *p as 0
    }

    if let Some(g) = goal {
        s = g;
    }

    let pat_terminal = p >= pb.len() || pb[p] <= 0x07;
    if matched != 0 && pat_terminal {
        let ret = if p < pb.len() && pb[p] > 0 { pb[p] as i32 } else { 1 };
        let to_off = if skp_to { start } else { s };
        let end_off = s;
        let to_str = if src.is_char_boundary(to_off) {
            &src[to_off..]
        } else {
            src
        };
        let end_str = if src.is_char_boundary(end_off) {
            &src[end_off..]
        } else {
            src
        };
        return (ret, to_str, end_str);
    }

    (0, src, src)
}

/// In the C header a set of macros provides variants:
///   skp(src, pat), skp(src, pat, end) and skp(src, pat, to, end).
///
/// The following functions mimic those overloads.
pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    let (ret, t, e) = skp_(src, pat);
    if let Some(slot) = to {
        // Need a 'static-like reference; we cheat by transmuting lifetimes via pointer write.
        // SAFETY: caller's `to` ref points into the same source they passed in.
        unsafe {
            let p = slot as *mut &str;
            *p = std::mem::transmute::<&str, &str>(t);
        }
    }
    if let Some(slot) = end {
        unsafe {
            let p = slot as *mut &str;
            *p = std::mem::transmute::<&str, &str>(e);
        }
    }
    ret
}
pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    skp_4(src, pat, None, end)
}
pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_4(src, pat, None, None)
}

// ---------------------------------------------------------------------------
//
// # AST Parsing Functions and Types
//
// ---------------------------------------------------------------------------
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

/// In C, the AST "memory zone" structure:
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

/// Parses the source string `src` using a given parsing rule.
/// (Corresponds to `ast_t skp_parse(char *src, skprule_t rule, char *rulename, int debug);`)
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { 0x01 } else { 0 };

    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
    if open >= 0 {
        let mut ret_val = ast.ret;
        rule(&mut ast, &mut ret_val);
        ast.ret = ret_val;

        if ast.fail != 0 && ast.err_pos < ast.pos {
            ast.err_pos = ast.pos;
            ast.err_rule = Some(rulename.to_string());
        }

        let cur_pos = ast.pos;
        ast_close(&mut ast, cur_pos, open);

        if ast.nodes_cnt > 0 {
            ast.err_pos = -1;
            let info = ast.lastinfo;
            ast_setinfo(&mut ast, info, -1);
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
    if pos > ast.start.len() || !ast.start.is_char_boundary(pos) {
        return Some("");
    }
    Some(&ast.start[pos..])
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
    if ast.start.is_char_boundary(ln) {
        &ast.start[ln..]
    } else {
        ""
    }
}

/// Returns the error column number.
pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let total_remaining = ast.start.len() as i32 - ast.err_pos;
    (line.len() as i32) - total_remaining
}

/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    let mut ast = Ast::default();
    ast.nodes_max = 8;
    ast.par_max = 16;
    ast.mmz_max = 64;
    ast.lastpos = 0;
    ast.pos = 0;
    ast.fail = 0;
    ast.depth = 0;
    ast.err_msg = None;
    ast.err_pos = -1;
    ast.err_rule = None;
    ast.cur_node = -1;
    ast.cur_rule = None;
    ast.auxptr = None;
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
    let par = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;
    {
        let nd = &mut ast.nodes[node_idx as usize];
        nd.to = to;
        nd.delta = par - open;
        nd.tag = 0;
    }
    let delta = ast.nodes[node_idx as usize].delta;
    ast.par[par as usize] = -delta;
    ast.cur_node = par;
    ast.cur_rule = Some(ast.nodes[node_idx as usize].rule.clone());
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
}

/// Records memoization of AST nodes (for left recursion etc.).
pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // No-op stub: full memoization is non-trivial to port faithfully.
}

/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

/// Sets AST node information.
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= node {
        return;
    }
    let mut node = node;
    if node == -1 {
        node = ast.par_cnt - 1;
    }
    if node < 0 {
        return;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return;
    }
    let n_idx = ast.par[node as usize];
    if (n_idx as usize) < ast.nodes.len() {
        ast.nodes[n_idx as usize].tag = info;
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
    if par >= 0 {
        let n_idx = ast.par[par as usize];
        if (n_idx as usize) < ast.nodes.len() {
            ast.nodes[n_idx as usize].tag = info;
        }
    }
    ast.lastinfo = info;
}

/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return 0;
    }
    let n_idx = ast.par[node as usize];
    if (n_idx as usize) >= ast.nodes.len() {
        return 0;
    }
    ast.nodes[n_idx as usize].tag
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
    let o2u = o2 as usize;
    let o1u = o1 as usize;
    let c1u = c1 as usize;
    let c2u = c2 as usize;
    let tmp: Vec<i32> = ast.par[o2u..=c2u].to_vec();
    let group2: Vec<i32> = ast.par[o1u..=c1u].to_vec();
    let mut combined = group2;
    combined.extend(tmp);
    for (i, v) in combined.into_iter().enumerate() {
        ast.par[o2u + i] = v;
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
    if lft < 0 || rgt < 0 {
        return;
    }
    let from_node = ast.par[lft as usize];
    let to_node = ast.par[rgt as usize];
    if from_node < 0 || to_node < 0 {
        return;
    }
    let node_from = ast.nodes[from_node as usize].from;
    let node_to = ast.nodes[to_node as usize].to;
    rgt += ast.nodes[to_node as usize].delta;

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
    // Add two new par slots
    ast.par.push(0);
    ast.par.push(0);
    ast.par_cnt += 2;

    let par_cnt = ast.par_cnt;
    let lftu = lft as usize;
    let rgtu = rgt as usize;

    // Move stuff after rgt over by 2
    if (par_cnt - 1 - rgt) > 2 {
        let move_len = (par_cnt - 1 - rgt - 2) as usize;
        for i in (0..move_len).rev() {
            ast.par[rgtu + 3 + i] = ast.par[rgtu + 1 + i];
        }
    }

    // Move (lft..=rgt) over by 1
    let block_len = (rgt - lft + 1) as usize;
    for i in (0..block_len).rev() {
        ast.par[lftu + 1 + i] = ast.par[lftu + i];
    }

    ast.par[lftu] = node;
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
    let n_idx = ast.par[o1 as usize] as usize;
    if ast.nodes[n_idx].tag == 0 {
        let move_len = (c2 - o2 + 1) as usize;
        for i in 0..move_len {
            ast.par[o1 as usize + i] = ast.par[o2 as usize + i];
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
    let n_idx = ast.par[o1 as usize] as usize;
    if ast.nodes[n_idx].from != ast.nodes[n_idx].to {
        return;
    }
    ast.par_cnt -= 2;
    ast.par.truncate(ast.par_cnt as usize);
}

/// Returns the index of the last AST node.
pub fn ast_lastnode(ast: &Ast) -> AstNodeT {
    if ast.fail != 0 || ast.par_cnt < 2 {
        return -1;
    }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 {
        return -1;
    }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 {
        return -1;
    }
    o1
}

/// Checks if the last node is empty.
pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == -1 {
        return false;
    }
    let n_idx = ast.par[node as usize] as usize;
    let nd = &ast.nodes[n_idx];
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
    ast.par.truncate(ast.par_cnt as usize);
}

/// Returns the "left" sibling of a node.
pub fn astleft(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return -1;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    node -= 1;
    if node <= 0 || ast.par[node as usize] >= 0 {
        return -1;
    }
    node + ast.par[node as usize]
}

/// Returns the "right" sibling of a node.
pub fn astright(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return -1;
    }
    let mut node = node;
    if ast.par[node as usize] > 0 {
        let n_idx = ast.par[node as usize] as usize;
        node += ast.nodes[n_idx].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 {
        return -1;
    }
    node
}

/// Returns the parent of a node.
pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut node = astfirst(ast, node);
    if node == -1 {
        return -1;
    }
    node -= 1;
    if node < 0 || ast.par[node as usize] < 0 {
        return -1;
    }
    node
}

/// Returns the first child of a node.
pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return -1;
    }
    let n = node + 1;
    if n >= ast.par_cnt || ast.par[n as usize] < 0 {
        return -1;
    }
    n
}

/// Returns the leftmost sibling (first child) of a node.
pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return -1;
    }
    let mut node = node;
    let mut n = astleft(ast, node);
    while n != -1 {
        node = n;
        n = astleft(ast, node);
    }
    node
}

/// Returns the rightmost sibling of a node.
pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return -1;
    }
    let mut node = node;
    let mut n = astright(ast, node);
    while n != -1 {
        node = n;
        n = astright(ast, node);
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
        return -1;
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
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return "";
    }
    let n_idx = ast.par[node as usize] as usize;
    if n_idx >= ast.nodes.len() {
        return "";
    }
    &ast.nodes[n_idx].rule
}

/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return "";
    }
    let n_idx = ast.par[node as usize] as usize;
    if n_idx >= ast.nodes.len() {
        return "";
    }
    let from = ast.nodes[n_idx].from as usize;
    if from > ast.start.len() || !ast.start.is_char_boundary(from) {
        return "";
    }
    &ast.start[from..]
}

/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return "";
    }
    let n_idx = ast.par[node as usize] as usize;
    if n_idx >= ast.nodes.len() {
        return "";
    }
    let to = ast.nodes[n_idx].to as usize;
    if to > ast.start.len() || !ast.start.is_char_boundary(to) {
        return "";
    }
    &ast.start[to..]
}

/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return 0;
    }
    let n_idx = ast.par[node as usize] as usize;
    if n_idx >= ast.nodes.len() {
        return 0;
    }
    ast.nodes[n_idx].to - ast.nodes[n_idx].from
}

/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 {
        return false;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return false;
    }
    let n_idx = ast.par[node as usize] as usize;
    if n_idx >= ast.nodes.len() {
        return false;
    }
    ast.nodes[n_idx].delta == 1
}

/// Returns the next node in the AST (wrapper for astnextdf).
pub fn astnext(ast: &Ast, node: AstNodeT) -> AstNodeT {
    astnextdf(ast, node)
}

/// Returns a match code if the node's rule is one of several provided.
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

/// Checks if a node's rule matches a given rule.
pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == -1 || node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return 0;
    }
    let n_idx = ast.par[node as usize] as usize;
    if n_idx >= ast.nodes.len() {
        return 0;
    }
    if ast.nodes[n_idx].rule == rulename {
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
    let mut node: AstNodeT = -1;
    loop {
        node = astnextdf(ast, node);
        if node == -1 {
            break;
        }
        if astisnodeentry(ast, node) {
            let _ = write!(f, "({} ", astnoderule(ast, node));
            if astisleaf(ast, node) {
                let _ = write!(f, "'");
                let rule = astnoderule(ast, node);
                if rule == "#" {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from_pos = {
                        let mut n = node;
                        if ast.par[n as usize] < 0 {
                            n += ast.par[n as usize];
                        }
                        let n_idx = ast.par[n as usize] as usize;
                        ast.nodes[n_idx].from as usize
                    };
                    let to_pos = {
                        let mut n = node;
                        if ast.par[n as usize] < 0 {
                            n += ast.par[n as usize];
                        }
                        let n_idx = ast.par[n as usize] as usize;
                        ast.nodes[n_idx].to as usize
                    };
                    let bytes = ast.start.as_bytes();
                    if from_pos <= to_pos && to_pos <= bytes.len() {
                        for &b in &bytes[from_pos..to_pos] {
                            if b == b'\'' {
                                let _ = write!(f, "\\");
                            }
                            let _ = f.write_all(&[b]);
                        }
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
    let mut node: AstNodeT = -1;
    let mut levl: i32 = 0;
    loop {
        node = astnextdf(ast, node);
        if node == -1 {
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
                let from_pos = {
                    let mut n = node;
                    if ast.par[n as usize] < 0 {
                        n += ast.par[n as usize];
                    }
                    let n_idx = ast.par[n as usize] as usize;
                    ast.nodes[n_idx].from as usize
                };
                let to_pos = {
                    let mut n = node;
                    if ast.par[n as usize] < 0 {
                        n += ast.par[n as usize];
                    }
                    let n_idx = ast.par[n as usize] as usize;
                    ast.nodes[n_idx].to as usize
                };
                let bytes = ast.start.as_bytes();
                if from_pos <= to_pos && to_pos <= bytes.len() {
                    for &b in &bytes[from_pos..to_pos] {
                        if b == b'\'' {
                            let _ = write!(f, "\\");
                        }
                        let _ = f.write_all(&[b]);
                    }
                }
                let _ = write!(f, "'");
            }
            let _ = writeln!(f);
        } else {
            levl -= 4;
        }
    }
}
