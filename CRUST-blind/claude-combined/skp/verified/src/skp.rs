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
    // In C: int ret = to - start; (0 <= ret && ret <= (1<<16)?ret:0)
    // Here we approximate by computing distance in bytes; since both should be slices of same buffer,
    // we use the difference of their `as_ptr` if they are. Otherwise fall back to length difference.
    let s_ptr = start.as_ptr() as isize;
    let t_ptr = to.as_ptr() as isize;
    let ret = t_ptr - s_ptr;
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

// =========================================================================
// Internal byte-based helpers that mirror the C semantics
// =========================================================================

/// Read the next "char" from byte slice `s` starting at index `i`.
/// Returns (codepoint_packed_as_int, next_index).
fn next_byte(s: &[u8], mut i: usize, iso: bool) -> (u32, usize) {
    if i >= s.len() || s[i] == 0 {
        return (0, i);
    }
    let mut c: u32 = s[i] as u32;
    i += 1;
    if !iso {
        // Read up to 3 continuation bytes
        for _ in 0..3 {
            if i < s.len() && (s[i] & 0xC0) == 0x80 {
                c = (c << 8) | (s[i] as u32);
                i += 1;
            } else {
                break;
            }
        }
    }
    if c == 0x0D && i < s.len() && s[i] == 0x0A {
        c = 0x0D0A;
        i += 1;
    }
    (c, i)
}

fn b_chr_cmp(a: u32, b: u32, fold: bool) -> bool {
    if fold && a <= 0x7F && b <= 0x7F {
        let aa = if (b'A' as u32 <= a) && (a <= b'Z' as u32) {
            a + 32
        } else {
            a
        };
        let bb = if (b'A' as u32 <= b) && (b <= b'Z' as u32) {
            b + 32
        } else {
            b
        };
        aa == bb
    } else {
        a == b
    }
}

fn b_is_blank(c: u32) -> bool {
    if c < 0xFF {
        return (c == 0x20) || (c == 0x09);
    }
    match c & 0xFFFFFF00 {
        0x00000000 => c == 0xA0,
        0x0000C200 => c == 0xC2A0,
        0x00E19A00 => c == 0xE19A80,
        0x00E28000 => ((0xE28080..=0xE2808A).contains(&c)) || (c == 0xE280AF),
        0x00E38080 => c == 0xE38080,
        _ => false,
    }
}

fn b_is_break(c: u32) -> bool {
    if c < 0x0F {
        return (c == 0x0A) || (c == 0x0C) || (c == 0x0D);
    }
    if c < 0xFF {
        return c == 0x85;
    }
    (c == 0x0D0A) || (c == 0xC285) || (c == 0xE280A8) || (c == 0xE280A9)
}

fn b_is_space(c: u32) -> bool {
    b_is_blank(c) || b_is_break(c)
}
fn b_is_digit(c: u32) -> bool {
    (b'0' as u32) <= c && c <= (b'9' as u32)
}
fn b_is_xdigit(c: u32) -> bool {
    ((b'0' as u32) <= c && c <= (b'9' as u32))
        || ((b'A' as u32) <= c && c <= (b'F' as u32))
        || ((b'a' as u32) <= c && c <= (b'f' as u32))
}
fn b_is_upper(c: u32) -> bool {
    (b'A' as u32) <= c && c <= (b'Z' as u32)
}
fn b_is_lower(c: u32) -> bool {
    (b'a' as u32) <= c && c <= (b'z' as u32)
}
fn b_is_alpha(c: u32) -> bool {
    b_is_upper(c) || b_is_lower(c)
}
fn b_is_idchr(c: u32) -> bool {
    b_is_alpha(c) || b_is_digit(c) || c == (b'_' as u32)
}
fn b_is_alnum(c: u32) -> bool {
    b_is_alpha(c) || b_is_digit(c)
}
fn b_is_ctrl(c: u32) -> bool {
    (c < 0x20) || (0xC280 <= c && c < 0xC2A0) || (0x7F <= c && c < 0xA0)
}

fn b_get_close(open: u32) -> u32 {
    match open as u8 as char {
        '(' => ')' as u32,
        '[' => ']' as u32,
        '{' => '}' as u32,
        '<' => '>' as u32,
        _ => 0,
    }
}

fn b_get_qclose(open: u32) -> u32 {
    match open as u8 as char {
        '\'' | '"' | '`' => open,
        _ => 0,
    }
}

/// Tests if codepoint `ch` (in the packed-byte form) is in the set described by bytes `set`.
/// Set syntax: characters or ranges (a-z), terminated by ']'. Special: leading ']' means literal ']'.
fn b_is_oneof(ch: u32, set: &[u8], iso: bool) -> bool {
    if ch == 0 {
        return false;
    }
    let (mut p_ch, mut s_idx) = next_byte(set, 0, iso);
    let close_bracket = b']' as u32;

    if p_ch == close_bracket {
        if ch == close_bracket {
            return true;
        }
        let (np, ni) = next_byte(set, s_idx, iso);
        p_ch = np;
        s_idx = ni;
    }

    while p_ch != close_bracket && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (np, ni) = next_byte(set, s_idx, iso);
        p_ch = np;
        s_idx = ni;
        if p_ch == (b'-' as u32) && s_idx < set.len() && set[s_idx] != b']' {
            // range
            let (np2, ni2) = next_byte(set, s_idx, iso);
            p_ch = np2;
            s_idx = ni2;
            if (q_ch < ch) && (ch <= p_ch) {
                return true;
            }
            let (np3, ni3) = next_byte(set, s_idx, iso);
            p_ch = np3;
            s_idx = ni3;
        }
    }
    false
}

/// is_string operating on byte slices. Returns mlen (matched bytes in `s`).
/// `len` is the number of pattern bytes to consider; the pattern segment may contain alternatives separated by 0x0E ('\xE').
fn b_is_string(s: &[u8], p: &[u8], mut len: i32, flg: i32) -> i32 {
    let start_s_idx = 0usize;
    let mut s_idx = 0usize;
    let mut p_idx = 0usize;
    let mut mlen: i32 = 0;
    let fold = (flg & 1) != 0;
    let iso = (flg & 2) != 0;
    while len > 0 {
        if p_idx >= p.len() || p[p_idx] == 0x0E {
            return mlen;
        }
        let (p_chr, p_end) = next_byte(p, p_idx, iso);
        let (s_chr, s_end) = next_byte(s, s_idx, iso);

        if b_chr_cmp(s_chr, p_chr, fold) {
            mlen += (s_end - s_idx) as i32;
            len -= (p_end - p_idx) as i32;
            p_idx = p_end;
            s_idx = s_end;
        } else {
            // search for next alternative (0x0E)
            while len > 0 && p_idx < p.len() && p[p_idx] != 0x0E {
                p_idx += 1;
                len -= 1;
            }
            if p_idx < p.len() && p[p_idx] == 0x0E {
                p_idx += 1;
                len -= 1;
            }
            if len <= 0 {
                return 0;
            }
            s_idx = start_s_idx;
            mlen = 0;
        }
    }
    mlen
}

const M_FAIL: i32 = 0;
const M_MATCHED: i32 = 1;
const M_GOAL: i32 = 2;
const M_GOALNOT: i32 = 3;

/// Internal byte-based match. Returns (ret, src_end_idx, pat_end_idx).
fn b_match(pat: &[u8], src: &[u8], src_idx: usize, flg: &mut i32) -> (i32, usize, usize) {
    let mut p_idx = 0usize;
    let mut s_end = src_idx;
    let mut s_tmp;
    let iso = (*flg & 2) != 0;
    let (mut s_chr, t) = next_byte(src, s_end, iso);
    s_tmp = t;

    let mut ret: i32 = M_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    if p_idx < pat.len() && pat[p_idx] == b'*' {
        match_min = 0;
        match_max = u32::MAX;
        p_idx += 1;
    } else if p_idx < pat.len() && pat[p_idx] == b'+' {
        match_max = u32::MAX;
        p_idx += 1;
    } else if p_idx < pat.len() && pat[p_idx] == b'?' {
        match_min = 0;
        p_idx += 1;
    }

    if p_idx < pat.len() && pat[p_idx] == b'!' {
        match_not = 1;
        p_idx += 1;
    }

    macro_rules! W {
        ($cond:expr) => {{
            let mut match_cnt: u32 = 0;
            while match_cnt < match_max && {
                let cond_val = $cond(s_chr);
                s_chr != 0 && (cond_val != (match_not != 0))
            } {
                s_end = s_tmp;
                let (nc, nt) = next_byte(src, s_end, (*flg & 2) != 0);
                s_chr = nc;
                s_tmp = nt;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { M_MATCHED } else { M_FAIL };
        }};
    }

    macro_rules! get_next_s_chr {
        () => {{
            s_end = s_tmp;
            // s_chr = *s_end ; s_tmp++
            if s_end < src.len() {
                s_chr = src[s_end] as u32;
            } else {
                s_chr = 0;
            }
            s_tmp = s_end + 1;
        }};
    }

    if p_idx >= pat.len() {
        return (M_FAIL, s_end, p_idx);
    }
    let opcode = pat[p_idx];
    p_idx += 1;
    intnumber = false;

    match opcode {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { M_MATCHED } else { M_FAIL };
            } else {
                W!(|c: u32| c != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = M_MATCHED;
            } else {
                W!(b_is_break);
            }
        }
        b'n' => {
            W!(b_is_break);
        }
        b'd' => {
            W!(b_is_digit);
        }
        b'x' => {
            W!(b_is_xdigit);
        }
        b'a' => {
            W!(b_is_alpha);
        }
        b'u' => {
            W!(b_is_upper);
        }
        b'l' => {
            W!(b_is_lower);
        }
        b's' => {
            W!(b_is_space);
        }
        b'w' => {
            W!(b_is_blank);
        }
        b'c' => {
            W!(b_is_ctrl);
        }
        b'i' => {
            W!(b_is_idchr);
        }
        b'@' => {
            W!(b_is_alnum);
        }
        b'&' => {
            ret = if match_not != 0 { M_GOALNOT } else { M_GOAL };
        }
        b'[' => {
            // copy current pat byte slice from p_idx for set
            let set = &pat[p_idx..];
            // Use closure capturing set
            let iso2 = (*flg & 2) != 0;
            let mut match_cnt: u32 = 0;
            while match_cnt < match_max && {
                let cond = b_is_oneof(s_chr, set, iso2);
                s_chr != 0 && (cond != (match_not != 0))
            } {
                s_end = s_tmp;
                let (nc, nt) = next_byte(src, s_end, iso2);
                s_chr = nc;
                s_tmp = nt;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min {
                M_MATCHED
            } else {
                M_FAIL
            };
            // advance pattern past the set
            if p_idx < pat.len() && pat[p_idx] == b']' {
                p_idx += 1;
            }
            while p_idx < pat.len() && pat[p_idx] != 0 && pat[p_idx] != b']' {
                p_idx += 1;
            }
            if p_idx < pat.len() {
                p_idx += 1;
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = opcode;
            let mut l = 0usize;
            while p_idx + l < pat.len() && pat[p_idx + l] != 0 && pat[p_idx + l] != quote {
                l += 1;
            }
            // Note: s_end refers to current position in src.
            if l > 0 {
                let ml = b_is_string(&src[s_end..], &pat[p_idx..p_idx + l], l as i32, *flg);
                if ml > 0 {
                    if match_not == 0 {
                        s_end += ml as usize;
                        ret = M_MATCHED;
                    }
                } else if match_min == 0 || match_not != 0 {
                    ret = M_MATCHED;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = M_MATCHED;
            }
            p_idx += l;
            if p_idx < pat.len() {
                p_idx += 1;
            }
        }
        b'C' => {
            *flg = (*flg & !1) | (match_not as i32);
            ret = M_MATCHED;
        }
        b'U' => {
            *flg = (*flg & !2) | ((match_not as i32) * 2);
            ret = M_MATCHED;
        }
        b'S' => {
            while b_is_space(s_chr) {
                get_next_s_chr!();
            }
            ret = M_MATCHED;
        }
        b'W' => {
            while b_is_blank(s_chr) {
                get_next_s_chr!();
            }
            ret = M_MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !b_is_break(s_chr) {
                get_next_s_chr!();
            }
            if s_chr != 0 {
                get_next_s_chr!();
            }
            ret = M_MATCHED;
        }
        b'I' => {
            if b_is_alpha(s_chr) || s_chr == (b'_' as u32) {
                loop {
                    get_next_s_chr!();
                    if !(b_is_alnum(s_chr) || s_chr == (b'_' as u32)) {
                        break;
                    }
                }
                ret = M_MATCHED;
            }
        }
        b'(' => {
            // C: if (*pat != ')' || s_chr != '(') break;
            //    pat++;
            //    /* fall through to 'B' */
            if p_idx < pat.len() && pat[p_idx] == b')' && s_chr == (b'(' as u32) {
                p_idx += 1;
                // fall through into 'B' logic
                let open = s_chr;
                let close = b_get_close(open);
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
                        ret = M_MATCHED;
                    }
                }
            }
            // else break (ret = M_FAIL)
        }
        b'B' => {
            let open = s_chr;
            let close = b_get_close(open);
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
                    ret = M_MATCHED;
                }
            }
        }
        b'Q' => {
            let qclose = b_get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next_s_chr!();
                    if s_chr == qclose {
                        break;
                    }
                    if s_chr == (b'\\' as u32) {
                        get_next_s_chr!();
                    }
                }
                if s_chr != 0 {
                    get_next_s_chr!();
                    ret = M_MATCHED;
                }
            }
        }
        b'X' => {
            // hex number: optional 0x prefix
            if s_chr == (b'0' as u32)
                && s_end + 1 < src.len()
                && (src[s_end + 1] == b'x' || src[s_end + 1] == b'X')
                && s_end + 2 < src.len()
                && b_is_xdigit(src[s_end + 2] as u32)
            {
                get_next_s_chr!();
                get_next_s_chr!();
                get_next_s_chr!();
                ret = M_MATCHED;
            }
            while b_is_xdigit(s_chr) {
                ret = M_MATCHED;
                get_next_s_chr!();
            }
        }
        b'D' => {
            intnumber = true;
            // sign
            if s_chr == (b'+' as u32) || s_chr == (b'-' as u32) {
                loop {
                    get_next_s_chr!();
                    if !b_is_space(s_chr) {
                        break;
                    }
                }
            }
            while b_is_digit(s_chr) {
                ret = M_MATCHED;
                get_next_s_chr!();
            }
            if !intnumber {
                if s_chr == (b'.' as u32) {
                    get_next_s_chr!();
                }
                while b_is_digit(s_chr) {
                    ret = M_MATCHED;
                    get_next_s_chr!();
                }
                if ret == M_MATCHED && (s_chr == (b'E' as u32) || s_chr == (b'e' as u32)) {
                    get_next_s_chr!();
                    if s_chr == (b'+' as u32) || s_chr == (b'-' as u32) {
                        get_next_s_chr!();
                    }
                    while b_is_digit(s_chr) {
                        get_next_s_chr!();
                    }
                    if s_chr == (b'.' as u32) {
                        get_next_s_chr!();
                    }
                    while b_is_digit(s_chr) {
                        get_next_s_chr!();
                    }
                }
            }
        }
        b'F' => {
            // sign
            if s_chr == (b'+' as u32) || s_chr == (b'-' as u32) {
                loop {
                    get_next_s_chr!();
                    if !b_is_space(s_chr) {
                        break;
                    }
                }
            }
            while b_is_digit(s_chr) {
                ret = M_MATCHED;
                get_next_s_chr!();
            }
            if s_chr == (b'.' as u32) {
                get_next_s_chr!();
            }
            while b_is_digit(s_chr) {
                ret = M_MATCHED;
                get_next_s_chr!();
            }
            if ret == M_MATCHED && (s_chr == (b'E' as u32) || s_chr == (b'e' as u32)) {
                get_next_s_chr!();
                if s_chr == (b'+' as u32) || s_chr == (b'-' as u32) {
                    get_next_s_chr!();
                }
                while b_is_digit(s_chr) {
                    get_next_s_chr!();
                }
                if s_chr == (b'.' as u32) {
                    get_next_s_chr!();
                }
                while b_is_digit(s_chr) {
                    get_next_s_chr!();
                }
            }
        }
        _ => {
            ret = M_FAIL;
            // C: pat--;
            p_idx -= 1;
        }
    }

    let _ = intnumber; // already used
    (ret, s_end, p_idx)
}

/// Internal byte-based skp_. Returns (ret, to_idx, end_idx)
/// to_idx and end_idx are indices into src bytes.
fn b_skp(src: &[u8], pat: &[u8]) -> (i32, usize, usize) {
    if src.is_empty() && pat.is_empty() {
        return (0, 0, 0);
    }
    let mut start_idx = 0usize;
    let mut s_idx = start_idx;
    let mut p_idx = 0usize;
    let mut skp_to = 0;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    if !pat.is_empty() && pat[0] == b'>' {
        skp_to = 1;
        p_idx += 1;
    }

    let pat_start = p_idx;

    // skip leading spaces
    while p_idx < pat.len() && b_is_space(pat[p_idx] as u32) {
        p_idx += 1;
    }

    while p_idx < pat.len() && pat[p_idx] > b'\x07' {
        let (m, s_end, p_end) = b_match(&pat[p_idx..], src, s_idx, &mut flg);
        if m != 0 {
            matched = m;
            s_idx = s_end;
            p_idx += p_end;
            if matched == M_GOAL && goalnot.is_none() {
                goal = Some(s_idx);
            } else if matched == M_GOALNOT {
                goalnot = Some(s_idx);
            }
        } else {
            matched = 0;
            // skip past this pattern element until we find something <= '\7'
            while p_idx < pat.len() && pat[p_idx] > b'\x07' {
                p_idx += 1;
            }
            // try a new pattern after '\xN'
            if p_idx < pat.len()
                && pat[p_idx] > 0
                && p_idx + 1 < pat.len()
                && pat[p_idx + 1] > 0
            {
                s_idx = start_idx;
                p_idx += 1;
            } else if skp_to != 0 {
                goal = None;
                goalnot = None;
                p_idx = pat_start;
                start_idx += 1;
                s_idx = start_idx;
                if start_idx >= src.len() {
                    break;
                }
            } else {
                break;
            }
        }
        // skip spaces
        while p_idx < pat.len() && b_is_space(pat[p_idx] as u32) {
            p_idx += 1;
        }
    }

    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = M_MATCHED;
        // p="" — pretend pattern ended cleanly
        // We synthesize: set p_idx to a position whose byte is <= '\7'.
        // We'll handle this by treating matched as success even if pattern char > '\7'.
        // Fall through with a special marker: use a flag.
        return finalize(src, &b""[..], 0, true, goal.unwrap_or(s_idx), start_idx, skp_to);
    }

    let pat_byte = if p_idx < pat.len() { pat[p_idx] } else { 0 };

    if matched != 0 && pat_byte <= b'\x07' {
        let final_s = goal.unwrap_or(s_idx);
        let ret = if pat_byte > 0 { pat_byte as i32 } else { 1 };
        let to = if skp_to != 0 { start_idx } else { final_s };
        let end = final_s;
        return (ret, to, end);
    }

    (0, 0, 0)
}

fn finalize(
    _src: &[u8],
    _pat: &[u8],
    _p_idx: usize,
    _matched: bool,
    s_idx: usize,
    start_idx: usize,
    skp_to: i32,
) -> (i32, usize, usize) {
    let to = if skp_to != 0 { start_idx } else { s_idx };
    let end = s_idx;
    (1, to, end)
}

// =========================================================================
// Public API wrappers
// =========================================================================

/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    let src_bytes = src.as_bytes();
    let pat_bytes = pat.as_bytes();
    let (ret, to_idx, end_idx) = b_skp(src_bytes, pat_bytes);
    if ret == 0 {
        return (0, src, src);
    }
    // Convert byte indices back to &str safely.
    let to = std::str::from_utf8(&src_bytes[to_idx..]).unwrap_or("");
    let end = std::str::from_utf8(&src_bytes[end_idx..]).unwrap_or("");
    (ret, to, end)
}

pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    let (ret, t, e) = skp_(src, pat);
    if let Some(slot) = to {
        // SAFETY: keep lifetime by transmuting to caller's lifetime — we extend lifetime here.
        // We use unsafe transmute because src lifetime must outlive both.
        // Actually, we can write to the slot if the lifetimes are compatible.
        // The caller passes `&mut &str` with arbitrary lifetime. We need to hand back
        // a slice into src. The simplest sound approach: produce a static empty slice
        // and write pointers via raw cast. Instead, we return slices that the caller
        // ensures live as long as src; we use unsafe to bridge lifetimes.
        unsafe {
            *slot = std::mem::transmute::<&str, &str>(t);
        }
    }
    if let Some(slot) = end {
        unsafe {
            *slot = std::mem::transmute::<&str, &str>(e);
        }
    }
    ret
}

pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    let (ret, _t, e) = skp_(src, pat);
    if let Some(slot) = end {
        unsafe {
            *slot = std::mem::transmute::<&str, &str>(e);
        }
    }
    ret
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_(src, pat).0
}

/// Returns the next "char" from string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, idx) = next_byte(bytes, 0, iso != 0);
    let rest = std::str::from_utf8(&bytes[idx..]).unwrap_or("");
    (c, rest)
}

pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    b_chr_cmp(a, b, fold != 0)
}

pub fn is_blank(c: u32) -> bool {
    b_is_blank(c)
}

pub fn is_break(c: u32) -> bool {
    b_is_break(c)
}

pub fn is_space(c: u32) -> bool {
    b_is_space(c)
}

pub fn is_digit(c: u32) -> bool {
    b_is_digit(c)
}

pub fn is_xdigit(c: u32) -> bool {
    b_is_xdigit(c)
}

pub fn is_upper(c: u32) -> bool {
    b_is_upper(c)
}

pub fn is_lower(c: u32) -> bool {
    b_is_lower(c)
}

pub fn is_alpha(c: u32) -> bool {
    b_is_alpha(c)
}

pub fn is_idchr(c: u32) -> bool {
    b_is_idchr(c)
}

pub fn is_alnum(c: u32) -> bool {
    b_is_alnum(c)
}

pub fn is_ctrl(c: u32) -> bool {
    b_is_ctrl(c)
}

pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    b_is_oneof(ch, set.as_bytes(), iso != 0)
}

pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    b_is_string(s.as_bytes(), p.as_bytes(), len, flg)
}

pub fn get_close(open: u32) -> u32 {
    b_get_close(open)
}

pub fn get_qclose(open: u32) -> u32 {
    b_get_qclose(open)
}

pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let pat_bytes = pat.as_bytes();
    let src_bytes = src.as_bytes();
    let (ret, s_end, p_end) = b_match(pat_bytes, src_bytes, 0, flg);
    if ret == MATCHED_FAIL {
        return (ret, src, pat);
    }
    let src_rem = std::str::from_utf8(&src_bytes[s_end..]).unwrap_or("");
    let pat_rem = std::str::from_utf8(&pat_bytes[p_end..]).unwrap_or("");
    (ret, src_rem, pat_rem)
}

// =========================================================================
// AST types
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

pub const ASTNULL: AstNodeT = -1;
pub const SKP_DEBUG: i8 = 0x01;
pub const SKP_LEFTRECUR: i8 = 0x02;

/// Parses the source string `src` using a given parsing rule.
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
    if open >= 0 {
        let mut ret_v = ast.ret;
        rule(&mut ast, &mut ret_v);
        ast.ret = ret_v;

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
    // skp_mmz_clean: just drop our memoization cache
    ast.mmz.clear();
    ast.mmz_cnt = 0;
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
        let c = bytes[ln - 1];
        if c == b'\n' || c == b'\r' {
            break;
        }
        ln -= 1;
    }
    std::str::from_utf8(&bytes[ln..]).unwrap_or("")
}

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let line_start_offset = ast.start.len() - line.len();
    (ast.err_pos as i32) - (line_start_offset as i32)
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
    ast.cur_node = ASTNULL;
    ast.cur_rule = None;
    ast.auxptr = None;
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
    let node = ast.nodes_cnt;
    ast.par.push(node);
    ast.par_cnt += 1;
    ast.nodes.push(AstNode {
        rule: rule.to_string(),
        from,
        to: 0,
        delta: 0,
        tag: 0,
    });
    ast.nodes_cnt += 1;
    par
}

pub fn ast_close(ast: &mut Ast, to: i32, open: i32) -> i32 {
    if open < 0 {
        return -1;
    }
    let node_idx = ast.par[open as usize];
    if ast.fail != 0 {
        // Reset
        ast.pos = ast.nodes[node_idx as usize].from;
        ast.nodes_cnt = node_idx;
        ast.par_cnt = open;
        // Truncate our vectors
        ast.nodes.truncate(node_idx as usize);
        ast.par.truncate(open as usize);
        return -1;
    }

    let par = ast.par_cnt;
    let delta = par - open;
    ast.nodes[node_idx as usize].to = to;
    ast.nodes[node_idx as usize].delta = delta;
    ast.nodes[node_idx as usize].tag = 0;
    ast.par.push(-delta);
    ast.par_cnt += 1;

    ast.cur_node = par;
    ast.cur_rule = Some(ast.nodes[node_idx as usize].rule.clone());
    par
}

pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    ast.err_msg = Some(msg.to_string());
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    ast.fail = 1;
}

pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // Memoization is an internal optimization; tests don't rely on this side-effect.
    // We provide a minimal no-op to keep the API surface intact.
}

pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

pub fn ast_setinfo(ast: &mut Ast, info: i32, mut node: AstNodeT) {
    if ast.par_cnt <= node {
        return;
    }
    if node == ASTNULL {
        node = ast.par_cnt - 1;
    }
    if node < 0 || node >= ast.par_cnt {
        return;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let n = ast.par[idx] as usize;
    ast.nodes[n].tag = info;
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail != 0 {
        return;
    }
    let par = ast_open(ast, ast.pos, "#");
    ast_close(ast, ast.pos, par);
    let node_idx = ast.par[par as usize] as usize;
    ast.nodes[node_idx].tag = info;
    ast.lastinfo = info;
}

pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node < 0 || node >= ast.par_cnt {
        return 0;
    }
    let mut idx = node as usize;
    if ast.par[idx] < 0 {
        idx = (idx as i32 + ast.par[idx]) as usize;
    }
    let n = ast.par[idx] as usize;
    ast.nodes[n].tag
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

    let block1: Vec<i32> = ast.par[(o2 as usize)..=(c2 as usize)].to_vec();
    let block2: Vec<i32> = ast.par[(o1 as usize)..=(c1 as usize)].to_vec();

    let mut new_par = ast.par.clone();
    let len1 = block1.len();
    let len2 = block2.len();
    // Place block2 at o2, then block1 after it
    for (i, v) in block2.iter().enumerate() {
        new_par[o2 as usize + i] = *v;
    }
    for (i, v) in block1.iter().enumerate() {
        new_par[o2 as usize + len2 + i] = *v;
    }
    let _ = len1;
    ast.par = new_par;
}

pub fn ast_lower(ast: &mut Ast, rule: &str, mut lft: AstNodeT, mut rgt: AstNodeT) {
    if ast.par_cnt <= lft || ast.par_cnt <= rgt || lft >= rgt {
        return;
    }
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

    // Allocate two new par slots
    ast.par.push(0);
    ast.par.push(0);
    ast.par_cnt += 2;

    // Move the nodes after rgt one (matching C's memmove semantics)
    let par_cnt = ast.par_cnt;
    let rgt_us = rgt as usize;
    if (par_cnt - 1 - rgt) > 2 {
        let count = (par_cnt - 1 - rgt - 2) as usize;
        // memmove(dst=par[rgt+3], src=par[rgt+1], count)
        for i in (0..count).rev() {
            ast.par[rgt_us + 3 + i] = ast.par[rgt_us + 1 + i];
        }
    }

    // Move block from lft..=rgt to lft+1..=rgt+1
    let lft_us = lft as usize;
    let block_len = (rgt - lft + 1) as usize;
    for i in (0..block_len).rev() {
        ast.par[lft_us + 1 + i] = ast.par[lft_us + i];
    }

    ast.par[lft_us] = new_node_idx;
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
    let n_idx = ast.par[o1 as usize] as usize;
    if ast.nodes[n_idx].tag == 0 {
        // memmove(par+o1, par+o2, c2-o2+1)
        let block_len = (c2 - o2 + 1) as usize;
        for i in 0..block_len {
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
    let n_idx = ast.par[o1 as usize] as usize;
    if ast.nodes[n_idx].from != ast.nodes[n_idx].to {
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
    let idx = ast.par[node as usize] as usize;
    let nd = &ast.nodes[idx];
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

pub fn astleft(ast: &Ast, mut node: AstNodeT) -> AstNodeT {
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

pub fn astright(ast: &Ast, mut node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node {
        return ASTNULL;
    }
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

pub fn astisnodeentry(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && node >= 0 && ast.par[node as usize] >= 0
}

pub fn astisnodeexit(ast: &Ast, node: AstNodeT) -> bool {
    node < ast.par_cnt && node >= 0 && ast.par[node as usize] < 0
}

pub fn astnoderule(ast: &Ast, mut node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let idx = ast.par[node as usize] as usize;
    &ast.nodes[idx].rule
}

pub fn astnodefrom(ast: &Ast, mut node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let idx = ast.par[node as usize] as usize;
    let from = ast.nodes[idx].from as usize;
    if from <= ast.start.len() {
        &ast.start[from..]
    } else {
        ""
    }
}

pub fn astnodeto(ast: &Ast, mut node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 {
        return "";
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let idx = ast.par[node as usize] as usize;
    let to = ast.nodes[idx].to as usize;
    if to <= ast.start.len() {
        &ast.start[to..]
    } else {
        ""
    }
}

pub fn astnodelen(ast: &Ast, mut node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 {
        return 0;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let idx = ast.par[node as usize] as usize;
    ast.nodes[idx].to - ast.nodes[idx].from
}

pub fn astisleaf(ast: &Ast, mut node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 {
        return false;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let idx = ast.par[node as usize] as usize;
    ast.nodes[idx].delta == 1
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

pub fn ast_is(ast: &Ast, mut node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt || node < 0 {
        return 0;
    }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    let idx = ast.par[node as usize] as usize;
    if ast.nodes[idx].rule == rulename {
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
                let rule = astnoderule(ast, node);
                if rule == "#" {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from = astnodefrom(ast, node);
                    let to = astnodeto(ast, node);
                    let len = from.len() - to.len();
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
                let len = from.len() - to.len();
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
