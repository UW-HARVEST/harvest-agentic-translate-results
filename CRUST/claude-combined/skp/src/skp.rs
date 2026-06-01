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
    let ret = (to.len() as isize) - (start.len() as isize);
    if 0 <= ret && ret <= (1 << 16) {
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
// Internal helpers (byte-position based)
// ---------------------------------------------------------------------------

#[inline]
fn byte_at(bytes: &[u8], pos: usize) -> u8 {
    if pos < bytes.len() {
        bytes[pos]
    } else {
        0
    }
}

fn skp_next_pos(bytes: &[u8], pos: usize, iso: i32) -> (u32, usize) {
    if byte_at(bytes, pos) == 0 {
        return (0, pos);
    }
    let mut c: u32 = bytes[pos] as u32;
    let mut i = pos + 1;
    if iso == 0 {
        if (byte_at(bytes, i) & 0xC0) == 0x80 {
            c = (c << 8) | (bytes[i] as u32);
            i += 1;
            if (byte_at(bytes, i) & 0xC0) == 0x80 {
                c = (c << 8) | (bytes[i] as u32);
                i += 1;
                if (byte_at(bytes, i) & 0xC0) == 0x80 {
                    c = (c << 8) | (bytes[i] as u32);
                    i += 1;
                }
            }
        }
    }
    if c == 0x0D && byte_at(bytes, i) == 0x0A {
        c = 0x0D0A;
        i += 1;
    }
    (c, i)
}

fn is_blank_u(c: u32) -> bool {
    if c < 0xFF {
        return c == 0x20 || c == 0x09;
    }
    match c & 0xFFFFFF00 {
        0x00000000 => c == 0xA0,
        0x0000C200 => c == 0xC2A0,
        0x00E19A00 => c == 0xE19A80,
        0x00E28000 => ((0xE28080..=0xE2808A).contains(&c)) || c == 0xE280AF,
        0x00E38080 => c == 0xE38080,
        _ => false,
    }
}

fn is_break_u(c: u32) -> bool {
    if c < 0x0F {
        return c == 0x0A || c == 0x0C || c == 0x0D;
    }
    if c < 0xFF {
        return c == 0x85;
    }
    c == 0x0D0A || c == 0xC285 || c == 0xE280A8 || c == 0xE280A9
}

fn is_space_u(c: u32) -> bool {
    is_blank_u(c) || is_break_u(c)
}

fn is_digit_u(c: u32) -> bool {
    (b'0' as u32) <= c && c <= (b'9' as u32)
}

fn is_xdigit_u(c: u32) -> bool {
    ((b'0' as u32)..=(b'9' as u32)).contains(&c)
        || ((b'A' as u32)..=(b'F' as u32)).contains(&c)
        || ((b'a' as u32)..=(b'f' as u32)).contains(&c)
}

fn is_upper_u(c: u32) -> bool {
    ((b'A' as u32)..=(b'Z' as u32)).contains(&c)
}

fn is_lower_u(c: u32) -> bool {
    ((b'a' as u32)..=(b'z' as u32)).contains(&c)
}

fn is_alpha_u(c: u32) -> bool {
    is_upper_u(c) || is_lower_u(c)
}

fn is_idchr_u(c: u32) -> bool {
    is_alpha_u(c) || is_digit_u(c) || c == (b'_' as u32)
}

fn is_alnum_u(c: u32) -> bool {
    is_alpha_u(c) || is_digit_u(c)
}

fn is_ctrl_u(c: u32) -> bool {
    c < 0x20 || (0xC280..0xC2A0).contains(&c) || (0x7F..0xA0).contains(&c)
}

fn chr_cmp_u(a: u32, b: u32, fold: i32) -> bool {
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        let af = if ((b'A' as u32)..=(b'Z' as u32)).contains(&a) {
            a + 32
        } else {
            a
        };
        let bf = if ((b'A' as u32)..=(b'Z' as u32)).contains(&b) {
            b + 32
        } else {
            b
        };
        af == bf
    } else {
        a == b
    }
}

fn get_close_u(open: u32) -> u32 {
    match open {
        x if x == b'(' as u32 => b')' as u32,
        x if x == b'[' as u32 => b']' as u32,
        x if x == b'{' as u32 => b'}' as u32,
        x if x == b'<' as u32 => b'>' as u32,
        _ => 0,
    }
}

fn get_qclose_u(open: u32) -> u32 {
    match open {
        x if x == b'\'' as u32 => open,
        x if x == b'"' as u32 => open,
        x if x == b'`' as u32 => open,
        _ => 0,
    }
}

fn is_oneof_pos(set: &[u8], pos: usize, ch: u32, iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let mut s = pos;
    let (mut p_ch, ns) = skp_next_pos(set, s, iso);
    s = ns;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let r = skp_next_pos(set, s, iso);
            p_ch = r.0;
            s = r.1;
        }
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let r = skp_next_pos(set, s, iso);
        p_ch = r.0;
        s = r.1;
        if p_ch == b'-' as u32 && byte_at(set, s) != b']' {
            let r2 = skp_next_pos(set, s, iso);
            p_ch = r2.0;
            s = r2.1;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let r3 = skp_next_pos(set, s, iso);
            p_ch = r3.0;
            s = r3.1;
        }
    }
    false
}

fn is_string_pos(
    src: &[u8],
    src_pos: usize,
    pat: &[u8],
    pat_pos: usize,
    mut len: i32,
    flg: i32,
) -> i32 {
    let start = src_pos;
    let mut s = src_pos;
    let mut p = pat_pos;
    let mut mlen: i32 = 0;

    while len > 0 {
        if byte_at(pat, p) == 0x0E {
            return mlen;
        }
        let (p_chr, p_end) = skp_next_pos(pat, p, flg & 2);
        let (s_chr, s_end) = skp_next_pos(src, s, flg & 2);

        if chr_cmp_u(s_chr, p_chr, flg & 1) {
            mlen += (s_end - s) as i32;
            len -= (p_end - p) as i32;
            p = p_end;
            s = s_end;
        } else {
            // search for alternative \xE
            loop {
                if len <= 0 {
                    break;
                }
                let b = byte_at(pat, p);
                p += 1;
                if b == 0x0E {
                    break;
                }
                len -= 1;
            }
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

const MATCHED_FAIL_I: i32 = 0;
const MATCHED_I: i32 = 1;
const MATCHED_GOAL_I: i32 = 2;
const MATCHED_GOALNOT_I: i32 = 3;

fn match_pat_pos(
    pat: &[u8],
    pat_pos: usize,
    src: &[u8],
    src_pos: usize,
    flg: &mut i32,
) -> Option<(i32, usize, usize)> {
    let mut p = pat_pos;
    let mut s_end = src_pos;
    let (mut s_chr, mut s_tmp) = skp_next_pos(src, s_end, *flg & 2);
    let mut ret: i32 = MATCHED_FAIL_I;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    if byte_at(pat, p) == b'*' {
        match_min = 0;
        match_max = u32::MAX;
        p += 1;
    } else if byte_at(pat, p) == b'+' {
        match_max = u32::MAX;
        p += 1;
    } else if byte_at(pat, p) == b'?' {
        match_min = 0;
        p += 1;
    }

    if byte_at(pat, p) == b'!' {
        match_not = 1;
        p += 1;
    }

    let pc = byte_at(pat, p);
    p += 1;

    // helper closures expressed via macros
    macro_rules! w_loop {
        ($cond:expr) => {{
            let mut cnt: u32 = 0;
            while cnt < match_max && (s_chr != 0 && (($cond) != (match_not != 0))) {
                s_end = s_tmp;
                let (c, t) = skp_next_pos(src, s_end, *flg & 2);
                s_chr = c;
                s_tmp = t;
                cnt += 1;
            }
            ret = if cnt >= match_min {
                MATCHED_I
            } else {
                MATCHED_FAIL_I
            };
        }};
    }

    macro_rules! get_next_byte {
        () => {{
            s_end = s_tmp;
            s_chr = byte_at(src, s_end) as u32;
            s_tmp = s_end + 1;
        }};
    }

    let mut handled = true;

    match pc {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED_I } else { MATCHED_FAIL_I };
            } else {
                w_loop!(s_chr != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED_I;
            } else {
                w_loop!(is_break_u(s_chr));
            }
        }
        b'n' => {
            w_loop!(is_break_u(s_chr));
        }
        b'd' => {
            w_loop!(is_digit_u(s_chr));
        }
        b'x' => {
            w_loop!(is_xdigit_u(s_chr));
        }
        b'a' => {
            w_loop!(is_alpha_u(s_chr));
        }
        b'u' => {
            w_loop!(is_upper_u(s_chr));
        }
        b'l' => {
            w_loop!(is_lower_u(s_chr));
        }
        b's' => {
            w_loop!(is_space_u(s_chr));
        }
        b'w' => {
            w_loop!(is_blank_u(s_chr));
        }
        b'c' => {
            w_loop!(is_ctrl_u(s_chr));
        }
        b'i' => {
            w_loop!(is_idchr_u(s_chr));
        }
        b'@' => {
            // `@` checks that the current source character is alphanumeric
            // (or, with `!`, that it isn't) but does NOT advance the source
            // position. It also sets a goal point so that, if subsequent
            // patterns succeed, the reported match position is the start of
            // the alnum span. This supports idioms such as
            // `D @ 'cm\xEmm\xEpt'` (a decimal number followed by a unit
            // suffix that must be one of cm/mm/pt).
            let ok = is_alnum_u(s_chr);
            if ok != (match_not != 0) && s_chr != 0 {
                ret = if match_not != 0 {
                    MATCHED_GOALNOT_I
                } else {
                    MATCHED_GOAL_I
                };
            } else if match_min == 0 {
                ret = MATCHED_I;
            }
            // Do not advance s_end.
        }
        b'&' => {
            ret = if match_not != 0 {
                MATCHED_GOALNOT_I
            } else {
                MATCHED_GOAL_I
            };
        }
        b'[' => {
            let set_start = p;
            w_loop!(is_oneof_pos(pat, set_start, s_chr, *flg & 2));
            if byte_at(pat, p) == b']' {
                p += 1;
            }
            while byte_at(pat, p) != 0 && byte_at(pat, p) != b']' {
                p += 1;
            }
            if byte_at(pat, p) != 0 {
                p += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = pc;
            let mut l: usize = 0;
            while byte_at(pat, p + l) != 0 && byte_at(pat, p + l) != quote {
                l += 1;
            }
            let mut did_match = false;
            if l > 0 {
                let ml = is_string_pos(src, s_end, pat, p, l as i32, *flg);
                if ml > 0 {
                    did_match = true;
                    if match_not == 0 {
                        s_end = (s_end as i32 + ml) as usize;
                        ret = MATCHED_I;
                    }
                }
            }
            if !did_match && (match_min == 0 || match_not != 0) {
                ret = MATCHED_I;
            }
            p += l + 1;
        }
        b'C' => {
            *flg = (*flg & !1) | (match_not as i32);
            ret = MATCHED_I;
        }
        b'U' => {
            *flg = (*flg & !2) | ((match_not as i32) * 2);
            ret = MATCHED_I;
        }
        b'S' => {
            while is_space_u(s_chr) {
                get_next_byte!();
            }
            ret = MATCHED_I;
        }
        b'W' => {
            while is_blank_u(s_chr) {
                get_next_byte!();
            }
            ret = MATCHED_I;
        }
        b'N' => {
            while s_chr != 0 && !is_break_u(s_chr) {
                get_next_byte!();
            }
            if s_chr != 0 {
                get_next_byte!();
            }
            ret = MATCHED_I;
        }
        b'I' => {
            if is_alpha_u(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next_byte!();
                    if !(is_alnum_u(s_chr) || s_chr == b'_' as u32) {
                        break;
                    }
                }
                ret = MATCHED_I;
            }
        }
        b'(' => {
            // matches only "(" with following ")"; if both present, advance past pat ')' and fall through to balanced
            if byte_at(pat, p) != b')' || s_chr != b'(' as u32 {
                // do nothing; ret remains FAIL
            } else {
                p += 1;
                // Balanced
                let open = s_chr;
                let close = get_close_u(open);
                if close != 0 {
                    let mut count: i32 = 1;
                    while s_chr != 0 && count > 0 {
                        get_next_byte!();
                        if s_chr == open {
                            count += 1;
                        }
                        if s_chr == close {
                            count -= 1;
                        }
                    }
                    if count == 0 {
                        get_next_byte!();
                        ret = MATCHED_I;
                    }
                }
            }
        }
        b'B' => {
            let open = s_chr;
            let close = get_close_u(open);
            if close != 0 {
                let mut count: i32 = 1;
                while s_chr != 0 && count > 0 {
                    get_next_byte!();
                    if s_chr == open {
                        count += 1;
                    }
                    if s_chr == close {
                        count -= 1;
                    }
                }
                if count == 0 {
                    get_next_byte!();
                    ret = MATCHED_I;
                }
            }
        }
        b'Q' => {
            let qclose = get_qclose_u(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next_byte!();
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == b'\\' as u32 {
                        get_next_byte!();
                    }
                }
                if s_chr != 0 {
                    get_next_byte!();
                    ret = MATCHED_I;
                }
            }
        }
        b'X' => {
            if s_chr == b'0' as u32
                && (byte_at(src, s_end + 1) == b'x' || byte_at(src, s_end + 1) == b'X')
                && is_xdigit_u(byte_at(src, s_end + 2) as u32)
            {
                get_next_byte!();
                get_next_byte!();
                get_next_byte!();
                ret = MATCHED_I;
            }
            while is_xdigit_u(s_chr) {
                ret = MATCHED_I;
                get_next_byte!();
            }
        }
        b'D' | b'F' => {
            intnumber = pc == b'D';
            // sign
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next_byte!();
                    if !is_space_u(s_chr) {
                        break;
                    }
                }
            }
            while is_digit_u(s_chr) {
                ret = MATCHED_I;
                get_next_byte!();
            }
            if !intnumber {
                if s_chr == b'.' as u32 {
                    get_next_byte!();
                }
                while is_digit_u(s_chr) {
                    ret = MATCHED_I;
                    get_next_byte!();
                }
                if ret == MATCHED_I && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                    get_next_byte!();
                    if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                        get_next_byte!();
                    }
                    while is_digit_u(s_chr) {
                        get_next_byte!();
                    }
                    if s_chr == b'.' as u32 {
                        get_next_byte!();
                    }
                    while is_digit_u(s_chr) {
                        get_next_byte!();
                    }
                }
            }
        }
        _ => {
            handled = false;
            ret = MATCHED_FAIL_I;
            // back out the pat increment
            if p > 0 {
                p -= 1;
            }
        }
    }
    let _ = handled;
    let _ = intnumber;

    if ret != MATCHED_FAIL_I {
        Some((ret, s_end, p))
    } else {
        None
    }
}

fn skp_impl(src: &[u8], pat: &[u8]) -> (i32, usize, usize) {
    let mut start: usize = 0;
    let mut s: usize = 0;
    let mut p: usize = 0;
    let mut skp_to_flag = false;
    // matched (returned-from-match): 0 means failure, otherwise 1/2/3 per match kind
    let mut matched: i32 = 0;
    // had_real_match: whether at least one MATCHED_I was returned during this attempt
    let mut had_real_match = false;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    if pat.is_empty() {
        return (0, 0, 0);
    }

    if byte_at(pat, p) == b'>' {
        skp_to_flag = true;
        p += 1;
    }

    let pat_start = p;

    while is_space_u(byte_at(pat, p) as u32) {
        p += 1;
    }

    while byte_at(pat, p) > 7 {
        let s_before = s;
        match match_pat_pos(pat, p, src, s, &mut flg) {
            Some((m, new_s_end, new_p_end)) => {
                matched = m;
                s = new_s_end;
                p = new_p_end;
                if matched == MATCHED_I {
                    had_real_match = true;
                } else if matched == MATCHED_GOAL_I && goalnot.is_none() {
                    // Goal is set at the position when the goal-marker matched.
                    // For `&`, s_before == s (no advance). For `@`, the goal
                    // position is s_before (start of the alnum span).
                    goal = Some(s_before);
                } else if matched == MATCHED_GOALNOT_I {
                    goalnot = Some(s_before);
                }
            }
            None => {
                matched = 0;
                while byte_at(pat, p) > 7 {
                    p += 1;
                }
                if byte_at(pat, p) > 0 && byte_at(pat, p + 1) > 0 {
                    s = start;
                    had_real_match = false;
                    goal = None;
                    p += 1;
                } else if skp_to_flag {
                    goal = None;
                    goalnot = None;
                    had_real_match = false;
                    p = pat_start;
                    start += 1;
                    s = start;
                    if start >= src.len() || byte_at(src, start) == 0 {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        while is_space_u(byte_at(pat, p) as u32) {
            p += 1;
        }
    }

    let mut p_simulated_zero = false;
    if matched == 0 {
        if let Some(gn) = goalnot {
            goal = Some(gn);
            matched = MATCHED_I;
            had_real_match = true;
            p_simulated_zero = true;
        }
    }

    if let Some(g) = goal {
        s = g;
    }

    let p_byte = if p_simulated_zero { 0 } else { byte_at(pat, p) };

    // Require at least one "real" (non-goal-only) match for the call to
    // succeed. Otherwise a pattern that consists only of goal markers (`&`)
    // would erroneously be reported as a successful match.
    if matched != 0 && had_real_match && p_byte <= 7 {
        let ret = if p_byte > 0 { p_byte as i32 } else { 1 };
        return (ret, start, s);
    }

    (0, usize::MAX, usize::MAX)
}

// ---------------------------------------------------------------------------
// Public API matching the Rust signatures
// ---------------------------------------------------------------------------

/// The core scanning function from the C header.
///
/// Returns `(match_code, to, end)`. The slice conventions are:
///  - When the pattern starts with `>` (skp_to mode), `to` is the matched
///    substring (`&src[start..s]`) and `end` is the full source slice.
///    Then `end.len() - to.len()` gives the post-match remainder.
///  - Otherwise, both `to` and `end` are `&src[s..]` (the source after
///    the match).
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    let (ret, start_pos, s_pos) = skp_impl(src.as_bytes(), pat.as_bytes());
    if start_pos == usize::MAX {
        return (ret, src, src);
    }
    let skp_to_flag = !pat.is_empty() && pat.as_bytes()[0] == b'>';
    if skp_to_flag {
        let to_str = src.get(start_pos..s_pos).unwrap_or("");
        let end_str = src;
        (ret, to_str, end_str)
    } else {
        let post = src.get(s_pos..).unwrap_or("");
        (ret, post, post)
    }
}

pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    let (ret, to_str, end_str) = skp_(src, pat);
    if let Some(t) = to {
        let s: &str = unsafe { std::mem::transmute(to_str) };
        *t = s;
    }
    if let Some(e) = end {
        let s: &str = unsafe { std::mem::transmute(end_str) };
        *e = s;
    }
    ret
}

pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    skp_4(src, pat, None, end)
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_4(src, pat, None, None)
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, pos) = skp_next_pos(bytes, 0, iso);
    let rest = s.get(pos..).unwrap_or("");
    (c, rest)
}

pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    chr_cmp_u(a, b, fold)
}

pub fn is_blank(c: u32) -> bool {
    is_blank_u(c)
}

pub fn is_break(c: u32) -> bool {
    is_break_u(c)
}

pub fn is_space(c: u32) -> bool {
    is_space_u(c)
}

pub fn is_digit(c: u32) -> bool {
    is_digit_u(c)
}

pub fn is_xdigit(c: u32) -> bool {
    is_xdigit_u(c)
}

pub fn is_upper(c: u32) -> bool {
    is_upper_u(c)
}

pub fn is_lower(c: u32) -> bool {
    is_lower_u(c)
}

pub fn is_alpha(c: u32) -> bool {
    is_alpha_u(c)
}

pub fn is_idchr(c: u32) -> bool {
    is_idchr_u(c)
}

pub fn is_alnum(c: u32) -> bool {
    is_alnum_u(c)
}

pub fn is_ctrl(c: u32) -> bool {
    is_ctrl_u(c)
}

pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    is_oneof_pos(set.as_bytes(), 0, ch, iso)
}

pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_pos(s.as_bytes(), 0, p.as_bytes(), 0, len, flg)
}

pub fn get_close(open: u32) -> u32 {
    get_close_u(open)
}

pub fn get_qclose(open: u32) -> u32 {
    get_qclose_u(open)
}

/// Constants for match results.
pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

/// Matches the pattern `pat` against source `src` and returns a tuple:
/// `(match_result, src_end, pat_end)`.
pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    match match_pat_pos(pat.as_bytes(), 0, src.as_bytes(), 0, flg) {
        Some((ret, s_end, p_end)) => {
            let s = src.get(s_end..).unwrap_or("");
            let p = pat.get(p_end..).unwrap_or("");
            (ret, s, p)
        }
        None => (MATCHED_FAIL, src, pat),
    }
}

// ---------------------------------------------------------------------------
//
// AST Parsing Functions and Types
//
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

const ASTNULL: AstNodeT = -1;
const SKP_DEBUG: i8 = 0x01;

pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let initial_pos = ast.pos;
    let open = ast_open(&mut ast, initial_pos, rulename);
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
    ast.start.get((ast.err_pos as usize)..)
}

pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let bytes = ast.start.as_bytes();
    let mut i = ast.err_pos as usize;
    while i > 0 {
        let prev = bytes[i - 1];
        if prev == b'\n' || prev == b'\r' {
            break;
        }
        i -= 1;
    }
    ast.start.get(i..).unwrap_or("")
}

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line_start_byte = {
        let bytes = ast.start.as_bytes();
        let mut i = ast.err_pos as usize;
        while i > 0 {
            let prev = bytes[i - 1];
            if prev == b'\n' || prev == b'\r' {
                break;
            }
            i -= 1;
        }
        i as i32
    };
    ast.err_pos - line_start_byte
}

pub fn ast_new() -> Option<Ast> {
    let ast = Ast {
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
    };
    Some(ast)
}

pub fn astfree(_ast: Ast) -> Option<Ast> {
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
    let node_idx = ast.par[open as usize] as usize;

    if ast.fail != 0 {
        ast.pos = ast.nodes[node_idx].from;
        ast.nodes_cnt = ast.par[open as usize];
        ast.par_cnt = open;
        // truncate vectors so they remain consistent
        ast.nodes.truncate(ast.nodes_cnt as usize);
        ast.par.truncate(ast.par_cnt as usize);
        return -1;
    }

    let par = ast.par_cnt;
    ast.par.push(0);
    ast.par_cnt += 1;

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
    ast.err_msg = Some(msg.to_string());
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
}

pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // Memoization is performance optimization; safe to no-op for tests.
}

pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    if ast.par_cnt <= node && node != ASTNULL {
        return;
    }
    let mut node = if node == ASTNULL {
        ast.par_cnt - 1
    } else {
        node
    };
    if node < 0 || node >= ast.par_cnt {
        return;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || node >= ast.par_cnt {
        return;
    }
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        ast.nodes[idx].tag = info;
    }
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail != 0 {
        return;
    }
    let pos = ast.pos;
    let par = ast_open(ast, pos, "#");
    ast_close(ast, pos, par);
    if par >= 0 && (par as usize) < ast.par.len() {
        let idx = ast.par[par as usize] as usize;
        if idx < ast.nodes.len() {
            ast.nodes[idx].tag = info;
        }
    }
    ast.lastinfo = info;
}

pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
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
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        ast.nodes[idx].tag
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
    let o2 = o2 as usize;
    let c2 = c2 as usize;
    let o1 = o1 as usize;
    let c1 = c1 as usize;
    let block_a: Vec<i32> = ast.par[o2..=c2].to_vec();
    let block_b: Vec<i32> = ast.par[o1..=c1].to_vec();
    let mut i = o2;
    for v in &block_b {
        ast.par[i] = *v;
        i += 1;
    }
    for v in &block_a {
        ast.par[i] = *v;
        i += 1;
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
    let lft_node_idx = ast.par[lft as usize] as usize;
    let rgt_node_idx = ast.par[rgt as usize] as usize;
    let node_from = ast.nodes[lft_node_idx].from;
    let node_to = ast.nodes[rgt_node_idx].to;
    let rgt = rgt + ast.nodes[rgt_node_idx].delta;
    let new_node_idx = ast.nodes_cnt;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta: rgt - lft + 2,
        tag: 0,
    });
    ast.nodes_cnt += 1;
    let delta = rgt - lft + 2;
    ast.par.push(0);
    ast.par.push(0);
    ast.par_cnt += 2;
    let total = ast.par_cnt;
    // Move pars after rgt by 2
    if total - 1 - rgt > 2 {
        let cnt = (total - 1 - rgt - 2) as usize;
        for i in (0..cnt).rev() {
            ast.par[(rgt + 3) as usize + i] = ast.par[(rgt + 1) as usize + i];
        }
    }
    // Shift block [lft..=rgt] by 1
    let cnt = (rgt - lft + 1) as usize;
    for i in (0..cnt).rev() {
        ast.par[(lft + 1) as usize + i] = ast.par[lft as usize + i];
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
    let o1u = o1 as usize;
    if ast.nodes[ast.par[o1u] as usize].tag == 0 {
        let cnt = (c2 - o2 + 1) as usize;
        for i in 0..cnt {
            ast.par[o1u + i] = ast.par[o2 as usize + i];
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
    let nidx = ast.par[o1 as usize] as usize;
    if ast.nodes[nidx].from != ast.nodes[nidx].to {
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
    let mut node = node;
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
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
    let mut node = node;
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
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
        let n = astleft(ast, cur);
        if n == ASTNULL {
            break;
        }
        cur = n;
    }
    cur
}

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
    node >= 0 && node < ast.par_cnt && ast.par[node as usize] >= 0
}

pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node >= 0 && node < ast.par_cnt && ast.par[node as usize] < 0
}

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
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        &ast.nodes[idx].rule
    } else {
        ""
    }
}

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
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        let f = ast.nodes[idx].from as usize;
        ast.start.get(f..).unwrap_or("")
    } else {
        ""
    }
}

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
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        let t = ast.nodes[idx].to as usize;
        ast.start.get(t..).unwrap_or("")
    } else {
        ""
    }
}

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
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        ast.nodes[idx].to - ast.nodes[idx].from
    } else {
        0
    }
}

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
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() {
        ast.nodes[idx].delta == 1
    } else {
        false
    }
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
    if node == ASTNULL || node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut node = node;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 || (node as usize) >= ast.par.len() {
        return 0;
    }
    let idx = ast.par[node as usize] as usize;
    if idx < ast.nodes.len() && ast.nodes[idx].rule == rulename {
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
                let from = astnodefrom(ast, node);
                let to = astnodeto(ast, node);
                let from_b = from.as_bytes();
                let to_b = to.as_bytes();
                let len = from_b.len().saturating_sub(to_b.len());
                let s = &from_b[..len];
                for &c in s {
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
                let from = astnodefrom(ast, node);
                let to = astnodeto(ast, node);
                let from_b = from.as_bytes();
                let to_b = to.as_bytes();
                let len = from_b.len().saturating_sub(to_b.len());
                let s = &from_b[..len];
                for &c in s {
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
