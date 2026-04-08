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
    let s_ptr = start.as_ptr() as usize;
    let t_ptr = to.as_ptr() as usize;
    if t_ptr < s_ptr {
        return 0;
    }
    let ret = (t_ptr - s_ptr) as i32;
    if ret >= 0 && ret <= (1 << 16) { ret } else { 0 }
}
/// Global variable used in the C code.
/// (In C declared as `volatile int skp_zero;`—here we use a mutable static.)
pub static mut SKP_ZERO: i32 = 0;
/// Trace function (corresponds to the C macro skptrace).
pub fn skptrace(args: std::fmt::Arguments) {
    eprintln!("TRCE: {}", args);
}

// ---- Helper: get a byte at index, or 0 if out of bounds ----
fn byte_at(s: &str, i: usize) -> u8 {
    s.as_bytes().get(i).copied().unwrap_or(0)
}

/// Returns the next Unicode code point from the string `s` (similar to `skp_next` in C).
/// Returns a tuple `(code_point, rest_of_string)`.
pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    if s.is_empty() {
        return (0, s);
    }
    let b = s.as_bytes();
    let mut c = b[0] as u32;
    let mut pos = 1;

    if iso == 0 {
        // UTF-8 multi-byte: accumulate continuation bytes (0x80..0xBF)
        if pos < b.len() && (b[pos] & 0xC0) == 0x80 {
            c = (c << 8) | b[pos] as u32;
            pos += 1;
            if pos < b.len() && (b[pos] & 0xC0) == 0x80 {
                c = (c << 8) | b[pos] as u32;
                pos += 1;
                if pos < b.len() && (b[pos] & 0xC0) == 0x80 {
                    c = (c << 8) | b[pos] as u32;
                    pos += 1;
                }
            }
        }
    }

    // Handle CRLF
    if c == 0x0D && pos < b.len() && b[pos] == 0x0A {
        c = 0x0D0A;
        pos += 1;
    }

    (c, &s[pos..])
}

/// Compares two code points. If `fold` is nonzero, performs case‑insensitive comparison.
/// (Corresponds to `chr_cmp`.)
pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    if fold != 0 && a <= 0x7F && b <= 0x7F {
        return (a as u8).to_ascii_lowercase() == (b as u8).to_ascii_lowercase();
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
    (0x30..=0x39).contains(&c)
}

/// Returns true if `c` is a hexadecimal digit.
pub fn is_xdigit(c: u32) -> bool {
    (0x30..=0x39).contains(&c) || (0x41..=0x46).contains(&c) || (0x61..=0x66).contains(&c)
}

/// Returns true if `c` is an uppercase letter.
pub fn is_upper(c: u32) -> bool {
    (0x41..=0x5A).contains(&c)
}

/// Returns true if `c` is a lowercase letter.
pub fn is_lower(c: u32) -> bool {
    (0x61..=0x7A).contains(&c)
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
    let mut s = set;
    let (mut p_ch, mut rest) = skp_next(s, iso);

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        }
        s = rest;
        let r = skp_next(s, iso);
        p_ch = r.0;
        rest = r.1;
    }

    while p_ch != b']' as u32 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        s = rest;
        let r = skp_next(s, iso);
        p_ch = r.0;
        rest = r.1;
        if p_ch == b'-' as u32 && !rest.is_empty() && byte_at(rest, 0) != b']' {
            s = rest;
            let r = skp_next(s, iso);
            p_ch = r.0;
            rest = r.1;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            s = rest;
            let r = skp_next(s, iso);
            p_ch = r.0;
            rest = r.1;
        }
    }
    false
}

/// Checks if the string `s` starts with the pattern `p` for `len` characters, using flag `flg`.
pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    let start = s;
    let mut s_cur = s;
    let mut p_cur = p;
    let mut remaining = len;
    let mut mlen: i32 = 0;

    while remaining != 0 {
        if !p_cur.is_empty() && byte_at(p_cur, 0) == 0x0E {
            return mlen;
        }

        let (p_chr, p_end) = skp_next(p_cur, flg & 2);
        let (s_chr, s_end) = skp_next(s_cur, flg & 2);

        if chr_cmp(s_chr, p_chr, flg & 1) {
            let s_advance = s_cur.len() - s_end.len();
            let p_advance = p_cur.len() - p_end.len();
            mlen += s_advance as i32;
            remaining -= p_advance as i32;
            p_cur = p_end;
            s_cur = s_end;
        } else {
            // search for alternative (0x0E separator)
            while remaining > 0 && !p_cur.is_empty() && byte_at(p_cur, 0) != 0x0E {
                p_cur = &p_cur[1..];
                remaining -= 1;
            }
            if remaining <= 0 {
                return 0;
            }
            remaining -= 1;
            p_cur = if p_cur.is_empty() { p_cur } else { &p_cur[1..] };
            s_cur = start;
            mlen = 0;
        }
    }
    mlen
}

/// Given an opening parenthesis code point, returns the corresponding closing code point.
pub fn get_close(open: u32) -> u32 {
    match open {
        0x28 => 0x29, // ( -> )
        0x5B => 0x5D, // [ -> ]
        0x7B => 0x7D, // { -> }
        0x3C => 0x3E, // < -> >
        _ => 0,
    }
}

/// Given a quote character, returns the corresponding closing quote.
pub fn get_qclose(open: u32) -> u32 {
    match open {
        0x27 | 0x22 | 0x60 => open, // ' " `
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
    let mut s_end = src;
    let mut s_tmp = src;
    let (mut s_chr, next_tmp) = skp_next(s_end, *flg & 2);
    s_tmp = next_tmp;

    let mut pat_cur = pat;

    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;

    if !pat_cur.is_empty() && byte_at(pat_cur, 0) == b'*' {
        match_min = 0; match_max = u32::MAX; pat_cur = &pat_cur[1..];
    } else if !pat_cur.is_empty() && byte_at(pat_cur, 0) == b'+' {
        match_max = u32::MAX; pat_cur = &pat_cur[1..];
    } else if !pat_cur.is_empty() && byte_at(pat_cur, 0) == b'?' {
        match_min = 0; pat_cur = &pat_cur[1..];
    }

    if !pat_cur.is_empty() && byte_at(pat_cur, 0) == b'!' {
        match_not = 1; pat_cur = &pat_cur[1..];
    }

    // Macro W equivalent: match while test is true (XOR with match_not)
    macro_rules! w_match {
        ($test:expr) => {{
            let mut match_cnt: u32 = 0;
            while match_cnt < match_max && s_chr != 0 && (($test) != (match_not != 0)) {
                match_cnt += 1;
                s_end = s_tmp;
                let r = skp_next(s_end, *flg & 2);
                s_chr = r.0;
                s_tmp = r.1;
            }
            if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL }
        }};
    }

    // get_next_s_chr: advance one raw byte at a time
    macro_rules! get_next {
        () => {{
            s_end = s_tmp;
            if s_end.is_empty() {
                s_chr = 0;
            } else {
                s_chr = byte_at(s_end, 0) as u32;
                s_tmp = &s_end[1..];
            }
        }};
    }

    if pat_cur.is_empty() {
        return (MATCHED_FAIL, src, pat);
    }

    let pat_byte = byte_at(pat_cur, 0);
    pat_cur = &pat_cur[1..];
    let mut intnumber = false;
    let mut ret;

    match pat_byte {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { MATCHED } else { MATCHED_FAIL };
            } else {
                ret = w_match!(s_chr != 0);
            }
        }
        b'$' => {
            if s_chr == 0 {
                ret = MATCHED;
            } else {
                ret = w_match!(is_break(s_chr));
            }
        }
        b'n' => { ret = w_match!(is_break(s_chr)); }
        b'd' => { ret = w_match!(is_digit(s_chr)); }
        b'x' => { ret = w_match!(is_xdigit(s_chr)); }
        b'a' => { ret = w_match!(is_alpha(s_chr)); }
        b'u' => { ret = w_match!(is_upper(s_chr)); }
        b'l' => { ret = w_match!(is_lower(s_chr)); }
        b's' => { ret = w_match!(is_space(s_chr)); }
        b'w' => { ret = w_match!(is_blank(s_chr)); }
        b'c' => { ret = w_match!(is_ctrl(s_chr)); }
        b'i' => { ret = w_match!(is_idchr(s_chr)); }
        b'@' => { ret = w_match!(is_alnum(s_chr)); }
        b'&' => {
            ret = if match_not != 0 { MATCHED_GOALNOT } else { MATCHED_GOAL };
        }
        b'[' => {
            ret = w_match!(is_oneof(s_chr, pat_cur, *flg & 2));
            if !pat_cur.is_empty() && byte_at(pat_cur, 0) == b']' {
                pat_cur = &pat_cur[1..];
            }
            while !pat_cur.is_empty() && byte_at(pat_cur, 0) != b']' {
                pat_cur = &pat_cur[1..];
            }
            if !pat_cur.is_empty() {
                pat_cur = &pat_cur[1..]; // skip ']'
            }
        }
        b'"' | b'\'' | b'`' => {
            let quote = pat_byte;
            let mut l = 0usize;
            while l < pat_cur.len() && byte_at(pat_cur, l) != quote {
                l += 1;
            }
            if l > 0 {
                let ml = is_string(s_end, pat_cur, l as i32, *flg);
                if ml > 0 {
                    if match_not == 0 {
                        s_end = &s_end[ml as usize..];
                        ret = MATCHED;
                    } else {
                        ret = MATCHED_FAIL;
                    }
                } else if match_min == 0 || match_not != 0 {
                    ret = MATCHED;
                } else {
                    ret = MATCHED_FAIL;
                }
            } else if match_min == 0 || match_not != 0 {
                ret = MATCHED;
            } else {
                ret = MATCHED_FAIL;
            }
            pat_cur = if l < pat_cur.len() { &pat_cur[l + 1..] } else { &pat_cur[l..] };
        }
        b'C' => {
            *flg = (*flg & !1) | match_not as i32;
            ret = MATCHED;
        }
        b'U' => {
            *flg = (*flg & !2) | (match_not as i32 * 2);
            ret = MATCHED;
        }
        b'S' => {
            while is_space(s_chr) { get_next!(); }
            ret = MATCHED;
        }
        b'W' => {
            while is_blank(s_chr) { get_next!(); }
            ret = MATCHED;
        }
        b'N' => {
            while s_chr != 0 && !is_break(s_chr) { get_next!(); }
            if s_chr != 0 { get_next!(); }
            ret = MATCHED;
        }
        b'I' => {
            ret = MATCHED_FAIL;
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next!();
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) { break; }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            if !pat_cur.is_empty() && byte_at(pat_cur, 0) == b')' && s_chr == b'(' as u32 {
                pat_cur = &pat_cur[1..];
                // fall through to B
            } else {
                // no match, revert pat
                pat_cur = &pat_cur[..]; // pat already advanced past '('
                return (MATCHED_FAIL, src, &pat[pat_cur.as_ptr() as usize - pat.as_ptr() as usize - 1..]);
            }
            // Balanced parenthesis (same as B)
            ret = MATCHED_FAIL;
            let close = get_close(s_chr);
            if close != 0 {
                let open = s_chr;
                let mut count: i32 = 1;
                while s_chr != 0 && count > 0 {
                    get_next!();
                    if s_chr == open { count += 1; }
                    if s_chr == close { count -= 1; }
                }
                if count == 0 {
                    get_next!();
                    ret = MATCHED;
                }
            }
        }
        b'B' => {
            ret = MATCHED_FAIL;
            let close = get_close(s_chr);
            if close != 0 {
                let open = s_chr;
                let mut count: i32 = 1;
                while s_chr != 0 && count > 0 {
                    get_next!();
                    if s_chr == open { count += 1; }
                    if s_chr == close { count -= 1; }
                }
                if count == 0 {
                    get_next!();
                    ret = MATCHED;
                }
            }
        }
        b'Q' => {
            ret = MATCHED_FAIL;
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next!();
                    if s_chr == qclose { break; }
                    if s_chr == b'\\' as u32 { get_next!(); }
                }
                if s_chr != 0 {
                    get_next!();
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            ret = MATCHED_FAIL;
            if s_chr == b'0' as u32
                && s_end.len() > 2
                && (byte_at(s_end, 1) == b'x' || byte_at(s_end, 1) == b'X')
                && is_xdigit(byte_at(s_end, 2) as u32)
            {
                get_next!(); get_next!(); get_next!();
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                get_next!();
            }
        }
        b'D' => {
            intnumber = true;
            ret = MATCHED_FAIL;
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next!();
                    if !is_space(s_chr) { break; }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next!();
            }
        }
        b'F' => {
            ret = MATCHED_FAIL;
            if s_chr == b'+' as u32 || s_chr == b'-' as u32 {
                loop {
                    get_next!();
                    if !is_space(s_chr) { break; }
                }
            }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next!();
            }
            if s_chr == b'.' as u32 { get_next!(); }
            while is_digit(s_chr) {
                ret = MATCHED;
                get_next!();
            }
            if ret == MATCHED && (s_chr == b'E' as u32 || s_chr == b'e' as u32) {
                get_next!();
                if s_chr == b'+' as u32 || s_chr == b'-' as u32 { get_next!(); }
                while is_digit(s_chr) { get_next!(); }
                if s_chr == b'.' as u32 { get_next!(); }
                while is_digit(s_chr) { get_next!(); }
            }
        }
        _ => {
            ret = MATCHED_FAIL;
            // revert pat_cur back (we advanced past the unknown char)
            pat_cur = &pat[pat_cur.as_ptr() as usize - pat.as_ptr() as usize - 1..];
        }
    }

    if ret != MATCHED_FAIL {
        (ret, s_end, pat_cur)
    } else {
        (MATCHED_FAIL, src, pat)
    }
}

/// The core scanning function from the C header.
pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    if pat.is_empty() {
        return (0, src, src);
    }

    let mut pat_cur = pat;
    let mut skp_to = false;

    if byte_at(pat_cur, 0) == b'>' {
        skp_to = true;
        pat_cur = &pat_cur[1..];
    }

    let pat_start = pat_cur;
    let mut start = src;
    let mut s = start;
    let mut flg: i32 = 0;
    let mut matched: i32 = 0;
    let mut goal: Option<&str> = None;
    let mut goalnot: Option<&str> = None;

    // skip leading spaces in pattern
    while !pat_cur.is_empty() && is_space(byte_at(pat_cur, 0) as u32) {
        pat_cur = &pat_cur[1..];
    }

    while !pat_cur.is_empty() && byte_at(pat_cur, 0) > 7 {
        let (m, s_end, p_end) = match_pat(pat_cur, s, &mut flg);
        matched = m;
        if matched != MATCHED_FAIL {
            s = s_end;
            pat_cur = p_end;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s);
            }
        } else {
            // skip to end of current alternative (bytes > 7)
            while !pat_cur.is_empty() && byte_at(pat_cur, 0) > 7 {
                pat_cur = &pat_cur[1..];
            }
            if !pat_cur.is_empty() && byte_at(pat_cur, 0) > 0
                && pat_cur.len() > 1 && byte_at(pat_cur, 1) > 0
            {
                // Try next alternative
                s = start;
                pat_cur = &pat_cur[1..];
            } else if skp_to {
                goal = None;
                goalnot = None;
                pat_cur = pat_start;
                if start.is_empty() {
                    break;
                }
                start = &start[1..];
                s = start;
                if start.is_empty() {
                    break;
                }
            } else {
                break;
            }
        }
        // skip spaces in pattern
        while !pat_cur.is_empty() && is_space(byte_at(pat_cur, 0) as u32) {
            pat_cur = &pat_cur[1..];
        }
    }

    if matched == MATCHED_FAIL && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        pat_cur = "";
    }

    if let Some(g) = goal {
        s = g;
    }

    if matched != MATCHED_FAIL && (pat_cur.is_empty() || byte_at(pat_cur, 0) <= 7) {
        let ret_code = if pat_cur.is_empty() || byte_at(pat_cur, 0) == 0 {
            1
        } else {
            byte_at(pat_cur, 0) as i32
        };
        let to = if skp_to { start } else { s };
        return (ret_code, to, s);
    }

    (0, src, src)
}

/// In the C header a set of macros provides variants:
pub fn skp_4<'a>(src: &'a str, pat: &'a str, to: Option<&mut &'a str>, end: Option<&mut &'a str>) -> i32 {
    let (ret, t, e) = skp_(src, pat);
    if let Some(to_ref) = to { *to_ref = t; }
    if let Some(end_ref) = end { *end_ref = e; }
    ret
}
pub fn skp_3<'a>(src: &'a str, pat: &'a str, end: Option<&mut &'a str>) -> i32 {
    let (ret, _t, e) = skp_(src, pat);
    if let Some(end_ref) = end { *end_ref = e; }
    ret
}
pub fn skp_2(src: &str, pat: &str) -> i32 {
    let (ret, _, _) = skp_(src, pat);
    ret
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

pub const ASTNULL: i32 = -1;
const SKP_DEBUG: i8 = 0x01;
const SKP_MAXDEPTH: u16 = 10000;
const SKP_STARTNODES: i32 = 8;

static SKP_N_STRING: &str = "$";
static SKP_N_INFO: &str = "#";

/// A function pointer type for parsing rules.
pub type SkpRule = fn(ast: &mut Ast, ret: &mut i32);

fn ast_newpar(ast: &mut Ast) -> i32 {
    if ast.par_cnt >= ast.par_max {
        let mut new_max = ast.par_max;
        while ast.par_cnt + 1 > new_max {
            new_max += new_max / 2;
            new_max += new_max & 1;
        }
        ast.par.resize(new_max as usize, 0);
        ast.par_max = new_max;
    }
    let r = ast.par_cnt;
    ast.par_cnt += 1;
    r
}

fn ast_newnode(ast: &mut Ast) -> i32 {
    if ast.nodes_cnt >= ast.nodes_max {
        let mut new_max = ast.nodes_max;
        while ast.nodes_cnt + 1 > new_max {
            new_max += new_max / 2;
            new_max += new_max & 1;
        }
        ast.nodes.resize(new_max as usize, AstNode::default());
        ast.nodes_max = new_max;
    }
    let r = ast.nodes_cnt;
    ast.nodes_cnt += 1;
    r
}

fn par_makeroom(ast: &mut Ast, needed: i32) {
    if ast.par_cnt + needed > ast.par_max {
        let mut new_max = ast.par_max;
        while ast.par_cnt + needed > new_max {
            new_max += new_max / 2;
            new_max += new_max & 1;
        }
        ast.par.resize(new_max as usize, 0);
        ast.par_max = new_max;
    }
}

fn nodes_makeroom(ast: &mut Ast, needed: i32) {
    if ast.nodes_cnt + needed > ast.nodes_max {
        let mut new_max = ast.nodes_max;
        while ast.nodes_cnt + needed > new_max {
            new_max += new_max / 2;
            new_max += new_max & 1;
        }
        ast.nodes.resize(new_max as usize, AstNode::default());
        ast.nodes_max = new_max;
    }
}

/// Creates a new AST.
pub fn ast_new() -> Option<Ast> {
    let mut ast = Ast::default();
    ast.nodes_max = SKP_STARTNODES;
    ast.nodes = vec![AstNode::default(); ast.nodes_max as usize];
    ast.par_max = SKP_STARTNODES * 2;
    ast.par = vec![0i32; ast.par_max as usize];
    ast.mmz_max = 64;
    ast.err_msg = Some(String::new());
    ast.err_pos = -1;
    ast.cur_node = ASTNULL;
    Some(ast)
}

/// Frees an AST.
pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
}

/// Opens a new AST node starting at position `from` with the given rule.
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

/// Closes the current AST node at position `to`, linking with the open node `open`.
pub fn ast_close(ast: &mut Ast, to: i32, open: i32) -> i32 {
    if open < 0 { return -1; }
    let nd_idx = ast.par[open as usize];
    if ast.fail != 0 {
        ast.pos = ast.nodes[nd_idx as usize].from;
        ast.nodes_cnt = nd_idx;
        ast.par_cnt = open;
        return -1;
    }
    let par = ast_newpar(ast);
    if par < 0 { return -1; }
    let delta = par - open;
    ast.nodes[nd_idx as usize].to = to;
    ast.nodes[nd_idx as usize].delta = delta;
    ast.nodes[nd_idx as usize].tag = 0;
    ast.par[par as usize] = -delta;
    ast.cur_node = par;
    ast.cur_rule = Some(ast.nodes[nd_idx as usize].rule.clone());
    par
}

/// Parses the source string `src` using a given parsing rule.
pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { SKP_DEBUG } else { 0 };

    let pos = ast.pos;
    let open = ast_open(&mut ast, pos, rulename);
    if open >= 0 {
        let mut ret_val = 0i32;
        rule(&mut ast, &mut ret_val);
        ast.ret = ret_val;

        if ast.fail != 0 && ast.err_pos < ast.pos {
            ast.err_pos = ast.pos;
            ast.err_rule = Some(rulename.to_string());
        }

        let close_pos = ast.pos;
        ast_close(&mut ast, close_pos, open);
        if ast.nodes_cnt > 0 {
            ast.err_pos = -1;
            let li = ast.lastinfo;
            ast_setinfo(&mut ast, li, 0);
        }
    }
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

/// Returns the rule name at which an error occurred.
pub fn asterrrule(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 { return Some(""); }
    ast.err_rule.as_deref()
}

/// Returns the error position as a string pointer.
pub fn asterrpos(ast: &Ast) -> Option<&str> {
    if ast.err_pos < 0 { return Some(""); }
    Some(&ast.start[ast.err_pos as usize..])
}

/// Returns the start of the error line.
pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 { return ""; }
    let bytes = ast.start.as_bytes();
    let mut pos = ast.err_pos as usize;
    while pos > 0 {
        if bytes[pos - 1] == b'\n' || bytes[pos - 1] == b'\r' {
            break;
        }
        pos -= 1;
    }
    &ast.start[pos..]
}

/// Returns the error column number.
pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 { return 0; }
    let ln = asterrline(ast);
    let ln_start = ln.as_ptr() as usize;
    let err_ptr = ast.start.as_ptr() as usize + ast.err_pos as usize;
    (err_ptr - ln_start) as i32
}

/// Aborts parsing with the given message and rule.
pub fn skp__abort(ast: &mut Ast, msg: &str, rule: &str) {
    if !msg.is_empty() {
        ast.err_msg = Some(msg.to_string());
    }
    ast.err_pos = ast.pos;
    ast.err_rule = Some(rule.to_string());
    // In C this does longjmp; in Rust we set fail flag
    ast.fail = 1;
}

/// Records memoization of AST nodes (for left recursion etc.).
pub fn skp_memoize(ast: &mut Ast, mmz: &mut AstMmz, rule: &str, old_pos: i32, start_par: i32) {
    let mut end_par = ast.par_cnt;
    let mut actual_start = start_par;
    if ast.fail != 0 || end_par <= actual_start {
        actual_start = -1;
        end_par = -1;
    }
    let numnodes = if actual_start >= 0 { (end_par - actual_start) / 2 } else { 0 };

    mmz.pos = old_pos;
    mmz.endpos = ast.pos;
    mmz.numnodes = if ast.fail != 0 { -1 } else { numnodes };
    mmz.lastinfo = ast.lastinfo;

    mmz.nodes.clear();
    if actual_start >= 0 {
        for k in actual_start..end_par {
            if ast.par[k as usize] >= 0 {
                mmz.nodes.push(ast.nodes[ast.par[k as usize] as usize].clone());
            }
        }
    }
}

/// Attempts to retrieve a memoized result.
pub fn skp_dememoize(ast: &mut Ast, mmz: &mut AstMmz, _rule: &str) -> i32 {
    // Simplified: in the C code, mmz is an array of 4 slots with sentinel values.
    // In our Rust version, AstMmz is a single struct. We check if it has been populated.
    if mmz.pos != ast.pos {
        return 0;
    }
    let numnodes = mmz.numnodes;
    ast.fail = if numnodes < 0 { 1 } else { 0 };
    ast.lastpos = ast.pos;
    ast.pos = mmz.endpos;
    ast.lastinfo = mmz.lastinfo;

    if numnodes > 0 {
        nodes_makeroom(ast, numnodes);
        par_makeroom(ast, 2 * numnodes);

        let base_par = ast.par_cnt;
        // Initialize new par slots to i32::MAX (sentinel)
        for k in base_par..(base_par + 2 * numnodes) {
            if (k as usize) < ast.par.len() {
                ast.par[k as usize] = i32::MAX;
            }
        }

        let mut cur_par = base_par;
        for k in 0..numnodes {
            ast.nodes[ast.nodes_cnt as usize] = mmz.nodes[k as usize].clone();
            while (cur_par as usize) < ast.par.len() && ast.par[cur_par as usize] != i32::MAX {
                cur_par += 1;
            }
            if (cur_par as usize) < ast.par.len() {
                ast.par[cur_par as usize] = ast.nodes_cnt;
            }
            let delta = mmz.nodes[k as usize].delta;
            let close_idx = cur_par + delta;
            if (close_idx as usize) < ast.par.len() {
                ast.par[close_idx as usize] = -delta;
            }
            ast.nodes_cnt += 1;
        }
        ast.par_cnt += 2 * numnodes;
    }
    1
}

/// Sets AST node information.
pub fn ast_setinfo(ast: &mut Ast, info: i32, node: AstNodeT) {
    let mut n = node;
    if ast.par_cnt <= n { return; }
    if n == ASTNULL { n = ast.par_cnt - 1; }
    if n < 0 || n as usize >= ast.par.len() { return; }
    if ast.par[n as usize] < 0 {
        n += ast.par[n as usize];
    }
    if n < 0 || n as usize >= ast.par.len() { return; }
    let idx = ast.par[n as usize];
    if idx >= 0 && (idx as usize) < ast.nodes.len() {
        ast.nodes[idx as usize].tag = info;
    }
}

/// Records a new AST info node.
pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail != 0 { return; }
    let par = ast_open(ast, ast.pos, SKP_N_INFO);
    ast_close(ast, ast.pos, par);
    if par >= 0 && (par as usize) < ast.par.len() {
        let idx = ast.par[par as usize];
        if idx >= 0 && (idx as usize) < ast.nodes.len() {
            ast.nodes[idx as usize].tag = info;
        }
    }
    ast.lastinfo = info;
}

/// Retrieves the information associated with a node.
pub fn astnodeinfo(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 { return 0; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return 0; }
    ast.nodes[ast.par[n as usize] as usize].tag
}

/// Swaps the last two AST nodes.
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

    let len2 = (c2 - o2 + 1) as usize;
    let len1 = (c1 - o1 + 1) as usize;
    let o2u = o2 as usize;
    let o1u = o1 as usize;

    let tmp: Vec<i32> = ast.par[o2u..o2u + len2].to_vec();
    ast.par.copy_within(o1u..o1u + len1, o2u);
    ast.par[o2u + len1..o2u + len1 + len2].copy_from_slice(&tmp);
}

/// Lowers a node (wraps a group of nodes into a new parent).
pub fn ast_lower(ast: &mut Ast, rule: &str, f: AstNodeT, t: AstNodeT) {
    if ast.par_cnt <= f || ast.par_cnt <= t || f >= t { return; }
    let mut lft = f;
    let mut rgt = t;
    if ast.par[lft as usize] < 0 { lft += ast.par[lft as usize]; }
    if ast.par[rgt as usize] < 0 { rgt += ast.par[rgt as usize]; }

    let node_from = ast.nodes[ast.par[lft as usize] as usize].from;
    let node_to = ast.nodes[ast.par[rgt as usize] as usize].to;

    rgt += ast.nodes[ast.par[rgt as usize] as usize].delta;

    let node = ast_newnode(ast);
    if node < 0 { return; }
    let delta = rgt - lft + 2;
    ast.nodes[node as usize] = AstNode {
        rule: rule.to_string(),
        from: node_from,
        to: node_to,
        delta,
        tag: 0,
    };

    // make room for 2 new parentheses
    ast_newpar(ast);
    ast_newpar(ast);

    let rgt_u = rgt as usize;
    // Move nodes after rgt
    if ast.par_cnt as usize - 1 - rgt_u > 2 {
        let src_start = rgt_u + 1;
        let src_end = ast.par_cnt as usize - 2;
        let dst = rgt_u + 3;
        ast.par.copy_within(src_start..src_end, dst);
    }

    // Move the nodes between lft and rgt
    let lft_u = lft as usize;
    let count = rgt_u - lft_u + 1;
    ast.par.copy_within(lft_u..lft_u + count, lft_u + 1);

    ast.par[lft_u] = node;
    ast.par[rgt_u + 2] = -delta;
}

/// Lifts a node (removes a level from the AST).
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
    if o2 != o1 + 1 { return; } // More than one child

    if ast.nodes[ast.par[o1 as usize] as usize].tag == 0 {
        let o2u = o2 as usize;
        let o1u = o1 as usize;
        let len = (c2 - o2 + 1) as usize;
        ast.par.copy_within(o2u..o2u + len, o1u);
        ast.par_cnt -= 2;
    }
}

/// Lifts all single-child nodes.
pub fn ast_lift_all(ast: &mut Ast) {
    loop {
        let n = ast.par_cnt;
        ast_lift(ast);
        if n == ast.par_cnt { break; }
    }
}

/// Removes the last leaf node.
pub fn ast_noleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    if c1 == o1 + 1 { ast.par_cnt -= 2; }
}

/// Removes the last empty leaf node.
pub fn ast_noemptyleaf(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    if c1 != o1 + 1 { return; } // not a leaf
    let idx = ast.par[o1 as usize] as usize;
    if ast.nodes[idx].from != ast.nodes[idx].to { return; } // not empty
    ast.par_cnt -= 2;
}

/// Returns the index of the last AST node.
pub fn ast_lastnode(ast: &Ast) -> AstNodeT {
    if ast.fail != 0 || ast.par_cnt < 2 { return ASTNULL; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return ASTNULL; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return ASTNULL; }
    o1
}

/// Checks if the last node is empty.
pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == ASTNULL { return false; }
    let idx = ast.par[node as usize] as usize;
    ast.nodes[idx].from == ast.nodes[idx].to
}

/// Deletes the last node.
pub fn ast_delete(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    ast.par_cnt -= c1 - o1 + 1;
}

/// Returns the "left" sibling of a node.
pub fn astleft(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || node >= ast.par_cnt { return ASTNULL; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    n -= 1;
    if n <= 0 || ast.par[n as usize] >= 0 { return ASTNULL; }
    n += ast.par[n as usize];
    n
}

/// Returns the "right" sibling of a node.
pub fn astright(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node <= 0 || node >= ast.par_cnt { return ASTNULL; }
    let mut n = node;
    if ast.par[n as usize] > 0 {
        n += ast.nodes[ast.par[n as usize] as usize].delta;
    }
    n += 1;
    if n >= ast.par_cnt || ast.par[n as usize] < 0 { return ASTNULL; }
    n
}

/// Returns the parent of a node.
pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let n = astfirst(ast, node);
    if n == ASTNULL { return ASTNULL; }
    let up = n - 1;
    if up < 0 || ast.par[up as usize] < 0 { return ASTNULL; }
    up
}

/// Returns the first child of a node.
pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || node >= ast.par_cnt { return ASTNULL; }
    let n = node + 1;
    if n >= ast.par_cnt || ast.par[n as usize] < 0 { return ASTNULL; }
    n
}

/// Returns the leftmost sibling (first child) of a node.
pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || node >= ast.par_cnt { return ASTNULL; }
    let mut cur = node;
    loop {
        let n = astleft(ast, cur);
        if n == ASTNULL { break; }
        cur = n;
    }
    cur
}

/// Returns the rightmost sibling of a node.
pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || node >= ast.par_cnt { return ASTNULL; }
    let mut cur = node;
    loop {
        let n = astright(ast, cur);
        if n == ASTNULL { break; }
        cur = n;
    }
    cur
}

/// Returns the next node in a depth-first traversal.
pub fn astnextdf(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let ndx = node + 1;
    if ndx < 0 { return 0; }
    if ndx >= ast.par_cnt { return ASTNULL; }
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
    if node >= ast.par_cnt || node < 0 { return ""; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return ""; }
    &ast.nodes[ast.par[n as usize] as usize].rule
}

/// Returns the source substring from the start of the node.
pub fn astnodefrom(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return ""; }
    let from = ast.nodes[ast.par[n as usize] as usize].from as usize;
    &ast.start[from..]
}

/// Returns the source substring up to the end of the node.
pub fn astnodeto(ast: &Ast, node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return ""; }
    let to = ast.nodes[ast.par[n as usize] as usize].to as usize;
    &ast.start[to..]
}

/// Returns the length of the node.
pub fn astnodelen(ast: &Ast, node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 { return 0; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return 0; }
    let nd = &ast.nodes[ast.par[n as usize] as usize];
    nd.to - nd.from
}

/// Checks if a node is a leaf.
pub fn astisleaf(ast: &Ast, node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 { return false; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return false; }
    ast.nodes[ast.par[n as usize] as usize].delta == 1
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
    if ast_is(ast, node, r1) != 0 { return 1; }
    if let Some(r) = r2 { if ast_is(ast, node, r) != 0 { return 1; } }
    if let Some(r) = r3 { if ast_is(ast, node, r) != 0 { return 1; } }
    if let Some(r) = r4 { if ast_is(ast, node, r) != 0 { return 1; } }
    if let Some(r) = r5 { if ast_is(ast, node, r) != 0 { return 1; } }
    0
}

/// Checks if a node's rule matches a given rule.
pub fn ast_is(ast: &Ast, node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt { return 0; }
    let mut n = node;
    if ast.par[n as usize] < 0 { n += ast.par[n as usize]; }
    if n < 0 { return 0; }
    let nd = &ast.nodes[ast.par[n as usize] as usize];
    if nd.rule == rulename { 1 } else { 0 }
}

/// Checks if the AST contains an error.
pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

/// Prints the AST in s-expression format.
pub fn astprintsexpr(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node = ASTNULL;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL { break; }
        if astisnodeentry(ast, node) {
            let _ = write!(f, "({} ", astnoderule(ast, node));
            if astisleaf(ast, node) {
                let _ = write!(f, "'");
                if astnoderule(ast, node) == SKP_N_INFO {
                    let _ = write!(f, "{}", astnodeinfo(ast, node));
                } else {
                    let from = astnodefrom(ast, node);
                    let to = astnodeto(ast, node);
                    let len = from.len() - to.len();
                    for ch in from[..len].chars() {
                        if ch == '\'' { let _ = write!(f, "\\"); }
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
    let mut node = ASTNULL;
    let mut levl: i32 = 0;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL { break; }
        if astisnodeentry(ast, node) {
            for _ in (0..levl).step_by(4) {
                let _ = write!(f, "    ");
            }
            let _ = write!(f, "[{}", astnoderule(ast, node));
            let tag = astnodeinfo(ast, node);
            if tag != 0 { let _ = write!(f, " ({})", tag); }
            let _ = write!(f, "]");
            levl += 4;
            if astisleaf(ast, node) {
                let _ = write!(f, " '");
                let from = astnodefrom(ast, node);
                let to = astnodeto(ast, node);
                let len = from.len() - to.len();
                for ch in from[..len].chars() {
                    if ch == '\'' { let _ = write!(f, "\\"); }
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
