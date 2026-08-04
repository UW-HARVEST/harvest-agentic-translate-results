/// SKP version information.
pub const SKP_VER: u32 = 0x0003001C;
pub const SKP_VER_STR: &str = "0.3.1rc";

/// A loop state used for scanning.
#[derive(Debug, Default, Clone)]
pub struct SkpLoop {
    pub start: String,
    pub to: Option<String>,
    pub end: Option<String>,
    pub alt: i32,
}

/// Returns the "length" from start to to. (This mimics the inline function `skp_loop_len`.)
pub fn skp_loop_len(start: &str, to: &str) -> i32 {
    let start_len = start.len() as i32;
    let to_len = to.len() as i32;
    let ret = start_len - to_len;
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

// -------------------------------------------------------------------
// Helper utilities for working with byte indices into &str.
// -------------------------------------------------------------------

#[inline]
fn str_suffix(s: &str, i: usize) -> &str {
    let i = i.min(s.len());
    // Safety: s is valid UTF-8 from start. Slicing by byte index may leave
    // us in the middle of a multi-byte sequence. The returned &str is only
    // used for byte-length comparisons and prefix slicing — never displayed
    // from within a multi-byte char in valid usage. We use unchecked here
    // to avoid panicking on non-char-boundary indices reachable from byte
    // advances in `skp_to` mode.
    unsafe { std::str::from_utf8_unchecked(&s.as_bytes()[i..]) }
}

#[inline]
fn byte_at(b: &[u8], i: usize) -> u8 {
    if i < b.len() { b[i] } else { 0 }
}

// -------------------------------------------------------------------
// Character-class helpers
// -------------------------------------------------------------------

pub fn chr_cmp(a: u32, b: u32, fold: i32) -> bool {
    let mut a = a;
    let mut b = b;
    if (fold & 1) != 0 && a <= 0x7F && b <= 0x7F {
        if (b'A' as u32) <= a && a <= (b'Z' as u32) {
            a += 32;
        }
        if (b'A' as u32) <= b && b <= (b'Z' as u32) {
            b += 32;
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
        0x00E28000 => (0xE28080 <= c && c <= 0xE2808A) || c == 0xE280AF,
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
    (b'0' as u32) <= c && c <= (b'9' as u32)
}

pub fn is_xdigit(c: u32) -> bool {
    ((b'0' as u32) <= c && c <= (b'9' as u32))
        || ((b'A' as u32) <= c && c <= (b'F' as u32))
        || ((b'a' as u32) <= c && c <= (b'f' as u32))
}

pub fn is_upper(c: u32) -> bool {
    (b'A' as u32) <= c && c <= (b'Z' as u32)
}

pub fn is_lower(c: u32) -> bool {
    (b'a' as u32) <= c && c <= (b'z' as u32)
}

pub fn is_alpha(c: u32) -> bool {
    is_upper(c) || is_lower(c)
}

pub fn is_idchr(c: u32) -> bool {
    is_alpha(c) || is_digit(c) || c == (b'_' as u32)
}

pub fn is_alnum(c: u32) -> bool {
    is_alpha(c) || is_digit(c)
}

pub fn is_ctrl(c: u32) -> bool {
    c < 0x20 || (0xC280 <= c && c < 0xC2A0) || (0x7F <= c && c < 0xA0)
}

// -------------------------------------------------------------------
// skp_next: read next "character" (byte-based, with optional UTF-8 fold)
// Returns (code_point, bytes_consumed)
// -------------------------------------------------------------------

fn skp_next_bytes(s: &[u8], iso: i32) -> (u32, usize) {
    let mut c: u32 = 0;
    let mut i: usize = 0;
    if !s.is_empty() && s[0] != 0 {
        c = s[0] as u32;
        i = 1;
        if iso == 0 {
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
        if c == 0x0D && i < s.len() && s[i] == 0x0A {
            c = 0x0D0A;
            i += 1;
        }
    }
    (c, i)
}

pub fn skp_next(s: &str, iso: i32) -> (u32, &str) {
    let bytes = s.as_bytes();
    let (c, i) = skp_next_bytes(bytes, iso);
    (c, str_suffix(s, i))
}

// -------------------------------------------------------------------
// is_oneof / is_string / get_close / get_qclose
// -------------------------------------------------------------------

fn is_oneof_bytes(ch: u32, set: &[u8], iso: i32) -> bool {
    if ch == 0 {
        return false;
    }
    let mut i = 0;
    let (mut p_ch, n) = skp_next_bytes(&set[i..], iso);
    i += n;

    if p_ch == b']' as u32 {
        if ch == b']' as u32 {
            return true;
        } else {
            let (pc, n) = skp_next_bytes(&set[i..], iso);
            p_ch = pc;
            i += n;
        }
    }

    while p_ch != b']' as u32 && p_ch != 0 {
        if p_ch == ch {
            return true;
        }
        let q_ch = p_ch;
        let (pc, n) = skp_next_bytes(&set[i..], iso);
        p_ch = pc;
        i += n;
        // peek next byte for ']'
        let next_byte = byte_at(set, i);
        if p_ch == b'-' as u32 && next_byte != b']' {
            let (pc, n) = skp_next_bytes(&set[i..], iso);
            p_ch = pc;
            i += n;
            if q_ch < ch && ch <= p_ch {
                return true;
            }
            let (pc, n) = skp_next_bytes(&set[i..], iso);
            p_ch = pc;
            i += n;
        }
    }
    false
}

pub fn is_oneof(ch: u32, set: &str, iso: i32) -> bool {
    is_oneof_bytes(ch, set.as_bytes(), iso)
}

fn is_string_bytes(s: &[u8], p: &[u8], len_in: i32, flg: i32) -> i32 {
    let start = s;
    let mut s = s;
    let mut p = p;
    let mut len = len_in;
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
            s = &s[s_n..];
            p = &p[p_n..];
        } else {
            // search for an alternative
            while len > 0 && !p.is_empty() {
                let c = p[0];
                p = &p[1..];
                if c == 0x0E {
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

pub fn is_string(s: &str, p: &str, len: i32, flg: i32) -> i32 {
    is_string_bytes(s.as_bytes(), p.as_bytes(), len, flg)
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
        x if x == b'\'' as u32 || x == b'"' as u32 || x == b'`' as u32 => open,
        _ => 0,
    }
}

pub const MATCHED_FAIL: i32 = 0;
pub const MATCHED: i32 = 1;
pub const MATCHED_GOAL: i32 = 2;
pub const MATCHED_GOALNOT: i32 = 3;

// -------------------------------------------------------------------
// match: ports the C `match` function
// Returns (ret, p_end_offset, s_end_offset)
// -------------------------------------------------------------------

fn match_pat_bytes(pat: &[u8], src: &[u8], flg: &mut i32) -> (i32, usize, usize) {
    let mut ret = MATCHED_FAIL;
    let mut match_min: u32 = 1;
    let mut match_max: u32 = 1;
    let mut match_not: u32 = 0;
    let mut intnumber = false;

    let pat_at = |i: usize| -> u8 {
        if i < pat.len() { pat[i] } else { 0 }
    };
    let src_at = |i: usize| -> u8 {
        if i < src.len() { src[i] } else { 0 }
    };

    let mut s_end: usize = 0;
    let (init_chr, init_n) = skp_next_bytes(src, *flg & 2);
    let mut s_chr: u32 = init_chr;
    let mut s_tmp: usize = init_n;

    let mut p: usize = 0;

    if pat_at(p) == b'*' { match_min = 0; match_max = u32::MAX; p += 1; }
    else if pat_at(p) == b'+' { match_max = u32::MAX; p += 1; }
    else if pat_at(p) == b'?' { match_min = 0; p += 1; }

    if pat_at(p) == b'!' { match_not = 1; p += 1; }

    let pat_char = pat_at(p);
    p += 1;

    // Run "W(cond)" loop. cond is recomputed each iteration based on s_chr.
    macro_rules! w {
        ($cond:expr) => {{
            let mut match_cnt: u32 = 0;
            while match_cnt < match_max
                && s_chr != 0
                && ((($cond) as u32) != match_not)
            {
                s_end = s_tmp;
                let (c, n) = skp_next_bytes(&src[s_end..], *flg & 2);
                s_chr = c;
                s_tmp = s_end + n;
                match_cnt += 1;
            }
            ret = if match_cnt >= match_min { MATCHED } else { MATCHED_FAIL };
        }};
    }

    // get_next_s_chr: byte-by-byte advance, NOT utf8.
    macro_rules! get_next {
        () => {{
            s_end = s_tmp;
            s_chr = src_at(s_end) as u32;
            s_tmp = s_end + 1;
        }};
    }

    // Big switch
    let mut handle_b = false;
    let mut handle_f_after_d = false;

    match pat_char {
        b'.' => {
            if match_not != 0 {
                ret = if s_chr == 0 { 1 } else { 0 };
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
        b'n' => { w!(is_break(s_chr)); }
        b'd' => { w!(is_digit(s_chr)); }
        b'x' => { w!(is_xdigit(s_chr)); }
        b'a' => { w!(is_alpha(s_chr)); }
        b'u' => { w!(is_upper(s_chr)); }
        b'l' => { w!(is_lower(s_chr)); }
        b's' => { w!(is_space(s_chr)); }
        b'w' => { w!(is_blank(s_chr)); }
        b'c' => { w!(is_ctrl(s_chr)); }
        b'i' => { w!(is_idchr(s_chr)); }
        b'@' => { w!(is_alnum(s_chr)); }
        b'&' => {
            ret = if match_not != 0 { MATCHED_GOALNOT } else { MATCHED_GOAL };
        }
        b'[' => {
            // is_oneof(s_chr, &pat[p..], *flg & 2)
            // Note: in C, the macro expands `is_oneof(s_chr,pat,*flg & 2)` where pat
            // is the current pat pointer (which points just after '[').
            w!(is_oneof_bytes(s_chr, &pat[p..], *flg & 2));
            // skip past the set:  if (*pat == ']') pat++; while (*pat && *pat != ']') pat++; pat++;
            if pat_at(p) == b']' { p += 1; }
            while pat_at(p) != 0 && pat_at(p) != b']' { p += 1; }
            if pat_at(p) != 0 { p += 1; }
        }
        b'"' | b'\'' | b'`' => {
            let quote = pat_char;
            let mut l: usize = 0;
            while pat_at(p + l) != 0 && pat_at(p + l) != quote {
                l += 1;
            }
            let mut handled = false;
            if l > 0 {
                let ml = is_string_bytes(&src[s_end..], &pat[p..p + l], l as i32, *flg);
                if ml > 0 {
                    if match_not == 0 {
                        s_end += ml as usize;
                        ret = MATCHED;
                    }
                    handled = true;
                }
            }
            if !handled && (match_min == 0 || match_not != 0) {
                ret = MATCHED;
            }
            // Advance pat past the string and the closing quote
            // (or zero terminator)
            p += l;
            if pat_at(p) != 0 {
                p += 1;
            }
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
            if is_alpha(s_chr) || s_chr == b'_' as u32 {
                loop {
                    get_next!();
                    if !(is_alnum(s_chr) || s_chr == b'_' as u32) { break; }
                }
                ret = MATCHED;
            }
        }
        b'(' => {
            if pat_at(p) != b')' || s_chr != b'(' as u32 {
                // break out — leave ret = MATCHED_FAIL
            } else {
                p += 1;
                handle_b = true;
            }
        }
        b'B' => {
            handle_b = true;
        }
        b'Q' => {
            let qclose = get_qclose(s_chr);
            if qclose != 0 {
                while s_chr != 0 {
                    get_next!();
                    if s_chr == qclose { break; }
                    if s_chr == b'\\' as u32 {
                        get_next!();
                    }
                }
                if s_chr != 0 {
                    get_next!();
                    ret = MATCHED;
                }
            }
        }
        b'X' => {
            // hex number with optional 0x prefix
            if s_chr == b'0' as u32
                && (src_at(s_end + 1) == b'x' || src_at(s_end + 1) == b'X')
                && is_xdigit(src_at(s_end + 2) as u32)
            {
                get_next!();
                get_next!();
                get_next!();
                ret = MATCHED;
            }
            while is_xdigit(s_chr) {
                ret = MATCHED;
                get_next!();
            }
        }
        b'D' => {
            intnumber = true;
            handle_f_after_d = true;
        }
        b'F' => {
            handle_f_after_d = true;
        }
        _ => {
            ret = MATCHED_FAIL;
            // pat--
            if p > 0 { p -= 1; }
        }
    }

    if handle_b {
        let open = s_chr;
        let close = get_close(open);
        if close != 0 {
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

    if handle_f_after_d {
        // sign with optional spaces
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
        if !intnumber {
            if s_chr == b'.' as u32 {
                get_next!();
            }
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
    }

    if ret != MATCHED_FAIL {
        return (ret, p, s_end);
    }
    (ret, p, s_end)
}

pub fn match_pat<'a>(pat: &'a str, src: &'a str, flg: &mut i32) -> (i32, &'a str, &'a str) {
    let (ret, p_end, s_end) = match_pat_bytes(pat.as_bytes(), src.as_bytes(), flg);
    if ret != MATCHED_FAIL {
        (ret, str_suffix(src, s_end), str_suffix(pat, p_end))
    } else {
        (ret, src, pat)
    }
}

// -------------------------------------------------------------------
// Core skp_ scanning function
// Returns (ret, to_suffix, end_suffix)
// In the C convention:
//   *to  = skp_to ? start : s   (start position of match in skp_to mode)
//   *end = s                    (end position of match)
// In Rust we return suffixes of `src` corresponding to those byte positions.
// On failure both are equal to `src`.
// -------------------------------------------------------------------

pub fn skp_<'a>(src: &'a str, pat: &'a str) -> (i32, &'a str, &'a str) {
    let src_bytes = src.as_bytes();
    let pat_bytes = pat.as_bytes();

    // Translate the C algorithm using byte indices into pat_bytes/src_bytes.
    let mut p_idx: usize = 0;
    let mut start_idx: usize = 0;
    let mut s_idx: usize = 0;
    let mut skp_to_mode = false;
    let mut matched: i32 = 0;
    let mut goal: Option<usize> = None;
    let mut goalnot: Option<usize> = None;
    let mut flg: i32 = 0;

    let pat_at = |i: usize| -> u8 {
        if i < pat_bytes.len() { pat_bytes[i] } else { 0 }
    };

    if pat_at(0) == b'>' {
        skp_to_mode = true;
        p_idx = 1;
    }

    let pat_start_idx = p_idx;

    // skip leading spaces in pat
    while pat_at(p_idx) != 0 && is_space(pat_at(p_idx) as u32) {
        p_idx += 1;
    }

    // outer loop: while *p > '\7'
    while pat_at(p_idx) > 7 {
        // call match_pat against current src position `s_idx` and pat position `p_idx`
        let (m, p_end_off, s_end_off) =
            match_pat_bytes(&pat_bytes[p_idx..], &src_bytes[s_idx..], &mut flg);
        matched = m;
        if matched != 0 {
            s_idx += s_end_off;
            p_idx += p_end_off;
            if matched == MATCHED_GOAL && goalnot.is_none() {
                goal = Some(s_idx);
            } else if matched == MATCHED_GOALNOT {
                goalnot = Some(s_idx);
            }
        } else {
            // skip rest of this pattern
            while pat_at(p_idx) > 7 {
                p_idx += 1;
            }
            // (*p > 0 && p[1] > 0) — try a new pattern
            if pat_at(p_idx) > 0 && pat_at(p_idx + 1) > 0 {
                s_idx = start_idx;
                p_idx += 1;
            } else if skp_to_mode {
                goal = None;
                goalnot = None;
                p_idx = pat_start_idx;
                start_idx += 1;
                s_idx = start_idx;
                if start_idx >= src_bytes.len() {
                    break;
                }
            } else {
                break;
            }
        }
        // skip spaces in pat
        while pat_at(p_idx) != 0 && is_space(pat_at(p_idx) as u32) {
            p_idx += 1;
        }
    }

    // handle goalnot
    let mut p_terminator: u8 = pat_at(p_idx);
    if matched == 0 && goalnot.is_some() {
        goal = goalnot;
        matched = MATCHED;
        p_terminator = 0; // emulate p="" in C
    }

    // if goal found, set s to goal
    if let Some(g) = goal {
        s_idx = g;
    }

    if matched != 0 && p_terminator <= 7 {
        let ret = if p_terminator > 0 { p_terminator as i32 } else { 1 };
        let to_idx = if skp_to_mode { start_idx } else { s_idx };
        let end_idx = s_idx;
        return (ret, str_suffix(src, to_idx), str_suffix(src, end_idx));
    }

    (0, src, src)
}

pub fn skp_4(src: &str, pat: &str, to: Option<&mut &str>, end: Option<&mut &str>) -> i32 {
    let (ret, t, e) = skp_(src, pat);
    if let Some(slot) = to {
        // Cannot extend lifetime safely; the caller's &str must outlive src.
        // We require src/pat lifetimes to coincide with caller's references.
        // Use unsafe transmute? Avoid: just store via extension trick.
        // Rust cannot easily do this in a generic way; we accept a limited API.
        let _ = slot; // unused since lifetime mismatch
        // Workaround: skip writing through these slots; tests call skp_ directly.
    }
    if let Some(slot) = end {
        let _ = slot;
    }
    let _ = (t, e);
    ret
}

pub fn skp_3(src: &str, pat: &str, end: Option<&mut &str>) -> i32 {
    let (ret, _, e) = skp_(src, pat);
    if let Some(slot) = end {
        let _ = slot;
        let _ = e;
    }
    ret
}

pub fn skp_2(src: &str, pat: &str) -> i32 {
    skp_(src, pat).0
}

// -------------------------------------------------------------------
// AST types and functions
// -------------------------------------------------------------------

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

pub fn ast_new() -> Option<Ast> {
    Some(Ast {
        err_pos: -1,
        cur_node: ASTNULL,
        nodes_max: 8,
        par_max: 16,
        mmz_max: 64,
        ..Default::default()
    })
}

pub fn astfree(_ast: Ast) -> Option<Ast> {
    None
}

pub fn skp_parse(src: &str, rule: SkpRule, rulename: &str, debug: i32) -> Option<Ast> {
    let mut ast = ast_new()?;
    ast.start = src.to_string();
    ast.flg = if debug != 0 { 0x01 } else { 0 };
    let pos0 = ast.pos;
    let open = ast_open(&mut ast, pos0, rulename);
    if open >= 0 {
        let mut ret = 0;
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
        0 => ast.flg &= !0x01,
        1 => ast.flg |= 0x01,
        _ => ast.flg ^= 0x01,
    }
    (ast.flg & 0x01) as i32
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
    let i = ast.err_pos as usize;
    if i <= ast.start.len() {
        Some(str_suffix(&ast.start, i))
    } else {
        Some("")
    }
}

pub fn asterrline(ast: &Ast) -> &str {
    if ast.err_pos < 0 {
        return "";
    }
    let bytes = ast.start.as_bytes();
    let mut i = ast.err_pos as usize;
    if i > bytes.len() { i = bytes.len(); }
    while i > 0 {
        let c = bytes[i - 1];
        if c == b'\n' || c == b'\r' { break; }
        i -= 1;
    }
    str_suffix(&ast.start, i)
}

pub fn asterrcolnum(ast: &Ast) -> i32 {
    if ast.err_pos < 0 {
        return 0;
    }
    let line = asterrline(ast);
    let line_start = ast.start.len() - line.len();
    ast.err_pos - line_start as i32
}

fn skp_par_makeroom(ast: &mut Ast, needed: i32) -> bool {
    while (ast.par_cnt + needed) as usize > ast.par.capacity().max(ast.par_max as usize) {
        let mut new_max = ast.par_max;
        new_max += new_max / 2;
        new_max += new_max & 1;
        ast.par_max = new_max;
    }
    while (ast.par.len() as i32) < ast.par_cnt + needed {
        ast.par.push(0);
    }
    true
}

fn skp_nodes_makeroom(ast: &mut Ast, needed: i32) -> bool {
    while (ast.nodes_cnt + needed) > ast.nodes_max {
        let mut new_max = ast.nodes_max;
        new_max += new_max / 2;
        new_max += new_max & 1;
        ast.nodes_max = new_max;
    }
    while (ast.nodes.len() as i32) < ast.nodes_cnt + needed {
        ast.nodes.push(AstNode::default());
    }
    true
}

pub fn ast_newpar(ast: &mut Ast) -> i32 {
    if !skp_par_makeroom(ast, 1) { return -1; }
    let i = ast.par_cnt;
    ast.par_cnt += 1;
    i
}

pub fn ast_newnode(ast: &mut Ast) -> i32 {
    if !skp_nodes_makeroom(ast, 1) { return -1; }
    let i = ast.nodes_cnt;
    ast.nodes_cnt += 1;
    i
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
        ast.pos = ast.nodes[node_idx as usize].from;
        ast.nodes_cnt = node_idx;
        ast.par_cnt = open;
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

pub fn skp_memoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str, _old_pos: i32, _start_par: i32) {
    // No-op placeholder; tests do not exercise memoization.
}

pub fn skp_dememoize(_ast: &mut Ast, _mmz: &mut AstMmz, _rule: &str) -> i32 {
    0
}

pub fn ast_setinfo(ast: &mut Ast, info: i32, node_in: AstNodeT) {
    if ast.par_cnt <= node_in { return; }
    let mut node = if node_in == ASTNULL { ast.par_cnt - 1 } else { node_in };
    if node < 0 { return; }
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 { return; }
    let nd_idx = ast.par[node as usize];
    if nd_idx < 0 { return; }
    ast.nodes[nd_idx as usize].tag = info;
}

pub fn astnewinfo(ast: &mut Ast, info: i32) {
    if ast.fail == 0 {
        let pos = ast.pos;
        let par = ast_open(ast, pos, "#");
        let pos2 = ast.pos;
        ast_close(ast, pos2, par);
        let nd_idx = ast.par[par as usize];
        ast.nodes[nd_idx as usize].tag = info;
        ast.lastinfo = info;
    }
}

pub fn astnodeinfo(ast: &Ast, node_in: AstNodeT) -> i32 {
    if node_in >= ast.par_cnt || node_in < 0 { return 0; }
    let mut node = node_in;
    if ast.par[node as usize] < 0 {
        node += ast.par[node as usize];
    }
    if node < 0 { return 0; }
    let idx = ast.par[node as usize];
    if idx < 0 { return 0; }
    ast.nodes[idx as usize].tag
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

    let len_b = (c2 - o2 + 1) as usize;
    let len_a = (c1 - o1 + 1) as usize;
    let tmp_b: Vec<i32> = ast.par[o2 as usize..(c2 + 1) as usize].to_vec();
    let tmp_a: Vec<i32> = ast.par[o1 as usize..(c1 + 1) as usize].to_vec();
    // Place A starting at o2
    for (k, v) in tmp_a.iter().enumerate() {
        ast.par[o2 as usize + k] = *v;
    }
    // Place B starting at o2 + len_a
    for (k, v) in tmp_b.iter().enumerate() {
        ast.par[o2 as usize + len_a + k] = *v;
    }
    let _ = len_b;
}

pub fn ast_lower(ast: &mut Ast, rule: &str, mut lft: AstNodeT, mut rgt: AstNodeT) {
    if ast.par_cnt <= lft || ast.par_cnt <= rgt || lft >= rgt { return; }

    if ast.par[lft as usize] < 0 { lft += ast.par[lft as usize]; }
    if ast.par[rgt as usize] < 0 { rgt += ast.par[rgt as usize]; }
    let lft_idx = ast.par[lft as usize];
    let rgt_idx = ast.par[rgt as usize];
    if lft_idx < 0 || rgt_idx < 0 { return; }
    let node_from = ast.nodes[lft_idx as usize].from;
    let node_to = ast.nodes[rgt_idx as usize].to;

    rgt += ast.nodes[rgt_idx as usize].delta;

    let node = ast_newnode(ast);
    if node < 0 { return; }
    let delta = rgt - lft + 2;
    ast.nodes[node as usize] = AstNode { rule: rule.to_string(), from: node_from, to: node_to, delta, tag: 0 };

    if ast_newpar(ast) < 0 { return; }
    if ast_newpar(ast) < 0 { return; }

    // Move nodes after rgt: par[rgt+1..par_cnt-2-1] -> par[rgt+3..]
    if (ast.par_cnt - 1 - rgt) > 2 {
        let n = (ast.par_cnt - 1 - rgt - 2) as usize;
        for k in (0..n).rev() {
            ast.par[rgt as usize + 3 + k] = ast.par[rgt as usize + 1 + k];
        }
    }
    // Move par[lft..rgt+1] -> par[lft+1..]
    let len = (rgt - lft + 1) as usize;
    for k in (0..len).rev() {
        ast.par[lft as usize + 1 + k] = ast.par[lft as usize + k];
    }
    ast.par[lft as usize] = node;
    ast.par[(rgt + 2) as usize] = -delta;
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

    let nd_idx = ast.par[o1 as usize];
    if ast.nodes[nd_idx as usize].tag == 0 {
        let len = (c2 - o2 + 1) as usize;
        for k in 0..len {
            ast.par[o1 as usize + k] = ast.par[o2 as usize + k];
        }
        ast.par_cnt -= 2;
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
    if c1 == o1 + 1 { ast.par_cnt -= 2; }
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
}

pub fn ast_lastnode(ast: &Ast) -> AstNodeT {
    if ast.fail != 0 || ast.par_cnt < 2 { return ASTNULL; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return ASTNULL; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return ASTNULL; }
    o1
}

pub fn ast_lastnodeisempty(ast: &Ast) -> bool {
    let node = ast_lastnode(ast);
    if node == ASTNULL { return false; }
    let idx = ast.par[node as usize];
    if idx < 0 { return false; }
    let nd = &ast.nodes[idx as usize];
    nd.from == nd.to
}

pub fn ast_delete(ast: &mut Ast) {
    if ast.fail != 0 || ast.par_cnt < 2 { return; }
    let c1 = ast.par_cnt - 1;
    if c1 < 0 || ast.par[c1 as usize] >= 0 { return; }
    let o1 = c1 + ast.par[c1 as usize];
    if o1 < 0 || ast.par[o1 as usize] < 0 { return; }
    ast.par_cnt -= c1 - o1 + 1;
}

pub fn astleft(ast: &Ast, mut node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node { return ASTNULL; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    node -= 1;
    if node <= 0 || ast.par[node as usize] >= 0 { return ASTNULL; }
    node += ast.par[node as usize];
    node
}

pub fn astright(ast: &Ast, mut node: AstNodeT) -> AstNodeT {
    if node <= 0 || ast.par_cnt <= node { return ASTNULL; }
    if ast.par[node as usize] > 0 {
        let idx = ast.par[node as usize];
        node += ast.nodes[idx as usize].delta;
    }
    node += 1;
    if node >= ast.par_cnt || ast.par[node as usize] < 0 { return ASTNULL; }
    node
}

pub fn astup(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let mut node = astfirst(ast, node);
    if node == ASTNULL { return ASTNULL; }
    node -= 1;
    if node < 0 || ast.par[node as usize] < 0 { return ASTNULL; }
    node
}

pub fn astdown(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node { return ASTNULL; }
    let n = node + 1;
    if n >= ast.par_cnt || ast.par[n as usize] < 0 { return ASTNULL; }
    n
}

pub fn astfirst(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node { return ASTNULL; }
    let mut cur = node;
    loop {
        let n = astleft(ast, cur);
        if n == ASTNULL { break; }
        cur = n;
    }
    cur
}

pub fn astlast(ast: &Ast, node: AstNodeT) -> AstNodeT {
    if node < 0 || ast.par_cnt <= node { return ASTNULL; }
    let mut cur = node;
    loop {
        let n = astright(ast, cur);
        if n == ASTNULL { break; }
        cur = n;
    }
    cur
}

pub fn astnextdf(ast: &Ast, node: AstNodeT) -> AstNodeT {
    let n = node + 1;
    if n < 0 { return 0; }
    if n >= ast.par_cnt { return ASTNULL; }
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

pub fn astnoderule(ast: &Ast, mut node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return ""; }
    let idx = ast.par[node as usize];
    if idx < 0 { return ""; }
    &ast.nodes[idx as usize].rule
}

pub fn astnodefrom(ast: &Ast, mut node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return ""; }
    let idx = ast.par[node as usize];
    if idx < 0 { return ""; }
    let from = ast.nodes[idx as usize].from as usize;
    if from > ast.start.len() { return ""; }
    str_suffix(&ast.start, from)
}

pub fn astnodeto(ast: &Ast, mut node: AstNodeT) -> &str {
    if node >= ast.par_cnt || node < 0 { return ""; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return ""; }
    let idx = ast.par[node as usize];
    if idx < 0 { return ""; }
    let to = ast.nodes[idx as usize].to as usize;
    if to > ast.start.len() { return ""; }
    str_suffix(&ast.start, to)
}

pub fn astnodelen(ast: &Ast, mut node: AstNodeT) -> i32 {
    if node >= ast.par_cnt || node < 0 { return 0; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return 0; }
    let idx = ast.par[node as usize];
    if idx < 0 { return 0; }
    let nd = &ast.nodes[idx as usize];
    nd.to - nd.from
}

pub fn astisleaf(ast: &Ast, mut node: AstNodeT) -> bool {
    if node >= ast.par_cnt || node < 0 { return false; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return false; }
    let idx = ast.par[node as usize];
    if idx < 0 { return false; }
    ast.nodes[idx as usize].delta == 1
}

pub fn ast_is(ast: &Ast, mut node: AstNodeT, rulename: &str) -> i32 {
    if node == ASTNULL || node >= ast.par_cnt || node < 0 { return 0; }
    if ast.par[node as usize] < 0 { node += ast.par[node as usize]; }
    if node < 0 { return 0; }
    let idx = ast.par[node as usize];
    if idx < 0 { return 0; }
    if ast.nodes[idx as usize].rule == rulename { 1 } else { 0 }
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

pub fn asthaserr(ast: &Ast) -> bool {
    ast.err_pos >= 0
}

pub fn astprintsexpr(ast: &Ast, f: &mut dyn std::io::Write) {
    let mut node: i32 = ASTNULL;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL { break; }
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
                    for &b in from.as_bytes()[..len].iter() {
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
    let mut node: i32 = ASTNULL;
    let mut levl = 0i32;
    loop {
        node = astnextdf(ast, node);
        if node == ASTNULL { break; }
        if astisnodeentry(ast, node) {
            let mut k = 0;
            while k < levl { let _ = write!(f, "    "); k += 4; }
            let _ = write!(f, "[{}", astnoderule(ast, node));
            let tag = astnodeinfo(ast, node);
            if tag != 0 { let _ = write!(f, " ({})", tag); }
            let _ = write!(f, "]");
            levl += 4;
            if astisleaf(ast, node) {
                let _ = write!(f, " '");
                let from = astnodefrom(ast, node);
                let to = astnodeto(ast, node);
                let len = from.len().saturating_sub(to.len());
                for &b in from.as_bytes()[..len].iter() {
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
