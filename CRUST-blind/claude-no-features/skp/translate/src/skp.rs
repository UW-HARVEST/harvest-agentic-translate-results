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
    // In C: ret = to - start (pointer difference). In our model, `to` is a suffix
    // of `start`, so the offset is `start.len() - to.len()`.
    let ret = (start.len() as i64) - (to.len() as i64);
    if ret >= 0 && ret <= (1 << 16) {
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

// ---------------------------------------------------------------------------
// Helpers - byte-level scanning
// ---------------------------------------------------------------------------

/// Internal: read the next "char" (UTF-8 packed into u32) from `bytes` starting
/// at index `i`. Returns (char, new_index).
fn skp_next_bytes(bytes: &[u8], iso: i32) -> (u32, usize) {
    if bytes.is_empty() {
        return (0, 0);
    }
    let mut i = 0usize;
    let mut c: u32 = bytes[0] as u32;
    i += 1;
    if iso == 0 {
        if i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
            c = (c << 8) | (bytes[i] as u32);
            i += 1;
            if i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                c = (c << 8) | (bytes[i] as u32);
                i += 1;
                if i < bytes.len() && (bytes[i] & 0xC0) == 0x80 {
                    c = (c << 8) | (bytes[i] as u32);
                    i += 1;
                }
            }
        }
    }
    if c == 0x0D && i < bytes.len() && bytes[i] == 0x0A {
        c = 0x0D0A;
        i += 1;
    }
    (c, i)
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, i) = skp_next_bytes(bytes, iso);
    let rest = if i <= s.len() && s.is_char_boundary(i) {
        &s[i..]
    } else {
        // Fall back: try to find a valid boundary at or after i
        let mut j = i;
        while j < s.len() && !s.is_char_boundary(j) {
            j += 1;
        }
        if j <= s.len() {
            &s[j..]
        } else {
            ""
        }
    };
    (c, rest)
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        if (b'A' as u32..=b'Z' as u32).contains(&a) {
            a = a + 32;
        }
        if (b'A' as u32..=b'Z' as u32).contains(&b) {
            b = b + 32;
        }
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
    c == 0x0D0A || c == 0xC285 || c == 0xE280A8 || c == 0xE280A9
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

/// Returns true if `ch` is one of the characters in `set`. The `iso` flag is used for encoding.
pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let bytes = set.as_bytes();
    let mut idx = 0usize;
    let (mut p_ch, n) = skp_next_bytes(&bytes[idx..], iso);
    idx += n;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let (np, n2) = skp_next_bytes(&bytes[idx..], iso);
            p_ch = np;
            idx += n2;
        }
    }

    while p_ch != b']' as u32 {
        if p_ch == 0 {
            return false;
        }
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, n2) = skp_next_bytes(&bytes[idx..], iso);
        p_ch = np;
        idx += n2;
        // Check the *next* byte (the C code peeks `*s` directly).
        let next_byte_is_close = idx < bytes.len() && bytes[idx] == b']';
        if p_ch == b'-' as u32 && !next_byte_is_close {
            let (np2, n3) = skp_next_bytes(&bytes[idx..], iso);
            p_ch = np2;
            idx += n3;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, n4) = skp_next_bytes(&bytes[idx..], iso);
            p_ch = np3;
            idx += n4;
        }
    }
    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    let s_bytes = s.as_bytes();
    let p_bytes = p.as_bytes();
    let mut s_idx = 0usize;
    let mut p_idx = 0usize;
    let mut len = len;
    let mut mlen: i32 = 0;
    let s_start: usize = 0;
    while len > 0 {
        if p_idx >= p_bytes.len() {
            return 0;
        }
        if p_bytes[p_idx] == 0x0E {
            return mlen;
        }
        let (p_chr, pn) = skp_next_bytes(&p_bytes[p_idx..], flg & 2);
        let (s_chr, sn) = skp_next_bytes(&s_bytes[s_idx..], flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += sn as i32;
            len -= pn as i32;
            p_idx += pn;
            s_idx += sn;
        } else {
            // Search for an alternative
            while len > 0 && p_idx < p_bytes.len() {
                let b = p_bytes[p_idx];
                p_idx += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
            // The C code does: while (len>0 && *p++ != '\xE') len--; if (len-- <=0) return 0;
            // After the loop above, if we exited because of \xE, len was NOT decremented for it.
            // Actually let me re-check: loop decrements len for each char advanced (excluding the \xE which is found), then post-checks.
            // The C does (len-- <= 0) return 0. So len could become -1 from this decrement, but only if it was 0 - returns 0.
            // Actually `if (len-- <= 0) return 0` decrements len AFTER the check. If len was 0, returns 0. Otherwise len becomes len-1.
            if len <= 0 {
                return 0;
            }
            len -= 1;
            s_idx = s_start;
            mlen = 0;
        }
    }
    mlen
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    match open {
        x if x == b'(' as u32 => b')' as u32,
        x if x == b'[' as u32 => b']' as u32,
        x if x == b'{' as u32 => b'}' as u32,
        x if x == b'<' as u32 => b'>' as u32,
        _ => 0,
    }
}

/// Given a quote character, returns the corresponding closing quote.
pub fn get_qclose(open: u32) -> u32 {
    match open {
        x if x == b'\'' as u32 => open,
        x if x == b'"' as u32 => open,
        x if x == b'`' as u32 => open,
        _ => 0,
    }
}

/// Constants for match results.
pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Internal byte-index version of `match`.
fn match_bytes(
    pat_bytes: &[u8],
    pat_start: usize,
    src_bytes: &[u8],
    src_start: usize,
    flg: &mut i32,
) -> (i32, usize, usize) {
    // Returns (ret, src_end_idx, pat_end_idx)
    let mut p_idx = pat_start;
    let mut s_idx = src_start; // s_end in C
    let mut s_tmp; // index after current char
    let mut s_chr: u32;
    let mut ret: i32 = MATCHED_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = u32::MAX;
    let mut match_cnt: u32;
    let mut match_not: u32 = 0;
    let mut intnumber: bool = false;

    // Initialize counters with the same defaults as C
    match_min = 1;
    match_max = 1;

    // Read first char from src
    {
        let (c, n) = skp_next_bytes(&src_bytes[s_idx..], *flg & 2);
        s_chr = c;
        s_tmp = s_idx + n;
    }

    if p_idx < pat_bytes.len() {
        match pat_bytes[p_idx] {
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
    if p_idx < pat_bytes.len() && pat_bytes[p_idx] == b'!' {
        match_not = 1;
        p_idx += 1;
    }

    if p_idx >= pat_bytes.len() {
        return (MATCHED_FAIL, s_idx, p_idx);
    }

    let cur = pat_bytes[p_idx];
    p_idx += 1;

    intnumber = false;

    macro_rules! w_macro {
        ($cond:expr) => {{
            match_cnt = 0;
            while match_cnt < match_max && s_chr != 0 {
                let cond_val: bool = $cond(s_chr);
                if (cond_val as u32) == match_not {
                    break;
                }
                s_idx = s_tmp;
                let (c, n) = skp_next_bytes(&src_bytes[s_idx..], *flg & 2);
                s_chr = c;
                s_tmp = s_idx + n;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    // get_next_s_chr: s_end = s_tmp; s_chr = *s_end; s_tmp++
    macro_rules! get_next_s_chr {
        () => {{
            s_idx = s_tmp;
            s_chr = if s_idx < src_bytes.len() {
                src_bytes[s_idx] as u32
            } else {
                0
            };
            s_tmp = s_idx + 1;
        }};
    }

    let mut handled = true;
    let mut fall_to_n = false;
    match cur {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                w_macro!(|c: u32| c != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                fall_to_n = true;
            }
        }
        b'n' => {
            fall_to_n = true;
        }
        b'd' => w_macro!(|c| is_digit(c)),
        b'x' => w_macro!(|c| is_xdigit(c)),
        b'a' => w_macro!(|c| is_alpha(c)),
        b'u' => w_macro!(|c| is_upper(c)),
        b'l' => w_macro!(|c| is_lower(c)),
        b's' => w_macro!(|c| is_space(c)),
        b'w' => w_macro!(|c| is_blank(c)),
        b'c' => w_macro!(|c| is_ctrl(c)),
        b'i' => w_macro!(|c| is_idchr(c)),
        b'@' => w_macro!(|c| is_alnum(c)),
        b'&' => {
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'[' => {
            // The 'set' is the rest of pattern starting at p_idx, treated as a string.
            // We need to construct a &str from the pattern bytes starting at p_idx.
            let set_bytes = &pat_bytes[p_idx..];
            // Use a raw approach: use is_oneof with byte interpretation.
            // To pass `&str`, we need to ensure UTF-8 validity. We'll use a helper that
            // operates on bytes directly.
            w_macro!(|c| is_oneof_bytes(c, set_bytes, *flg & 2));
            // Skip the set in pattern: handle initial ']'
            if p_idx < pat_bytes.len() && pat_bytes[p_idx] == b']' {
                p_idx += 1;
            }
            while p_idx < pat_bytes.len() && pat_bytes[p_idx] != b']' {
                p_idx += 1;
            }
            if p_idx < pat_bytes.len() {
                p_idx += 1; // skip the closing ]
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = cur;
            let mut l: usize = 0;
            while p_idx + l < pat_bytes.len() && pat_bytes[p_idx + l] != quote {
                l += 1;
            }
            let mut outer_if_true = false;
            if l > 0 {
                let s_slice = &src_bytes[s_idx..];
                let p_slice = &pat_bytes[p_idx..p_idx + l];
                let ml = is_string_bytes(s_slice, p_slice, l as i32, *flg);
                if ml > 0 {
                    outer_if_true = true;
                    if match_not == 0 {
                        s_idx += ml as usize;
                        ret = MATCHED;
                    }
                }
            }
            if !outer_if_true {
                if match_min == 0 || match_not != 0 {
                    ret = MATCHED;
                }
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
            // case '(': if (*pat != ')' || s_chr != '(') break;
            //          pat++;
            //          // fallthrough to 'B'
            if p_idx < pat_bytes.len() && pat_bytes[p_idx] == b')' && s_chr == b'(' as u32 {
                p_idx += 1;
                // fall through to balanced
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
                && s_idx + 1 < src_bytes.len()
                && (src_bytes[s_idx + 1] == b'x' || src_bytes[s_idx + 1] == b'X')
                && s_idx + 2 < src_bytes.len()
                && is_xdigit(src_bytes[s_idx + 2] as u32)
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
            // fall through
            handle_df(
                &mut s_idx,
                &mut s_tmp,
                &mut s_chr,
                src_bytes,
                &mut ret,
                intnumber,
            );
        }
        b'F' => {
            handle_df(
                &mut s_idx,
                &mut s_tmp,
                &mut s_chr,
                src_bytes,
                &mut ret,
                intnumber,
            );
        }
        _ => {
            ret = MATCHED_FAIL;
            p_idx -= 1;
            handled = false;
        }
    }

    if fall_to_n {
        // 'n' case: W(is_break(s_chr))
        w_macro!(|c| is_break(c));
    }

    let _ = handled;
    let p_end = p_idx;

    if ret != MATCHED_FAIL {
        // s_idx is s_end; that's what we return
        (ret, s_idx, p_end)
    } else {
        (MATCHED_FAIL, src_start, p_end)
    }
}

/// Helper for D/F (digit/float) handling.
fn handle_df(
    s_idx: &mut usize,
    s_tmp: &mut usize,
    s_chr: &mut u32,
    src_bytes: &[u8],
    ret: &mut i32,
    intnumber: bool,
) {
    macro_rules! get_next_s_chr {
        () => {{
            *s_idx = *s_tmp;
            *s_chr = if *s_idx < src_bytes.len() {
                src_bytes[*s_idx] as u32
            } else {
                0
            };
            *s_tmp = *s_idx + 1;
        }};
    }

    if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
        loop {
            get_next_s_chr!();
            if !is_space(*s_chr) {
                break;
            }
        }
    }

    while is_digit(*s_chr) {
        *ret = MATCHED;
        get_next_s_chr!();
    }

    if intnumber {
        return;
    }

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
        while is_digit(*s_chr) {
            get_next_s_chr!();
        }
        if *s_chr == b'.' as u32 {
            get_next_s_chr!();
        }
        while is_digit(*s_chr) {
            get_next_s_chr!();
        }
    }
}

/// Byte-level is_oneof - same logic but takes a byte slice (which may not be UTF-8 valid).
fn is_oneof_bytes(ch: u32, set: &[u8], iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let mut idx = 0usize;
    let (mut p_ch, n) = skp_next_bytes(&set[idx..], iso);
    idx += n;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let (np, n2) = skp_next_bytes(&set[idx..], iso);
            p_ch = np;
            idx += n2;
        }
    }

    while p_ch != b']' as u32 {
        if p_ch == 0 {
            return false;
        }
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, n2) = skp_next_bytes(&set[idx..], iso);
        p_ch = np;
        idx += n2;
        let next_byte_is_close = idx < set.len() && set[idx] == b']';
        if p_ch == b'-' as u32 && !next_byte_is_close {
            let (np2, n3) = skp_next_bytes(&set[idx..], iso);
            p_ch = np2;
            idx += n3;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, n4) = skp_next_bytes(&set[idx..], iso);
            p_ch = np3;
            idx += n4;
        }
    }
    false
}

/// Byte-level is_string.
fn is_string_bytes(s: &[u8], p: &[u8], len: i32, flg: i32) -> i32 {
    let mut s_idx = 0usize;
    let mut p_idx = 0usize;
    let mut len = len;
    let mut mlen: i32 = 0;
    let s_start = 0usize;
    while len > 0 {
        if p_idx >= p.len() {
            // pattern exhausted
            return mlen;
        }
        if p[p_idx] == 0x0E {
            return mlen;
        }
        let (p_chr, pn) = skp_next_bytes(&p[p_idx..], flg & 2);
        let (s_chr, sn) = skp_next_bytes(&s[s_idx..], flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            mlen += sn as i32;
            len -= pn as i32;
            p_idx += pn;
            s_idx += sn;
        } else {
            // Search for an alternative
            let mut found_alt = false;
            while len > 0 && p_idx < p.len() {
                let b = p[p_idx];
                p_idx += 1;
                if b == 0x0E {
                    found_alt = true;
                    break;
                }
                len -= 1;
            }
            if !found_alt {
                return 0;
            }
            // C does: if (len-- <= 0) return 0
            if len <= 0 {
                return 0;
            }
            len -= 1;
            s_idx = s_start;
            mlen = 0;
        }
    }
    mlen
}

/// Public match_pat with str references.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let pat_bytes = pat.as_bytes();
    let src_bytes = src.as_bytes();
    let (ret, s_end_idx, p_end_idx) = match_bytes(pat_bytes, 0, src_bytes, 0, flg);
    (ret, slice_at(src, s_end_idx), slice_at(pat, p_end_idx))
}

/// Slice a `&str` at byte index, falling back to nearest valid boundary.
fn slice_at(s: &str, mut i: usize) -> &str {
    if i > s.len() {
        i = s.len();
    }
    if s.is_char_boundary(i) {
        &s[i..]
    } else {
        // walk backward to a valid boundary
        let mut j = i;
        while j > 0 && !s.is_char_boundary(j) {
            j -= 1;
        }
        &s[j..]
    }
}

/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    if pat.is_empty() && src.is_empty() {
        // Mirror: C's behavior with empty strings - skp_ would proceed with empty pattern.
    }
    let pat_bytes = pat.as_bytes();
    let src_bytes = src.as_bytes();

    let start: usize = 0;
    let mut s: usize;
    let mut p: usize;
    let mut p_end: usize;
    let mut s_end: usize;
    let mut skp_to: bool = false;
    let mut matched: i32 = 0;
    let ret: i32;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    // The C function checks pat or src null. Empty strings are not null in Rust.
    p = 0;
    if p < pat_bytes.len() && pat_bytes[p] == b'>' {
        skp_to = true;
        p += 1;
    }
    let pat_after_to = p; // start of pat after potential '>'

    s = start;

    // Skip spaces in pattern
    while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
        p += 1;
    }

    while p < pat_bytes.len() && pat_bytes[p] > 0x07 {
        let mut current_p = p;
        let mut current_s = s;
        let result = match_bytes(pat_bytes, current_p, src_bytes, current_s, &mut flg);
        matched = result.0;
        if matched != 0 {
            s_end = result.1;
            p_end = result.2;
            s = s_end;
            p = p_end;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s);
            }
        } else {
            // skip to end of current pattern alternative (chars > 7)
            while p < pat_bytes.len() && pat_bytes[p] > 0x07 {
                p += 1;
            }
            if p < pat_bytes.len() && pat_bytes[p] > 0x00 && p + 1 < pat_bytes.len() && pat_bytes[p + 1] > 0x00 {
                // Try a new pattern alternative
                s = start;
                p += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                p = pat_after_to;
                let mut new_start = start; // Wait: in C it's `s = ++start; pat = pat`, so start is incremented
                // Actually the C code increments `start` in place. We need to track it.
                // Let me do this with a mutable start variable.
                // I'll re-do this with mutable start.
                break; // Placeholder; we'll restructure below
            } else {
                break;
            }
        }
        while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
            p += 1;
        }
    }

    // Re-do this loop properly with mutable start
    // Reset and redo.
    let mut start: usize = 0;
    s = start;
    p = pat_after_to;
    // Skip spaces in pattern
    while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
        p += 1;
    }
    matched = 0;
    goal = None;
    goalnot = None;
    flg = 0;
    s_end = 0;
    p_end = 0;

    'outer: loop {
        while p < pat_bytes.len() && pat_bytes[p] > 0x07 {
            let result = match_bytes(pat_bytes, p, src_bytes, s, &mut flg);
            matched = result.0;
            if matched != 0 {
                s_end = result.1;
                p_end = result.2;
                s = s_end;
                p = p_end;
                if matched == MATCHED_GOAL && goalnot.is_none() {
                    goal = Some(s);
                } else if matched == MATCHED_GOALNOT {
                    goalnot = Some(s);
                }
            } else {
                while p < pat_bytes.len() && pat_bytes[p] > 0x07 {
                    p += 1;
                }
                // C: if ((*p > 0) && (p[1] > 0))
                let p_byte = if p < pat_bytes.len() { pat_bytes[p] } else { 0 };
                let p1_byte = if p + 1 < pat_bytes.len() { pat_bytes[p + 1] } else { 0 };
                if p_byte > 0 && p1_byte > 0 {
                    s = start;
                    p += 1;
                } else if skp_to {
                    goal = None;
                    goalnot = None;
                    p = pat_after_to;
                    start += 1;
                    s = start;
                    if s >= src_bytes.len() {
                        break 'outer;
                    }
                    // Skip pattern leading spaces
                    while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
                        p += 1;
                    }
                    continue 'outer;
                } else {
                    break 'outer;
                }
            }
            while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
                p += 1;
            }
        }
        break 'outer;
    }

    if matched == 0 {
        if let Some(gn) = goalnot {
            goal = Some(gn);
            matched = MATCHED;
            p = pat_bytes.len(); // p="" in C; no more pattern bytes
        }
    }

    if let Some(g) = goal {
        s = g;
    }

    let p_byte = if p < pat_bytes.len() { pat_bytes[p] } else { 0 };

    if matched != 0 && p_byte <= 0x07 {
        ret = if p_byte > 0 { p_byte as i32 } else { 1 };
        let to_idx = if skp_to { start } else { s };
        let end_idx = s;
        let to_str = slice_at(src, to_idx);
        let end_str = slice_at(src, end_idx);
        return (ret, to_str, end_str);
    }

    // No match path
    let to_str = src; // src is &src[0..]
    let end_str = src;
    (0, to_str, end_str)
}

/// In the C header a set of macros provides variants.
pub fn skp_4<'a>(
    src: &'a str,
    pat: &str,
    to: Option<&mut &'a str>,
    end: Option<&mut &'a str>,
) -> i32 {
    let (ret, t, e) = skp_inner(src, pat);
    if let Some(rto) = to {
        *rto = t;
    }
    if let Some(ren) = end {
        *ren = e;
    }
    ret
}

fn skp_inner<'a>(src: &'a str, pat: &str) -> (i32, &'a str, &'a str) {
    let pat_bytes = pat.as_bytes();
    let src_bytes = src.as_bytes();
    let mut start: usize = 0;
    let mut s: usize;
    let mut p: usize;
    let mut p_end: usize = 0;
    let mut s_end: usize = 0;
    let mut skp_to: bool = false;
    let mut matched: i32 = 0;
    let ret: i32;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    p = 0;
    if p < pat_bytes.len() && pat_bytes[p] == b'>' {
        skp_to = true;
        p += 1;
    }
    let pat_after_to = p;

    s = start;

    while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
        p += 1;
    }

    'outer: loop {
        while p < pat_bytes.len() && pat_bytes[p] > 0x07 {
            let result = match_bytes(pat_bytes, p, src_bytes, s, &mut flg);
            matched = result.0;
            if matched != 0 {
                s_end = result.1;
                p_end = result.2;
                s = s_end;
                p = p_end;
                if matched == MATCHED_GOAL && goalnot.is_none() {
                    goal = Some(s);
                } else if matched == MATCHED_GOALNOT {
                    goalnot = Some(s);
                }
            } else {
                while p < pat_bytes.len() && pat_bytes[p] > 0x07 {
                    p += 1;
                }
                let p_byte = if p < pat_bytes.len() { pat_bytes[p] } else { 0 };
                let p1_byte = if p + 1 < pat_bytes.len() { pat_bytes[p + 1] } else { 0 };
                if p_byte > 0 && p1_byte > 0 {
                    s = start;
                    p += 1;
                } else if skp_to {
                    goal = None;
                    goalnot = None;
                    p = pat_after_to;
                    start += 1;
                    s = start;
                    if s >= src_bytes.len() {
                        break 'outer;
                    }
                    while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
                        p += 1;
                    }
                    continue 'outer;
                } else {
                    break 'outer;
                }
            }
            while p < pat_bytes.len() && is_space(pat_bytes[p] as u32) {
                p += 1;
            }
        }
        break 'outer;
    }

    if matched == 0 {
        if let Some(gn) = goalnot {
            goal = Some(gn);
            matched = MATCHED;
            p = pat_bytes.len();
        }
    }

    if let Some(g) = goal {
        s = g;
    }

    let p_byte = if p < pat_bytes.len() { pat_bytes[p] } else { 0 };

    let _ = (s_end, p_end);

    if matched != 0 && p_byte <= 0x07 {
        ret = if p_byte > 0 { p_byte as i32 } else { 1 };
        let to_idx = if skp_to { start } else { s };
        let end_idx = s;
        let to_str = slice_at(src, to_idx);
        let end_str = slice_at(src, end_idx);
        return (ret, to_str, end_str);
    }

    (0, src, src)
}

pub fn skp_3<'a>(src: &'a str, pat: &str, end: Option<&mut &'a str>) -> i32 {
    // C: skp_3(s,p,e)  -> skp_(s,p, e,NULL).  The third C arg corresponds to
    // skp_inner's "to" output (i.e. the position to continue scanning from).
    let (ret, t, _e) = skp_inner(src, pat);
    if let Some(ren) = end {
        *ren = t;
    }
    ret
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    let (ret, _t, _e) = skp_inner(src, pat);
    ret
}

// ---------------------------------------------------------------------------
// AST functions
// ---------------------------------------------------------------------------

/// In C: `typedef int32_t astnode_t;`
pub type AstNodeT = i32;
/// Representation of an AST node.
#[derive(Debug, Default, Clone)]
pub struct AstNode {
    pub rule: String,
    pub from: AstNodeT,
    pub to: AstNodeT,
    pub delta: i32,
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
pub type SkpRule = fn(ast: &mut Ast, ret: &mut i32);

const ASTNULL: AstNodeT = -1;
const SKP_DEBUG: i8 = 0x01;
#[allow(dead_code)]
const SKP_LEFTRECUR: i8 = 0x02;
const SKP_MAXDEPTH: u16 = 10000;

/// Parses the source string `src` using a given parsing rule.
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
    if open >= 0 {
        let mut ret_val = ast.ret;
        // No setjmp/longjmp; we just call the rule.
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
            let lastinfo = ast.lastinfo;
            ast_setinfo(&mut ast, lastinfo, ASTNULL);
        }
    }
    // mmz cleanup is no-op in our model
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
    let pos = ast.err_pos as usize;
    if pos > ast.start.len() {
        return None;
    }
    if ast.start.is_char_boundary(pos) {
        Some(&ast.start[pos..])
    } else {
        None
    }
}

pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let pos = ast.err_pos as usize;
    let bytes = ast.start.as_bytes();
    if pos > bytes.len() {
        return "";
    }
    let mut ln = pos;
    while ln > 0 {
        let ch = bytes[ln - 1];
        if ch == b'\n' || ch == b'\r' {
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

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let pos = ast.err_pos as usize;
    let bytes = ast.start.as_bytes();
    let mut ln = pos;
    while ln > 0 {
        let ch = bytes[ln - 1];
        if ch == b'\n' || ch == b'\r' {
            break;
        }
        ln -= 1;
    }
    (pos - ln) as i32
}

pub fn ast_new() -> Option<Ast> {
    Some(Ast {
        start: String::new(),
        err_rule: None,
        err_msg: None,
        cur_rule: None,
        nodes: Vec::new(),
        mmz: Vec::new(),
        par: Vec::new(),
        auxptr: None,
        nodes_cnt: 0,
        nodes_max: 0,
        par_cnt: 0,
        par_max: 0,
        mmz_cnt: 0,
        mmz_max: 0,
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
    None
}

fn ast_newpar(ast: &mut Ast) -> i32 {
    let r = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;
    if ast.par_cnt > ast.par_max {
        ast.par_max = ast.par_cnt;
    }
    r
}

fn ast_newnode(ast: &mut Ast) -> i32 {
    let r = ast.nodes_cnt;
    ast.nodes.push(AstNode::default());
    ast.nodes_cnt += 1;
    if ast.nodes_cnt > ast.nodes_max {
        ast.nodes_max = ast.nodes_cnt;
    }
    r
}

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

pub fn ast_close(ast: &mut Ast, to: i32, open: i32) -> i32 {
    if open < 0 {
        return -1;
    }
    let node_idx = ast.par[open as usize] as usize;

    if ast.fail != 0 {
        ast.pos = ast.nodes[node_idx].from;
        ast.nodes_cnt = ast.par[open as usize];
        ast.par_cnt = open;
        // Truncate vectors
        ast.nodes.truncate(ast.nodes_cnt as usize);
        ast.par.truncate(ast.par_cnt as usize);
        return -1;
    }

    let par = ast_newpar(ast);
    if par < 0 {
        return -1;
    }
    let delta = par - open;
    {
        let nd = &mut ast.nodes[node_idx];
        nd.to = to;
        nd.delta = delta;
        nd.tag = 0;
    }
    ast.par[par as usize] = -delta;

    ast.cur_node = par;
    ast.cur_rule = Some(ast.nodes[node_idx].rule.clone());
    par
}

pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    if !msg.is_empty() {
        ast.err_msg = Some(msg.to_string());
    }
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
    // No longjmp; the parser sees fail=1 and bails out.
}

pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // Memoization is an optimization; safe to no-op for correctness.
}

pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

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
    if idx < 0 || idx >= ast.par_cnt {
        return;
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx < ast.nodes.len() {
        ast.nodes[nd_idx].tag = info;
    }
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail == 0 {
        let par = ast_open(ast, ast.pos, "#");
        ast_close(ast, ast.pos, par);
        if par >= 0 {
            let nd_idx = ast.par[par as usize] as usize;
            if nd_idx < ast.nodes.len() {
                ast.nodes[nd_idx].tag = info;
            }
        }
        ast.lastinfo = info;
    }
}

pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return 0;
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx < ast.nodes.len() {
        ast.nodes[nd_idx].tag
    } else {
        0
    }
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

    // Save (o2..=c2)
    let block2: Vec<i32> = ast.par[o2 as usize..=c2 as usize].to_vec();
    // Move (o1..=c1) to position o2
    let block1: Vec<i32> = ast.par[o1 as usize..=c1 as usize].to_vec();
    let len1 = block1.len();
    let len2 = block2.len();
    // Place block1 at o2
    for (i, v) in block1.iter().enumerate() {
        ast.par[o2 as usize + i] = *v;
    }
    // Place block2 immediately after
    for (i, v) in block2.iter().enumerate() {
        ast.par[o2 as usize + len1 + i] = *v;
    }
    let _ = len2;
}

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
    let lft_node = ast.par[lft as usize] as usize;
    let rgt_node = ast.par[rgt as usize] as usize;
    let node_from = ast.nodes[lft_node].from;
    let node_to = ast.nodes[rgt_node].to;
    rgt += ast.nodes[rgt_node].delta;

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

    // Move nodes after rgt: shift par[rgt+1..par_cnt-1-2] to par[rgt+3..]
    if ast.par_cnt - 1 - rgt > 2 {
        let count = (ast.par_cnt - 1 - rgt - 2) as usize;
        let src_start = (rgt + 1) as usize;
        let dst_start = (rgt + 3) as usize;
        // memmove semantics: backward copy if overlap
        for i in (0..count).rev() {
            ast.par[dst_start + i] = ast.par[src_start + i];
        }
    }
    // Shift par[lft..=rgt] one to the right (memmove)
    let count = (rgt - lft + 1) as usize;
    for i in (0..count).rev() {
        ast.par[(lft + 1) as usize + i] = ast.par[lft as usize + i];
    }
    ast.par[lft as usize] = new_node;
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
    let nd_idx = ast.par[o1 as usize] as usize;
    if ast.nodes[nd_idx].tag == 0 {
        // memmove par[o1..] = par[o2..o2+(c2-o2+1)]
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
    let nd_idx = ast.par[o1 as usize] as usize;
    if ast.nodes[nd_idx].from != ast.nodes[nd_idx].to {
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
    let nd_idx = ast.par[node as usize] as usize;
    let nd = &ast.nodes[nd_idx];
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
        let nd_idx = ast.par[node as usize] as usize;
        node += ast.nodes[nd_idx].delta;
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
    let new_node = node + 1;
    if new_node >= ast.par_cnt || ast.par[new_node as usize] < 0 {
        return ASTNULL;
    }
    new_node
}

pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut result = node;
    let mut n = node;
    loop {
        let next = astleft(ast, n);
        if next == ASTNULL {
            break;
        }
        result = next;
        n = next;
    }
    result
}

pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut result = node;
    let mut n = node;
    loop {
        let next = astright(ast, n);
        if next == ASTNULL {
            break;
        }
        result = next;
        n = next;
    }
    result
}

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

pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    if node < ast.par_cnt && node >= 0 && ast.par[node as usize] >= 0 {
        return true;
    }
    false
}

pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    if node < ast.par_cnt && node >= 0 && ast.par[node as usize] < 0 {
        return true;
    }
    false
}

pub fn astnoderule(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return "";
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx < ast.nodes.len() {
        &ast.nodes[nd_idx].rule
    } else {
        ""
    }
}

pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return "";
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx >= ast.nodes.len() {
        return "";
    }
    let from = ast.nodes[nd_idx].from as usize;
    if from > ast.start.len() {
        return "";
    }
    if ast.start.is_char_boundary(from) {
        &ast.start[from..]
    } else {
        ""
    }
}

pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node < 0 || node >= ast.par_cnt {
        return "";
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return "";
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx >= ast.nodes.len() {
        return "";
    }
    let to = ast.nodes[nd_idx].to as usize;
    if to > ast.start.len() {
        return "";
    }
    if ast.start.is_char_boundary(to) {
        &ast.start[to..]
    } else {
        ""
    }
}

pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return 0;
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx >= ast.nodes.len() {
        return 0;
    }
    ast.nodes[nd_idx].to - ast.nodes[nd_idx].from
}

pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node < 0 || node >= ast.par_cnt {
        return false;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return false;
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx >= ast.nodes.len() {
        return false;
    }
    ast.nodes[nd_idx].delta == 1
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
    if node == ASTNULL || node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut idx = node;
    if ast.par[idx as usize] < 0 {
        idx += ast.par[idx as usize];
    }
    if idx < 0 || idx >= ast.par_cnt {
        return 0;
    }
    let nd_idx = ast.par[idx as usize] as usize;
    if nd_idx >= ast.nodes.len() {
        return 0;
    }
    if ast.nodes[nd_idx].rule == rulename {
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
                    let from_str = astnodefrom(ast, node);
                    let to_str = astnodeto(ast, node);
                    let len = from_str.len().saturating_sub(to_str.len());
                    let slice = &from_str[..len];
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
                let from_str = astnodefrom(ast, node);
                let to_str = astnodeto(ast, node);
                let len = from_str.len().saturating_sub(to_str.len());
                let slice = &from_str[..len];
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

// Suppress unused warnings for SKP_MAXDEPTH which is referenced by macros not implemented here.
#[allow(dead_code)]
fn _unused() {
    let _ = SKP_MAXDEPTH;
}
