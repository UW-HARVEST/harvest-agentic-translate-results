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
    // The C inline function computes (to - start) and bounds-checks.
    // Here we compute the byte distance between start.as_ptr() and to.as_ptr().
    // Since we don't have raw pointers in safe Rust, we use len() difference
    // assuming `to` is a suffix of (or equal to) `start` in the same buffer.
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

// ---------------------------------------------------------------------------
// Character classification helpers
// ---------------------------------------------------------------------------

pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let (mut a, mut b) = (a, b);
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

// ---------------------------------------------------------------------------
// skp_next: read next "char" (UTF-8 aware).
// ---------------------------------------------------------------------------

/// Internal helper that returns the byte advance and the codepoint.
fn skp_next_bytes(bytes: &[u8], iso: i32) -> (u32, usize) {
    if bytes.is_empty() || bytes[0] == 0 {
        return (0, 0);
    }
    let mut idx = 1usize;
    let mut c: u32 = bytes[0] as u32;
    if iso & 2 == 0 {
        // C uses iso==0 for UTF-8; the code in match passes (*flg & 2)
        // which is non-zero when ISO mode. But the parameter we accept
        // here is the *raw* iso flag (matching the C function's `iso` arg).
        // The C source decides UTF-8 vs ISO by checking `if (!iso)`.
        // Read continuation bytes if present (max 3 more).
    }
    // The above iso check is not quite the same as C — C's `if (!iso)` means
    // "UTF-8 if iso is zero". Match that here.
    if iso == 0 {
        for _ in 0..3 {
            if idx < bytes.len() && (bytes[idx] & 0xC0) == 0x80 {
                c = (c << 8) | (bytes[idx] as u32);
                idx += 1;
            } else {
                break;
            }
        }
    }
    if c == 0x0D && idx < bytes.len() && bytes[idx] == 0x0A {
        c = 0x0D0A;
        idx += 1;
    }
    (c, idx)
}

pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let (c, idx) = skp_next_bytes(s.as_bytes(), iso);
    // Bounds: idx <= s.len(); for valid UTF-8 strings, idx falls on a char
    // boundary because we always consume complete UTF-8 sequences (or a
    // single ASCII byte).
    if idx == 0 {
        return (c, s);
    }
    if s.is_char_boundary(idx) {
        (c, &s[idx..])
    } else {
        // Fallback: should not happen in normal use; pick the largest
        // valid suffix at or beyond idx.
        let mut k = idx;
        while k < s.len() && !s.is_char_boundary(k) {
            k += 1;
        }
        (c, &s[k..])
    }
}

// ---------------------------------------------------------------------------
// is_oneof and is_string
// ---------------------------------------------------------------------------

pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let mut s = set;
    let (mut p_ch, rest) = skp_next(s, iso);
    s = rest;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        let (np, r) = skp_next(s, iso);
        p_ch = np;
        s = r;
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, r) = skp_next(s, iso);
        p_ch = np;
        s = r;
        // Check for range a-b
        if p_ch == b'-' as u32 && !s.is_empty() && s.as_bytes()[0] != b']' {
            let (np2, r2) = skp_next(s, iso);
            p_ch = np2;
            s = r2;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (np3, r3) = skp_next(s, iso);
            p_ch = np3;
            s = r3;
        }
    }
    false
}

pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    let s_bytes = s.as_bytes();
    let p_bytes = p.as_bytes();
    let mut s_idx = 0usize;
    let mut p_idx = 0usize;
    let start_s = 0usize;
    let mut len = len;
    let mut mlen: i32 = 0;
    let iso = flg & 2;
    let fold = flg & 1;

    while len > 0 {
        if p_idx >= p_bytes.len() || p_bytes[p_idx] == 0x0E {
            return mlen;
        }
        let (p_chr, p_adv) = skp_next_bytes(&p_bytes[p_idx..], iso);
        let (s_chr, s_adv) = skp_next_bytes(&s_bytes[s_idx..], iso);

        if chr_cmp(s_chr, p_chr, fold) {
            mlen += s_adv as i32;
            len -= p_adv as i32;
            p_idx += p_adv;
            s_idx += s_adv;
        } else {
            // Search for an alternative (\xE)
            while len > 0 && p_idx < p_bytes.len() {
                let b = p_bytes[p_idx];
                p_idx += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
            if len <= 0 {
                return 0;
            }
            len -= 1;
            s_idx = start_s;
            mlen = 0;
        }
    }
    mlen
}

pub fn get_close(open: u32) -> u32 {
    match open {
        x if x == b'(' as u32 => b')' as u32,
        x if x == b'[' as u32 => b']' as u32,
        x if x == b'{' as u32 => b'}' as u32,
        x if x == b'<' as u32 => b'>' as u32,
        _ => 0,
    }
}

pub fn get_qclose(open: u32) -> u32 {
    match open {
        x if x == b'\'' as u32 => open,
        x if x == b'"' as u32 => open,
        x if x == b'`' as u32 => open,
        _ => 0,
    }
}

// ---------------------------------------------------------------------------
// match_pat
// ---------------------------------------------------------------------------

pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Internal match function operating on byte slices.
/// Returns (match_result, new_p_idx, new_s_idx).
fn match_bytes(
    pat: &[u8],
    src: &[u8],
    p_idx0: usize,
    s_idx0: usize,
    flg: &mut i32,
) -> (i32, usize, usize) {
    let mut p_idx = p_idx0;
    let mut ret = MATCHED_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_cnt: u32;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    // s_end is the byte position before consuming the current char;
    // s_tmp is the byte position after consuming it.
    let mut s_end = s_idx0;
    let (mut s_chr, s_adv) = skp_next_bytes(&src[s_end..], *flg & 2);
    let mut s_tmp = s_end + s_adv;

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

    if p_idx >= pat.len() {
        return (MATCHED_FAIL, p_idx0, s_idx0);
    }

    let pat_chr = pat[p_idx];
    p_idx += 1;

    // Helper for "W" macro: count matching chars.
    let mut do_w = |test: &dyn Fn(u32) -> bool,
                    p_idx_unused: &mut usize,
                    s_end: &mut usize,
                    s_chr: &mut u32,
                    s_tmp: &mut usize|
     -> i32 {
        let _ = p_idx_unused;
        let mut cnt: u32 = 0;
        while cnt < match_max && (*s_chr != 0 && (test(*s_chr) != (match_not != 0))) {
            *s_end = *s_tmp;
            let (nc, na) = skp_next_bytes(&src[*s_end..], *flg & 2);
            *s_chr = nc;
            *s_tmp = *s_end + na;
            cnt += 1;
        }
        if cnt >= match_min {
            MATCHED
        } else {
            MATCHED_FAIL
        }
    };

    // get_next_s_chr: advance one ASCII byte (different semantics than skp_next)
    let get_next = |s_end: &mut usize, s_chr: &mut u32, s_tmp: &mut usize, src: &[u8]| {
        *s_end = *s_tmp;
        *s_chr = if *s_end < src.len() {
            src[*s_end] as u32
        } else {
            0
        };
        *s_tmp = *s_end + 1;
    };

    intnumber = false;

    match pat_chr {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                ret = do_w(&|c| c != 0, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                ret = do_w(&is_break, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
            }
        }
        b'n' => {
            ret = do_w(&is_break, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'd' => {
            ret = do_w(&is_digit, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'x' => {
            ret = do_w(&is_xdigit, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'a' => {
            ret = do_w(&is_alpha, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'u' => {
            ret = do_w(&is_upper, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'l' => {
            ret = do_w(&is_lower, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b's' => {
            ret = do_w(&is_space, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'w' => {
            ret = do_w(&is_blank, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'c' => {
            ret = do_w(&is_ctrl, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'i' => {
            ret = do_w(&is_idchr, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'@' => {
            ret = do_w(&is_alnum, &mut p_idx, &mut s_end, &mut s_chr, &mut s_tmp);
        }
        b'&' => {
            ret = if match_not != 0 {
                MATCHED_GOALNOT
            } else {
                MATCHED_GOAL
            };
        }
        b'[' => {
            // Build set as &str slice from pat[p_idx..]
            let set_bytes = &pat[p_idx..];
            // SAFETY: pat originally came from a &str (valid UTF-8)
            let set_str = unsafe { std::str::from_utf8_unchecked(set_bytes) };
            ret = do_w(
                &|c| is_oneof(c, set_str, *flg & 2),
                &mut p_idx,
                &mut s_end,
                &mut s_chr,
                &mut s_tmp,
            );
            // Skip past closing ']'
            if p_idx < pat.len() && pat[p_idx] == b']' {
                p_idx += 1;
            }
            while p_idx < pat.len() && pat[p_idx] != b']' {
                p_idx += 1;
            }
            if p_idx < pat.len() {
                p_idx += 1;
            }
        }
        q @ (b'"' | b'\'' | b'`') => {
            let mut l = 0usize;
            while p_idx + l < pat.len() && pat[p_idx + l] != q {
                l += 1;
            }
            // Use the slice pat[p_idx..p_idx+l] as the pattern for is_string.
            if l > 0 {
                let p_slice = unsafe { std::str::from_utf8_unchecked(&pat[p_idx..]) };
                let s_slice = unsafe { std::str::from_utf8_unchecked(&src[s_end..]) };
                let ml = is_string(s_slice, p_slice, l as i32, *flg);
                if ml > 0 {
                    if match_not == 0 {
                        s_end += ml as usize;
                        ret = MATCHED;
                    } else {
                        // match_not + matched -> not matched
                    }
                } else if match_min == 0 || match_not != 0 {
                    ret = MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            }
            p_idx += l + 1;
            // s_tmp may be stale relative to s_end; refresh it.
            let (nc, na) = skp_next_bytes(&src[s_end..], *flg & 2);
            s_chr = nc;
            s_tmp = s_end + na;
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
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
            }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) {
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
            }
            ret = MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !is_break(s_chr) {
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
            }
            if s_chr != 0 {
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
            }
            ret = MATCHED;
        }
        b'I' => {
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            // case '(' fall-through to 'B' if pat[p_idx]==')' AND s_chr=='('
            if p_idx < pat.len() && pat[p_idx] == b')' && s_chr == b'(' as u32 {
                p_idx += 1;
                // Fall through to 'B' logic
                let open = s_chr;
                let close = get_close(open);
                if close != 0 {
                    let mut count: i32 = 1;
                    let mut sc = s_chr;
                    while sc != 0 && count > 0 {
                        get_next(&mut s_end, &mut sc, &mut s_tmp, src);
                        if sc == open {
                            count += 1;
                        }
                        if sc == close {
                            count -= 1;
                        }
                    }
                    s_chr = sc;
                    if count == 0 {
                        get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
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
                let mut sc = s_chr;
                while sc != 0 && count > 0 {
                    get_next(&mut s_end, &mut sc, &mut s_tmp, src);
                    if sc == open {
                        count += 1;
                    }
                    if sc == close {
                        count -= 1;
                    }
                }
                s_chr = sc;
                if count == 0 {
                    get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
                    ret = MATCHED;
                }
            }
        }
        b'Q' => {
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                let mut sc = s_chr;
                while sc != 0 {
                    get_next(&mut s_end, &mut sc, &mut s_tmp, src);
                    if sc == qclose {
                        break;
                    }
                    if sc == b'\\' as u32 {
                        get_next(&mut s_end, &mut sc, &mut s_tmp, src);
                    }
                }
                s_chr = sc;
                if s_chr != 0 {
                    get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            // Hex number (optional 0x prefix)
            let s1 = if s_end + 1 < src.len() {
                src[s_end + 1]
            } else {
                0
            };
            let s2 = if s_end + 2 < src.len() {
                src[s_end + 2]
            } else {
                0
            };
            if s_chr == b'0' as u32 && (s1 == b'x' || s1 == b'X') && is_xdigit(s2 as u32) {
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                get_next(&mut s_end, &mut s_chr, &mut s_tmp, src);
            }
        }
        b'D' => {
            intnumber = true;
            // Fall through manually (Rust doesn't do C fall-through).
            do_number(
                src,
                &mut s_end,
                &mut s_chr,
                &mut s_tmp,
                &mut ret,
                intnumber,
                &get_next,
            );
        }
        b'F' => {
            do_number(
                src,
                &mut s_end,
                &mut s_chr,
                &mut s_tmp,
                &mut ret,
                false,
                &get_next,
            );
        }
        _ => {
            ret = MATCHED_FAIL;
            p_idx -= 1;
        }
    }

    if ret != MATCHED_FAIL {
        (ret, p_idx, s_end)
    } else {
        (ret, p_idx0, s_idx0)
    }
}

fn do_number<F>(
    src: &[u8],
    s_end: &mut usize,
    s_chr: &mut u32,
    s_tmp: &mut usize,
    ret: &mut i32,
    intnumber: bool,
    get_next: &F,
) where
    F: Fn(&mut usize, &mut u32, &mut usize, &[u8]),
{
    if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
        loop {
            get_next(s_end, s_chr, s_tmp, src);
            if !is_space(*s_chr) {
                break;
            }
        }
    }

    while is_digit(*s_chr) {
        *ret = MATCHED;
        get_next(s_end, s_chr, s_tmp, src);
    }

    if intnumber {
        return;
    }

    if *s_chr == b'.' as u32 {
        get_next(s_end, s_chr, s_tmp, src);
    }

    while is_digit(*s_chr) {
        *ret = MATCHED;
        get_next(s_end, s_chr, s_tmp, src);
    }

    if *ret == MATCHED && (*s_chr == b'E' as u32 || *s_chr == b'e' as u32) {
        get_next(s_end, s_chr, s_tmp, src);
        if *s_chr == b'+' as u32 || *s_chr == b'-' as u32 {
            get_next(s_end, s_chr, s_tmp, src);
        }
        while is_digit(*s_chr) {
            get_next(s_end, s_chr, s_tmp, src);
        }
        if *s_chr == b'.' as u32 {
            get_next(s_end, s_chr, s_tmp, src);
        }
        while is_digit(*s_chr) {
            get_next(s_end, s_chr, s_tmp, src);
        }
    }
}

pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let pb = pat.as_bytes();
    let sb = src.as_bytes();
    let (ret, p_idx, s_idx) = match_bytes(pb, sb, 0, 0, flg);
    let p_end = unsafe { std::str::from_utf8_unchecked(&pb[p_idx.min(pb.len())..]) };
    let s_end = unsafe { std::str::from_utf8_unchecked(&sb[s_idx.min(sb.len())..]) };
    (ret, s_end, p_end)
}

// ---------------------------------------------------------------------------
// skp_  — main matching routine
// ---------------------------------------------------------------------------

/// Internal version returning byte indices.
fn skp_bytes(src: &[u8], pat: &[u8]) -> (i32, usize, usize) {
    // Returns (ret, to_idx, end_idx) where to_idx is the "to" pointer position
    // (in the src buffer) and end_idx is the "end" position.
    if pat.is_empty() && src.is_empty() {
        return (0, 0, 0);
    }
    let mut start = 0usize;
    let mut s = start;
    let mut p_start = 0usize;
    let mut skp_to = false;

    if !pat.is_empty() && pat[0] == b'>' {
        skp_to = true;
        p_start = 1;
    }
    let pat_after_gt = p_start;

    let mut p = p_start;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    // Skip whitespace at start
    while p < pat.len() && is_space(pat[p] as u32) {
        p += 1;
    }

    while p < pat.len() && pat[p] > b'\x07' {
        let (m, np, ns) = match_bytes(pat, src, p, s, &mut flg);
        matched = m;
        if m != 0 {
            s = ns;
            p = np;
            if m == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s);
            } else if m == MATCHED_GOALNOT {
                goalnot = Some(s);
            }
        } else {
            // skip to next \xE alternative
            while p < pat.len() && pat[p] > b'\x07' {
                p += 1;
            }
            if p < pat.len() && pat[p] > 0 && p + 1 < pat.len() && pat[p + 1] > 0 {
                // Try a new pattern
                s = start;
                p += 1;
            } else if skp_to {
                goal = None;
                goalnot = None;
                p = pat_after_gt;
                start += 1;
                s = start;
                if start >= src.len() {
                    break;
                }
            } else {
                break;
            }
        }
        while p < pat.len() && is_space(pat[p] as u32) {
            p += 1;
        }
    }

    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        // emulate p="" (empty pattern at end)
        // We'll set p so that pat[p] <= '\7' is true (use len, treated as terminator).
        // Use a sentinel: we'll adjust the check below.
        p = pat.len(); // means "out of pattern", treated as terminator
    }

    if let Some(g) = goal {
        s = g;
    }

    let term_byte = if p < pat.len() { pat[p] } else { 0 };

    if matched != 0 && term_byte <= b'\x07' {
        let ret = if term_byte > 0 { term_byte as i32 } else { 1 };
        let to_idx = if skp_to { start } else { s };
        return (ret, to_idx, s);
    }

    // Failure: to and end == src start (which is position 0 in original src)
    (0, 0, 0)
}

pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    let sb = src.as_bytes();
    let pb = pat.as_bytes();
    let (ret, to_idx, end_idx) = skp_bytes(sb, pb);
    let to_idx = to_idx.min(sb.len());
    let end_idx = end_idx.min(sb.len());
    let to_str = unsafe { std::str::from_utf8_unchecked(&sb[to_idx..]) };
    let end_str = unsafe { std::str::from_utf8_unchecked(&sb[end_idx..]) };
    (ret, to_str, end_str)
}

pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    // The C macro skp_4(s,p,e,t) maps to skp_(s,p,e,t):
    // i.e., 3rd arg is `to`, 4th arg is `end`.
    let (ret, t_str, e_str) = skp_(src, pat);
    // SAFETY: extending lifetime; we promise the caller manages this.
    let t_str: &str = unsafe { std::mem::transmute::<&str, &'static str>(t_str) };
    let e_str: &str = unsafe { std::mem::transmute::<&str, &'static str>(e_str) };
    if let Some(t) = to {
        *t = t_str;
    }
    if let Some(e) = end {
        *e = e_str;
    }
    ret
}

pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    skp_4(src, pat, end, None)
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_4(src, pat, None, None)
}

// ---------------------------------------------------------------------------
// AST types and helpers
// ---------------------------------------------------------------------------

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

pub const ASTNULL: i32 = -1;
pub const SKP_DEBUG_FLAG: i8 = 0x01;

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
    // The C function frees memory. In Rust, dropping Ast does that.
    None
}

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
    ast.flg = if debug != 0 { SKP_DEBUG_FLAG } else { 0 };
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
        let cur_pos = ast.pos;
        ast_close(&mut ast, cur_pos, open);
        if ast.nodes_cnt > 0 {
            ast.err_pos = -1;
            let info = ast.lastinfo;
            ast_setinfo(&mut ast, info, 0);
        }
    }
    // skp_mmz_clean
    ast.mmz.clear();
    ast.mmz_cnt = 0;
    Some(ast)
}

pub fn skp_debug2(ast: &mut Ast, d: u8) -> i32 {
    match d {
        0 => ast.flg &= !SKP_DEBUG_FLAG,
        1 => ast.flg |= SKP_DEBUG_FLAG,
        _ => ast.flg ^= SKP_DEBUG_FLAG,
    }
    (ast.flg & SKP_DEBUG_FLAG) as i32
}

pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return Some("");
    }
    ast.err_rule.as_deref()
}

pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 {
        return Some("");
    }
    let pos = ast.err_pos as usize;
    if pos <= ast.start.len() {
        Some(&ast.start[pos..])
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
    let err_off = ast.err_pos as usize;
    let line_off = ast.start.len() - line.len();
    (err_off - line_off) as i32
}

pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    if !msg.is_empty() {
        ast.err_msg = Some(msg.to_string());
    }
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    // No setjmp/longjmp in Rust — just signal failure.
    ast.fail = 1;
}

pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // Memoization is an internal optimization. Without the rule's macro
    // expansion of skprule_/skprule, this isn't directly invocable from Rust
    // user code, but we provide a no-op-equivalent that wouldn't break
    // correctness. (The C version stores AST nodes in the slot.)
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
    if node < 0 {
        return;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        let new_node = node + ast.par[idx];
        if new_node < 0 {
            return;
        }
        idx = new_node as usize;
    }
    let n_idx = ast.par[idx] as usize;
    ast.nodes[n_idx].tag = info;
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail == 0 {
        let par = ast_open(ast, ast.pos, "#");
        ast_close(ast, ast.pos, par);
        let n_idx = ast.par[par as usize] as usize;
        ast.nodes[n_idx].tag = info;
        ast.lastinfo = info;
    }
}

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
    let n_idx = ast.par[node as usize] as usize;
    ast.nodes[n_idx].tag
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
    let l1 = (c2 - o2 + 1) as usize;
    let l2 = (c1 - o1 + 1) as usize;
    let mut tmp: Vec<i32> = Vec::with_capacity(l1);
    for k in 0..l1 {
        tmp.push(ast.par[(o2 as usize) + k]);
    }
    // Move (o1..=c1) to start at o2
    for k in 0..l2 {
        ast.par[(o2 as usize) + k] = ast.par[(o1 as usize) + k];
    }
    // Place tmp after the moved block
    for k in 0..l1 {
        ast.par[(o2 as usize) + l2 + k] = tmp[k];
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
    if lft < 0 || rgt < 0 {
        return;
    }
    let node_from = ast.nodes[ast.par[lft as usize] as usize].from;
    let node_to = ast.nodes[ast.par[rgt as usize] as usize].to;
    let rgt_delta = ast.nodes[ast.par[rgt as usize] as usize].delta;
    let rgt = rgt + rgt_delta;
    let new_node_idx = ast.nodes_cnt;
    let delta = rgt - lft + 2;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta,
        tag: 0,
    });
    ast.nodes_cnt += 1;

    // Make room for two new par entries
    ast.par.push(0);
    ast.par.push(0);
    ast.par_cnt += 2;

    // Move the nodes after rgt (originally par_cnt-1-rgt-2 elements) over by 2
    let new_par_cnt = ast.par_cnt;
    let trail_count = (new_par_cnt - 1 - rgt - 2) as i32;
    if trail_count > 2 {
        // Shift par[rgt+1..new_par_cnt-2] -> par[rgt+3..new_par_cnt]
        for i in (0..(trail_count - 2) as usize).rev() {
            ast.par[(rgt as usize) + 3 + i] = ast.par[(rgt as usize) + 1 + i];
        }
    }

    // Move par[lft..=rgt] to par[lft+1..=rgt+1]
    for i in (0..(rgt - lft + 1) as usize).rev() {
        ast.par[(lft as usize) + 1 + i] = ast.par[(lft as usize) + i];
    }
    ast.par[lft as usize] = new_node_idx;
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
        // memmove par[o1..] = par[o2..o2 + (c2-o2+1)]
        let len = (c2 - o2 + 1) as usize;
        for i in 0..len {
            ast.par[(o1 as usize) + i] = ast.par[(o2 as usize) + i];
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
    let n = &ast.nodes[ast.par[o1 as usize] as usize];
    if n.from != n.to {
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
    if ast.par[node as usize] < 0 {
        return false;
    }
    let nd = &ast.nodes[ast.par[node as usize] as usize];
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
    if node <= 0 {
        return ASTNULL;
    }
    if ast.par[node as usize] >= 0 {
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
        let n_idx = ast.par[node as usize] as usize;
        node += ast.nodes[n_idx].delta;
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
    let mut cur = node;
    loop {
        let l = astleft(ast, cur);
        if l == ASTNULL {
            break;
        }
        cur = l;
    }
    cur
}

pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
    let mut cur = node;
    loop {
        let r = astright(ast, cur);
        if r == ASTNULL {
            break;
        }
        cur = r;
    }
    cur
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
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return "";
    }
    let idx = ast.par[node as usize] as usize;
    &ast.nodes[idx].rule
}

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
    if node < 0 {
        return "";
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
    if node < 0 {
        return 0;
    }
    let n = &ast.nodes[ast.par[node as usize] as usize];
    n.to - n.from
}

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
    ast.nodes[ast.par[node as usize] as usize].delta == 1
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
    let mut ret = ast_is(ast, node, r1);
    if ret == 0 {
        if let Some(r) = r2 {
            ret = ast_is(ast, node, r);
        }
    }
    if ret == 0 {
        if let Some(r) = r3 {
            ret = ast_is(ast, node, r);
        }
    }
    if ret == 0 {
        if let Some(r) = r4 {
            ret = ast_is(ast, node, r);
        }
    }
    if ret == 0 {
        if let Some(r) = r5 {
            ret = ast_is(ast, node, r);
        }
    }
    ret
}

pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt || node < 0 {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 {
        return 0;
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
                if astnoderule(ast, node) == "#" {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from_str = astnodefrom(ast, node);
                    let to_str = astnodeto(ast, node);
                    let from_len = ast.start.len() - from_str.len();
                    let to_len = ast.start.len() - to_str.len();
                    if from_len <= to_len && to_len <= ast.start.len() {
                        for byte in &ast.start.as_bytes()[from_len..to_len] {
                            if *byte == b'\'' {
                                let _ = write!(f, "\\");
                            }
                            let _ = f.write_all(&[*byte]);
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

pub fn astprinttree(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: i32 = ASTNULL;
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
                let from_len = ast.start.len() - from_str.len();
                let to_len = ast.start.len() - to_str.len();
                if from_len <= to_len && to_len <= ast.start.len() {
                    for byte in &ast.start.as_bytes()[from_len..to_len] {
                        if *byte == b'\'' {
                            let _ = write!(f, "\\");
                        }
                        let _ = f.write_all(&[*byte]);
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
