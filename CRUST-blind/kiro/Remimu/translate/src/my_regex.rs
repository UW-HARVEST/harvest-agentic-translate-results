pub const REMIMU_FLAG_DOT_NO_NEWLINES: i32 = 1;
pub const REMIMU_KIND_NORMAL: u8 = 0;
pub const REMIMU_KIND_OPEN: u8 = 1;
pub const REMIMU_KIND_NCOPEN: u8 = 2;
pub const REMIMU_KIND_CLOSE: u8 = 3;
pub const REMIMU_KIND_OR: u8 = 4;
pub const REMIMU_KIND_CARET: u8 = 5;
pub const REMIMU_KIND_DOLLAR: u8 = 6;
pub const REMIMU_KIND_BOUND: u8 = 7;
pub const REMIMU_KIND_NBOUND: u8 = 8;
pub const REMIMU_KIND_END: u8 = 9;
pub const REMIMU_MODE_POSSESSIVE: u8 = 1;
pub const REMIMU_MODE_LAZY: u8 = 2;
pub const REMIMU_MODE_INVERTED: u8 = 128;

#[derive(Debug, Clone, Copy)]
pub struct RegexToken {
    pub kind: u8,
    pub mode: u8,
    pub count_lo: u16,
    pub count_hi: u16,
    pub mask: [u16; 16],
    pub pair_offset: i16,
}

impl RegexToken {
    pub fn new(kind: u8, mode: u8) -> Self {
        Self { kind, mode, count_lo: 1, count_hi: 2, mask: [0; 16], pair_offset: 0 }
    }
    pub fn set_mask(&mut self, byte: u8) {
        self.mask[(byte >> 4) as usize] |= 1 << (byte & 0xF);
    }
    pub fn invert_mask(&mut self) {
        for n in 0..16 { self.mask[n] = !self.mask[n]; }
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
    }
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        if tokens.is_empty()
            || tokens.last().unwrap().kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND)
        {
            if (self.mode & REMIMU_MODE_INVERTED) != 0 {
                self.invert_mask();
                self.mode &= !REMIMU_MODE_INVERTED;
            }
            if tokens.len() >= max_len { return Err(-2); }
            tokens.push(*self);
            *self = RegexToken::default();
        }
        Ok(())
    }
}

impl Default for RegexToken {
    fn default() -> Self { Self::new(REMIMU_KIND_NORMAL, 0) }
}

pub struct RegexMatcherState {
    pub k: u32,
    pub group_state: u32,
    pub prev: u32,
    pub i: u64,
    pub range_min: u64,
    pub range_max: u64,
}

impl RegexMatcherState {
    pub fn new(k: u32, i: u64) -> Self {
        Self { k, group_state: 0, prev: 0, i, range_min: 0, range_max: 0 }
    }
}

fn apply_class_mask(token: &mut RegexToken, c: u8) {
    let is_upper = c <= b'Z';
    let lc = if is_upper { c + 0x20 } else { c };
    let mut m = [0u16; 16];
    if lc == b'd' || lc == b'w' { m[3] |= 0x03FF; }
    if lc == b's' { m[0] |= 0x3E00; m[2] |= 1; }
    if lc == b'w' { m[4] |= 0xFFFE; m[5] |= 0x87FF; m[6] |= 0xFFFE; m[7] |= 0x07FF; }
    for n in 0..16 { token.mask[n] |= if is_upper { !m[n] } else { m[n] }; }
}

fn parse_hex(pat: &[u8], i: usize) -> Result<(u8, usize), i32> {
    if i + 1 >= pat.len() || i + 2 >= pat.len() { return Err(-1); }
    // C code bug: both n0 and n1 read pat[i+1]
    let mut n0 = pat[i + 1];
    let mut n1 = pat[i + 1];
    if n0 < b'0' || n0 > b'f' || n1 < b'0' || n1 > b'f'
        || (n0 > b'9' && n0 < b'A') || (n1 > b'9' && n1 < b'A') { return Err(-1); }
    if n0 > b'F' { n0 -= 0x20; }
    if n1 > b'F' { n1 -= 0x20; }
    if n0 >= b'A' { n0 -= b'A' - 10; }
    if n1 >= b'A' { n1 -= b'A' - 10; }
    n0 -= b'0';
    n1 -= b'0';
    Ok(((n1 << 4) | n0, i + 2))
}

fn is_esc_lit(c: u8) -> bool {
    matches!(c, b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$' | b'*' | b'+' | b'?' | b':' | b'.' | b'/' | b'\\')
}

pub fn regex_parse(pattern: &str, tokens: &mut Vec<RegexToken>, token_count: &mut i16, flags: i32) -> Result<(), i32> {
    let max_len = *token_count as usize;
    let pat = pattern.as_bytes();
    tokens.clear();

    const ST_NORMAL: u8 = 1;
    const ST_QUANT: u8 = 2;
    const ST_MODE: u8 = 3;
    const ST_CC_INIT: u8 = 4;
    const ST_CC_NORMAL: u8 = 5;
    const ST_CC_RANGE: u8 = 6;

    let mut esc = 0u8;
    let mut state = ST_NORMAL;
    let mut cc_mem: i32 = -1;
    let mut token = RegexToken::default();
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;
    let mut paren_count: i32 = 0;
    let mut i = 0usize;

    while i < pat.len() {
        let c = pat[i];

        if state == ST_QUANT {
            state = ST_MODE;
            match c {
                b'?' => { token.count_lo = 0; token.count_hi = 2; i += 1; continue; }
                b'+' => { token.count_lo = 1; token.count_hi = 0; i += 1; continue; }
                b'*' => { token.count_lo = 0; token.count_hi = 0; i += 1; continue; }
                b'{' => {
                    if i + 1 >= pat.len() || pat[i + 1] < b'0' || pat[i + 1] > b'9' {
                        state = ST_NORMAL;
                    } else {
                        i += 1;
                        let mut val: u32 = 0;
                        while i < pat.len() && pat[i] >= b'0' && pat[i] <= b'9' {
                            val = val * 10 + (pat[i] - b'0') as u32;
                            if val > 0xFFFF { return Err(-1); }
                            i += 1;
                        }
                        token.count_lo = val as u16;
                        token.count_hi = val as u16 + 1;
                        if i < pat.len() && pat[i] == b',' {
                            token.count_hi = 0;
                            i += 1;
                            if i < pat.len() && pat[i] >= b'0' && pat[i] <= b'9' {
                                let mut val2: u32 = 0;
                                while i < pat.len() && pat[i] >= b'0' && pat[i] <= b'9' {
                                    val2 = val2 * 10 + (pat[i] - b'0') as u32;
                                    if val2 > 0xFFFF { return Err(-1); }
                                    i += 1;
                                }
                                if val2 < val { return Err(-1); }
                                token.count_hi = val2 as u16 + 1;
                            }
                        }
                        if i < pat.len() && pat[i] == b'}' { i += 1; continue; }
                        else { return Err(-1); }
                    }
                }
                _ => {}
            }
        }

        if state == ST_MODE {
            state = ST_NORMAL;
            if c == b'?' { token.mode |= REMIMU_MODE_LAZY; i += 1; continue; }
            else if c == b'+' { token.mode |= REMIMU_MODE_POSSESSIVE; i += 1; continue; }
        }

        if state == ST_NORMAL {
            if esc == 1 {
                esc = 0;
                match c {
                    b'n' => token.set_mask(b'\n'),
                    b'r' => token.set_mask(b'\r'),
                    b't' => token.set_mask(b'\t'),
                    b'v' => token.set_mask(0x0B),
                    b'f' => token.set_mask(0x0C),
                    b'x' => { let (v, ni) = parse_hex(pat, i)?; token.set_mask(v); i = ni; }
                    _ if is_esc_lit(c) => { token.set_mask(c); state = ST_QUANT; }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        apply_class_mask(&mut token, c);
                        token.kind = REMIMU_KIND_NORMAL;
                        state = ST_QUANT;
                    }
                    b'b' => { token.kind = REMIMU_KIND_BOUND; state = ST_NORMAL; }
                    b'B' => { token.kind = REMIMU_KIND_NBOUND; state = ST_NORMAL; }
                    _ => return Err(-1),
                }
            } else {
                token.push_to_vec(tokens, max_len)?;
                match c {
                    b'\\' => { esc = 1; }
                    b'[' => {
                        state = ST_CC_INIT; cc_mem = -1; token.kind = REMIMU_KIND_NORMAL;
                        if i + 1 < pat.len() && pat[i + 1] == b'^' { token.mode |= REMIMU_MODE_INVERTED; i += 1; }
                    }
                    b'(' => {
                        paren_count += 1; state = ST_NORMAL;
                        token.kind = REMIMU_KIND_OPEN; token.count_lo = 0; token.count_hi = 1;
                        if i + 2 < pat.len() && pat[i + 1] == b'?' && pat[i + 2] == b':' {
                            token.kind = REMIMU_KIND_NCOPEN; i += 2;
                        } else if i + 2 < pat.len() && pat[i + 1] == b'?' && pat[i + 2] == b'>' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.push_to_vec(tokens, max_len)?;
                            state = ST_NORMAL;
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.count_lo = 1; token.count_hi = 2;
                            i += 2;
                        }
                    }
                    b')' => {
                        paren_count -= 1;
                        if paren_count < 0 || tokens.is_empty() { return Err(-1); }
                        token.kind = REMIMU_KIND_CLOSE; state = ST_QUANT;
                        let kk = tokens.len();
                        let mut balance = 0i32;
                        let mut found: i64 = -1;
                        for l in (0..kk).rev() {
                            if tokens[l].kind == REMIMU_KIND_NCOPEN || tokens[l].kind == REMIMU_KIND_OPEN {
                                if balance == 0 { found = l as i64; break; } else { balance -= 1; }
                            } else if tokens[l].kind == REMIMU_KIND_CLOSE { balance += 1; }
                        }
                        if found == -1 { return Err(-1); }
                        let diff = kk as i64 - found;
                        if diff > 32767 { return Err(-1); }
                        token.pair_offset = -(diff as i16);
                        tokens[found as usize].pair_offset = diff as i16;
                        if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                            token.push_to_vec(tokens, max_len)?;
                            token.kind = REMIMU_KIND_CLOSE;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.pair_offset = -(diff as i16) - 2;
                            tokens[found as usize - 1].pair_offset = diff as i16 + 2;
                        }
                    }
                    b'?' | b'+' | b'*' | b'{' => return Err(-1),
                    b'.' => {
                        for n in 0..16 { token.mask[n] = 0xFFFF; }
                        if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
                            token.mask[1] ^= 0x04;
                            token.mask[1] ^= 0x20;
                        }
                        state = ST_QUANT;
                    }
                    b'^' => { token.kind = REMIMU_KIND_CARET; state = ST_NORMAL; }
                    b'$' => { token.kind = REMIMU_KIND_DOLLAR; state = ST_NORMAL; }
                    b'|' => { token.kind = REMIMU_KIND_OR; state = ST_NORMAL; }
                    _ => { token.set_mask(c); state = ST_QUANT; }
                }
            }
        } else if state >= ST_CC_INIT {
            if c == b'\\' && esc == 0 { esc = 1; i += 1; continue; }
            let mut esc_c: u8 = 0;
            if esc == 1 {
                esc = 0;
                match c {
                    b'n' => esc_c = b'\n',
                    b'r' => esc_c = b'\r',
                    b't' => esc_c = b'\t',
                    b'v' => esc_c = 0x0B,
                    b'f' => esc_c = 0x0C,
                    b'x' => { let (v, ni) = parse_hex(pat, i)?; esc_c = v; i = ni; }
                    _ if is_esc_lit(c) => esc_c = c,
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        if state == ST_CC_RANGE { return Err(-1); }
                        apply_class_mask(&mut token, c);
                        cc_mem = -1; i += 1; continue;
                    }
                    _ => return Err(-1),
                }
            }
            if state == ST_CC_INIT {
                let eff = if esc_c != 0 { esc_c } else { c };
                cc_mem = eff as i32; token.set_mask(eff); state = ST_CC_NORMAL;
            } else if state == ST_CC_NORMAL {
                if c == b']' && esc_c == 0 { cc_mem = -1; state = ST_QUANT; }
                else if c == b'-' && esc_c == 0 && cc_mem >= 0 { state = ST_CC_RANGE; }
                else { let eff = if esc_c != 0 { esc_c } else { c }; cc_mem = eff as i32; token.set_mask(eff); }
            } else if state == ST_CC_RANGE {
                if c == b']' && esc_c == 0 { cc_mem = -1; token.set_mask(b'-'); state = ST_QUANT; }
                else {
                    let eff = if esc_c != 0 { esc_c } else { c };
                    if cc_mem == -1 { return Err(-1); }
                    if (eff as i32) < cc_mem { return Err(-1); }
                    let mut j = eff;
                    while j > cc_mem as u8 { token.set_mask(j); j -= 1; }
                    state = ST_CC_NORMAL; cc_mem = -1;
                }
            }
        }
        i += 1;
    }

    if paren_count > 0 { return Err(-1); }
    if esc != 0 { return Err(-1); }
    if state >= ST_CC_INIT { return Err(-1); }

    token.push_to_vec(tokens, max_len)?;
    token.kind = REMIMU_KIND_CLOSE; token.count_lo = 1; token.count_hi = 2;
    token.push_to_vec(tokens, max_len)?;
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len)?;

    let k = tokens.len();
    tokens[0].pair_offset = (k as i16) - 2;
    tokens[k - 2].pair_offset = -((k as i16) - 2);
    *token_count = k as i16;

    let mut n: u64 = 0;
    for k2 in 0..k {
        if tokens[k2].kind == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16; n += 1;
            let k3 = (k2 as i32 + tokens[k2].pair_offset as i32) as usize;
            tokens[k3].count_lo = tokens[k2].count_lo;
            tokens[k3].count_hi = tokens[k2].count_hi;
            tokens[k3].mask[0] = n as u16; n += 1;
            tokens[k3].mode = tokens[k2].mode;
            if n > 1024 { return Err(-1); }
        } else if tokens[k2].kind == REMIMU_KIND_OR || tokens[k2].kind == REMIMU_KIND_OPEN || tokens[k2].kind == REMIMU_KIND_NCOPEN {
            let mut balance = 0i32;
            let mut found: i64 = -1;
            for l in (k2 + 1)..k {
                if tokens[l].kind == REMIMU_KIND_OR && balance == 0 { found = l as i64; break; }
                else if tokens[l].kind == REMIMU_KIND_CLOSE {
                    if balance == 0 { found = l as i64; break; } else { balance -= 1; }
                } else if tokens[l].kind == REMIMU_KIND_NCOPEN || tokens[l].kind == REMIMU_KIND_OPEN { balance += 1; }
            }
            if found == -1 { return Err(-1); }
            let diff = found - k2 as i64;
            if diff > 32767 { return Err(-1); }
            if tokens[k2].kind == REMIMU_KIND_OR { tokens[k2].pair_offset = diff as i16; }
            else { tokens[k2].mask[15] = diff as u16; }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum State { Normal, Quant, Mode, CharClassInit, CharClassNormal, CharClassRange }

fn check_is_w(byte: u8) -> bool {
    const W: [u16; 16] = [0,0,0,0x03FF,0xFFFE,0x87FF,0xFFFE,0x07FF,0,0,0,0,0,0,0,0];
    (W[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
}

fn text_byte(text: &[u8], i: u64) -> u8 {
    let idx = i as usize;
    if idx < text.len() { text[idx] } else { 0 }
}

fn text_at_end(text: &[u8], i: u64) -> bool {
    (i as usize) >= text.len()
}

pub fn regex_match(
    tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Option<usize> {
    let text = text.as_bytes();
    let stack_max: usize = 1024;
    let aux_size: usize = 1024;
    let cap_slots = (cap_slots as usize).min(aux_size);

    let mut q_accepts_zero = vec![0u8; aux_size];
    let mut q_state = vec![0u32; aux_size];
    let mut q_stack = vec![0u32; aux_size];
    let mut q_cap_idx = vec![0xFFFFu16; aux_size];

    // init pass
    let mut tk: u32 = 0;
    let mut caps: u16 = 0;
    while tokens[tk as usize].kind != REMIMU_KIND_END {
        if tokens[tk as usize].kind == REMIMU_KIND_OPEN && (caps as usize) < cap_slots {
            let m0 = tokens[tk as usize].mask[0] as usize;
            let pair = (tk as i32 + tokens[tk as usize].pair_offset as i32) as usize;
            let m0p = tokens[pair].mask[0] as usize;
            q_cap_idx[m0] = caps;
            q_cap_idx[m0p] = caps;
            cap_pos[caps as usize] = -1;
            cap_span[caps as usize] = -1;
            caps += 1;
        }
        tk += 1;
        let kind = tokens[tk as usize].kind;
        if kind == REMIMU_KIND_CLOSE || kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
            let m0 = tokens[tk as usize].mask[0] as usize;
            if m0 >= aux_size { return None; }
            q_state[m0] = 0;
            q_stack[m0] = 0;
            q_accepts_zero[m0] = 0;
        }
    }
    let tokens_len = tk as u64;

    let mut rstack: Vec<RegexMatcherState> = Vec::with_capacity(stack_max);
    let mut sn: u16 = 0;
    let mut i: u64 = start_i as u64;
    let mut rmin: u64 = 0;
    let mut rmax: u64 = 0;
    let mut just_rw: bool = false;

    // Macro-like helpers as closures won't work due to borrow issues.
    // We'll use labeled blocks and gotos via loop/break patterns.

    macro_rules! save_raw {
        ($kv:expr, $is_dummy:expr, $sn:expr, $rstack:expr, $i:expr, $rmin:expr, $rmax:expr, $tokens:expr, $q_state:expr, $q_stack:expr, $stack_max:expr) => {{
            if ($sn as usize) >= $stack_max { return None; }
            let mut s = RegexMatcherState::new($kv, $i);
            s.range_min = $rmin;
            s.range_max = $rmax;
            s.prev = 0;
            if $is_dummy { s.prev = 0xFAC7; }
            else if $tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                let m0 = $tokens[s.k as usize].mask[0] as usize;
                s.group_state = $q_state[m0];
                s.prev = $q_stack[m0];
                $q_stack[m0] = $sn as u32;
            }
            if ($sn as usize) < $rstack.len() {
                $rstack[$sn as usize] = s;
            } else {
                $rstack.push(s);
            }
            $sn += 1;
        }};
    }

    macro_rules! rewind_or_abort {
        ($sn:expr, $rstack:expr, $just_rw:expr, $rmin:expr, $rmax:expr, $i:expr, $k:expr, $tokens:expr, $q_state:expr, $q_stack:expr) => {{
            if $sn == 0 { return None; }
            $sn -= 1;
            while $sn > 0 && $rstack[$sn as usize].prev == 0xFAC7 { $sn -= 1; }
            $just_rw = true;
            $rmin = $rstack[$sn as usize].range_min;
            $rmax = $rstack[$sn as usize].range_max;
            $i = $rstack[$sn as usize].i;
            $k = $rstack[$sn as usize].k;
            if $tokens[$k as usize].kind == REMIMU_KIND_CLOSE {
                let m0 = $tokens[$k as usize].mask[0] as usize;
                $q_state[m0] = $rstack[$sn as usize].group_state;
                $q_stack[m0] = $rstack[$sn as usize].prev;
            }
            $k = $k.wrapping_sub(1); // the k+=1 at end of loop will bring it back
        }};
    }

    let mut k: u32 = 0;
    while k < tokens_len as u32 {
        let tkind = tokens[k as usize].kind;

        if tkind == REMIMU_KIND_CARET {
            if i != 0 {
                rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
            }
            k = k.wrapping_add(1); continue;
        }
        if tkind == REMIMU_KIND_DOLLAR {
            if !text_at_end(text, i) {
                rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
            }
            k = k.wrapping_add(1); continue;
        }
        if tkind == REMIMU_KIND_BOUND {
            let cur_w = if !text_at_end(text, i) { check_is_w(text[i as usize]) } else { false };
            let prev_w = if i > 0 && !text_at_end(text, i - 1) { check_is_w(text[(i - 1) as usize]) } else { false };
            let at_bound = if i == 0 { cur_w }
                else if text_at_end(text, i) { prev_w }
                else { prev_w != cur_w };
            if !at_bound {
                rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
            }
            k = k.wrapping_add(1); continue;
        }
        if tkind == REMIMU_KIND_NBOUND {
            let cur_w = if !text_at_end(text, i) { check_is_w(text[i as usize]) } else { false };
            let prev_w = if i > 0 && !text_at_end(text, i - 1) { check_is_w(text[(i - 1) as usize]) } else { false };
            let at_bound = if i == 0 { cur_w }
                else if text_at_end(text, i) { prev_w }
                else { prev_w != cur_w };
            if at_bound {
                rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
            }
            k = k.wrapping_add(1); continue;
        }

        // deliberately unmatchable token
        if tokens[k as usize].count_hi == 1 {
            if tkind == REMIMU_KIND_OPEN || tkind == REMIMU_KIND_NCOPEN {
                k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
            } else {
                k += 1;
            }
            k = k.wrapping_add(1); continue;
        }

        if tkind == REMIMU_KIND_OPEN || tkind == REMIMU_KIND_NCOPEN {
            if !just_rw {
                let tk = &tokens[k as usize];
                let pair_m0 = tokens[(k as i32 + tk.pair_offset as i32) as usize].mask[0] as usize;
                if (tk.mode & REMIMU_MODE_LAZY) != 0
                    && (tk.count_lo == 0 || q_accepts_zero[pair_m0] != 0)
                {
                    rmin = 0; rmax = 0;
                    save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                    k = (k as i32 + tk.pair_offset as i32) as u32; // past matching )
                } else {
                    rmin = 1; rmax = 0;
                    save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                }
            } else {
                just_rw = false;
                let orig_k = k;

                if rmin != 0 {
                    k = k.wrapping_add(rmin as u32);
                    let prev_kind = tokens[(k - 1) as usize].kind;
                    if prev_kind == REMIMU_KIND_OR {
                        k = (k as i32 + tokens[(k - 1) as usize].pair_offset as i32 - 1) as u32;
                    } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN {
                        k = (k as i32 + tokens[(k - 1) as usize].mask[15] as i32 - 1) as u32;
                    }

                    if tokens[k as usize].kind == REMIMU_KIND_END { return None; }

                    if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                        let m0 = tokens[k as usize].mask[0] as usize;
                        if tokens[k as usize].count_lo == 0 || q_accepts_zero[m0] != 0 {
                            q_state[m0] = 0;
                            if (tokens[k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                q_stack[m0] = 0;
                            }
                            k = k.wrapping_add(1); continue;
                        } else {
                            rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                            k = k.wrapping_add(1); continue;
                        }
                    }
                    // must be OR
                }

                let k_diff = k as i64 - orig_k as i64;
                rmin = (k_diff + 1) as u64;
                save_raw!(orig_k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
            }
        } else if tkind == REMIMU_KIND_CLOSE {
            let tk = &tokens[k as usize];
            // unquantified
            if tk.count_lo == 1 && tk.count_hi == 2 {
                let cap_index = q_cap_idx[tk.mask[0] as usize];
                if cap_index != 0xFFFF {
                    save_raw!(k, true, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                }
            } else {
                // quantified
                if !just_rw {
                    let m0 = tk.mask[0] as usize;
                    let pair_off = tk.pair_offset;
                    let pair_m0 = tokens[(k as i32 + pair_off as i32) as usize].mask[0] as usize;

                    rmax = if tk.count_hi == 0 { 0 } else { tk.count_hi as u64 - 1 };
                    rmin = if q_accepts_zero[m0] != 0 { 0 } else { tk.count_lo as u64 };

                    let prev = q_stack[m0];

                    // minimum not yet met
                    if q_state[m0] + 1 < rmin as u32 {
                        q_state[m0] += 1;
                        save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                        k = (k as i32 + pair_off as i32) as u32;
                        k = k.wrapping_sub(1);
                        k = k.wrapping_add(1); continue;
                    }
                    // maximum exceeded
                    if tk.count_hi != 0 && q_state[m0] as u64 + 1 > rmax {
                        rmax -= 1;
                        rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                        k = k.wrapping_add(1); continue;
                    }

                    // detect zero-length match fallback
                    let mut force_zero = false;
                    if prev != 0 && rstack[prev as usize].i > i {
                        let mut n = sn as usize - 1;
                        let open_k = (k as i32 + pair_off as i32) as u32;
                        while n > 0 && rstack[n].k != open_k { n -= 1; }
                        if rstack[n].i == i { force_zero = true; }
                    }

                    if force_zero || (prev != 0 && rstack[prev as usize].i == i) {
                        q_accepts_zero[m0] = 1;
                        rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                    } else if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                        q_state[m0] += 1;
                        save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                        q_state[m0] = 0;
                    } else {
                        // greedy
                        if (tk.mode & REMIMU_MODE_POSSESSIVE) != 0 {
                            let mut k2 = k;
                            if q_state[m0] == 0 {
                                k2 = (k as i32 + pair_off as i32) as u32;
                            }
                            if sn == 0 { return None; }
                            sn -= 1;
                            while sn > 0 && rstack[sn as usize].k != k2 { sn -= 1; }
                            if sn == 0 { return None; }
                        }
                        if (q_state[pair_m0] as u64) < i {
                            q_state[m0] += 1;
                            save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                            k = (k as i32 + pair_off as i32) as u32;
                            k = k.wrapping_sub(1);
                        }
                    }
                } else {
                    // rewinded into CLOSE
                    just_rw = false;
                    let tk = &tokens[k as usize];
                    let m0 = tk.mask[0] as usize;

                    if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                        save_raw!(k, true, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                        q_stack[m0] = sn as u32;
                        let pair_off = tk.pair_offset;
                        k = (k as i32 + pair_off as i32) as u32;
                        k = k.wrapping_sub(1);
                    } else {
                        if q_state[m0] < rmin as u32 && q_accepts_zero[m0] == 0 {
                            rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                        } else {
                            q_state[m0] = 0;
                            let cap_index = q_cap_idx[tk.mask[0] as usize];
                            if cap_index != 0xFFFF {
                                save_raw!(k, true, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                            }
                        }
                    }
                }
            }
        } else if tkind == REMIMU_KIND_OR {
            let off = tokens[k as usize].pair_offset;
            k = (k as i32 + off as i32) as u32;
            k = k.wrapping_sub(1);
        } else if tkind == REMIMU_KIND_NORMAL {
            if !just_rw {
                let tk = &tokens[k as usize];
                let mut n: u64 = 0;
                let old_i = i;
                while n < tk.count_lo as u64 && !text_at_end(text, i) && tk.check_mask(text[i as usize]) {
                    i += 1; n += 1;
                }
                if n < tk.count_lo as u64 {
                    i = old_i;
                    rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                    k = k.wrapping_add(1); continue;
                }
                if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                    rmin = n; rmax = if tk.count_hi == 0 { 0 } else { tk.count_hi as u64 - 1 };
                    save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                } else {
                    let limit: u64 = if tk.count_hi == 0 { u64::MAX } else { tk.count_hi as u64 };
                    rmin = n;
                    while !text_at_end(text, i) && tk.check_mask(text[i as usize]) && n + 1 < limit {
                        i += 1; n += 1;
                    }
                    rmax = n;
                    if (tk.mode & REMIMU_MODE_POSSESSIVE) == 0 {
                        save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                    }
                }
            } else {
                just_rw = false;
                let tk = &tokens[k as usize];
                if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                    let limit = if rmax == 0 { u64::MAX } else { rmax };
                    if tk.check_mask(text_byte(text, i)) && !text_at_end(text, i) && rmin < limit {
                        i += 1; rmin += 1;
                        save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                    } else {
                        rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                    }
                } else {
                    if rmax > rmin {
                        i -= 1; rmax -= 1;
                        save_raw!(k, false, sn, rstack, i, rmin, rmax, tokens, q_state, q_stack, stack_max);
                    } else {
                        rewind_or_abort!(sn, rstack, just_rw, rmin, rmax, i, k, tokens, q_state, q_stack);
                    }
                }
            }
        }

        k = k.wrapping_add(1);
    }

    // capture extraction
    if caps != 0 {
        for n in 0..sn as usize {
            let s = &rstack[n];
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let ci = q_cap_idx[tokens[s.k as usize].mask[0] as usize];
                if ci == 0xFFFF { continue; }
                let ci = ci as usize;
                if tokens[s.k as usize].kind == REMIMU_KIND_OPEN {
                    cap_pos[ci] = s.i as i64;
                } else if cap_pos[ci] >= 0 {
                    cap_span[ci] = s.i as i64 - cap_pos[ci];
                }
            }
        }
        for n in 0..caps as usize {
            if cap_span[n] == -1 { cap_pos[n] = -1; }
        }
    }

    Some(i as usize)
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_str = ["NORMAL","OPEN","NCOPEN","CLOSE","OR","CARET","DOLLAR","BOUND","NBOUND","END"];
    let mode_str = ["GREEDY","POSSESS","LAZY"];

    for k in 0.. {
        let tk = &tokens[k];
        let ks = kind_str.get(tk.kind as usize).unwrap_or(&"?");
        let ms = mode_str.get(tk.mode as usize).unwrap_or(&"?");
        print!("{}\t{}\t", ks, ms);

        if tk.kind == REMIMU_KIND_NORMAL {
            let mut c_old: i32 = -1;
            for c in 0..=255u16 {
                let cb = c as u8;
                if tk.check_mask(cb) {
                    if c_old == -1 { c_old = c as i32; }
                } else if c_old != -1 {
                    let end = c as i32 - 1;
                    if end == c_old {
                        print_c_smart(c_old as u8);
                    } else if end == c_old + 1 {
                        print_c_smart(c_old as u8);
                        print_c_smart((c_old + 1) as u8);
                    } else {
                        print_c_smart(c_old as u8);
                        print!("-");
                        print_c_smart(end as u8);
                    }
                    c_old = -1;
                }
            }
            // handle range that extends to 255
            if c_old != -1 {
                let end = 255i32;
                if end == c_old {
                    print_c_smart(c_old as u8);
                } else if end == c_old + 1 {
                    print_c_smart(c_old as u8);
                    print_c_smart((c_old + 1) as u8);
                } else {
                    print_c_smart(c_old as u8);
                    print!("-");
                    print_c_smart(end as u8);
                }
            }
        }

        let hi_display = tk.count_hi as i32 - 1;
        println!("\t{{{},{}}}\t({})", tk.count_lo, hi_display, tk.pair_offset);

        if tk.kind == REMIMU_KIND_END { break; }
    }
}

fn print_c_smart(c: u8) {
    if c >= 0x20 && c <= 0x7E {
        print!("{}", c as char);
    } else {
        print!("\\x{:02x}", c);
    }
}
