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
        Self {
            kind,
            mode,
            count_lo: 1,
            count_hi: 2,
            mask: [0; 16],
            pair_offset: 0,
        }
    }
    pub fn set_mask(&mut self, byte: u8) {
        self.mask[(byte >> 4) as usize] |= 1u16 << (byte & 0xF);
    }
    pub fn invert_mask(&mut self) {
        for n in 0..16 {
            self.mask[n] = !self.mask[n];
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
    }
    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        let k = tokens.len();
        let allow_push = k == 0
            || tokens[k - 1].kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND);
        if allow_push {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
            }
            if k >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            self.kind = 0;
            self.mode = 0;
            self.count_lo = 1;
            self.count_hi = 2;
            self.mask = [0u16; 16];
            self.pair_offset = 0;
        }
        Ok(())
    }
}
impl Default for RegexToken {
    fn default() -> Self {
        Self::new(REMIMU_KIND_NORMAL, 0)
    }
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
        Self {
            k,
            group_state: 0,
            prev: 0,
            i,
            range_min: 0,
            range_max: 0,
        }
    }
}

const STATE_NORMAL: i32 = 1;
const STATE_QUANT: i32 = 2;
const STATE_MODE: i32 = 3;
const STATE_CC_INIT: i32 = 4;
const STATE_CC_NORMAL: i32 = 5;
const STATE_CC_RANGE: i32 = 6;

#[inline]
fn pat_at(pat: &[u8], i: usize) -> u8 {
    if i < pat.len() { pat[i] } else { 0 }
}

pub fn regex_parse(
    pattern: &str,
    tokens: &mut Vec<RegexToken>,
    token_count: &mut i16,
    flags: i32,
) -> Result<(), i32> {
    let tokens_len = *token_count as usize;
    if *token_count <= 0 {
        return Err(-2);
    }
    let pat = pattern.as_bytes();
    let pattern_len = pat.len();

    tokens.clear();

    let mut esc_state: i32 = 0;
    let mut state = STATE_NORMAL;
    let mut char_class_mem: i32 = -1;

    let mut token = RegexToken::default();

    // Start with an invisible group specifier.
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pat[i];

        // STATE_QUANT
        if state == STATE_QUANT {
            state = STATE_MODE;
            if c == b'?' {
                token.count_lo = 0;
                token.count_hi = 2;
                i += 1;
                continue;
            } else if c == b'+' {
                token.count_lo = 1;
                token.count_hi = 0;
                i += 1;
                continue;
            } else if c == b'*' {
                token.count_lo = 0;
                token.count_hi = 0;
                i += 1;
                continue;
            } else if c == b'{' {
                let next = pat_at(pat, i + 1);
                if next == 0 || next < b'0' || next > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while pat_at(pat, i) >= b'0' && pat_at(pat, i) <= b'9' {
                        val = val * 10 + (pat_at(pat, i) - b'0') as u32;
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = (val + 1) as u16;
                    if pat_at(pat, i) == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if pat_at(pat, i) >= b'0' && pat_at(pat, i) <= b'9' {
                            let mut val2: u32 = 0;
                            while pat_at(pat, i) >= b'0' && pat_at(pat, i) <= b'9' {
                                val2 = val2 * 10 + (pat_at(pat, i) - b'0') as u32;
                                if val2 > 0xFFFF {
                                    return Err(-1);
                                }
                                i += 1;
                            }
                            if val2 < val {
                                return Err(-1);
                            }
                            token.count_hi = (val2 + 1) as u16;
                        }
                    }
                    if pat_at(pat, i) == b'}' {
                        i += 1;
                        continue;
                    } else {
                        return Err(-1);
                    }
                }
            }
        }

        // STATE_MODE
        if state == STATE_MODE {
            state = STATE_NORMAL;
            if c == b'?' {
                token.mode |= REMIMU_MODE_LAZY;
                i += 1;
                continue;
            } else if c == b'+' {
                token.mode |= REMIMU_MODE_POSSESSIVE;
                i += 1;
                continue;
            }
        }

        if state == STATE_NORMAL {
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' {
                    token.set_mask(b'\n');
                    state = STATE_QUANT;
                } else if c == b'r' {
                    token.set_mask(b'\r');
                    state = STATE_QUANT;
                } else if c == b't' {
                    token.set_mask(b'\t');
                    state = STATE_QUANT;
                } else if c == b'v' {
                    token.set_mask(0x0B);
                    state = STATE_QUANT;
                } else if c == b'f' {
                    token.set_mask(0x0C);
                    state = STATE_QUANT;
                } else if c == b'x' {
                    if pat_at(pat, i + 1) == 0 || pat_at(pat, i + 2) == 0 {
                        return Err(-1);
                    }
                    let mut n0 = pat_at(pat, i + 1);
                    let mut n1 = pat_at(pat, i + 1); // bug-compatible with C source
                    if n0 < b'0'
                        || n0 > b'f'
                        || n1 < b'0'
                        || n1 > b'f'
                        || (n0 > b'9' && n0 < b'A')
                        || (n1 > b'9' && n1 < b'A')
                    {
                        return Err(-1);
                    }
                    if n0 > b'F' {
                        n0 -= 0x20;
                    }
                    if n1 > b'F' {
                        n1 -= 0x20;
                    }
                    if n0 >= b'A' {
                        n0 -= b'A' - 10;
                    }
                    if n1 >= b'A' {
                        n1 -= b'A' - 10;
                    }
                    n0 -= b'0';
                    n1 -= b'0';
                    let val = (n1 << 4) | n0;
                    token.set_mask(val);
                    i += 2;
                    state = STATE_QUANT;
                } else if matches!(
                    c,
                    b'{' | b'}'
                        | b'['
                        | b']'
                        | b'-'
                        | b'('
                        | b')'
                        | b'|'
                        | b'^'
                        | b'$'
                        | b'*'
                        | b'+'
                        | b'?'
                        | b':'
                        | b'.'
                        | b'/'
                        | b'\\'
                ) {
                    token.set_mask(c);
                    state = STATE_QUANT;
                } else if matches!(c, b'd' | b's' | b'w' | b'D' | b'S' | b'W') {
                    let is_upper = c <= b'Z';
                    let mut m = [0u16; 16];
                    let lc = if is_upper { c + 0x20 } else { c };
                    if lc == b'd' || lc == b'w' {
                        m[3] |= 0x03FF;
                    }
                    if lc == b's' {
                        m[0] |= 0x3E00;
                        m[2] |= 1;
                    }
                    if lc == b'w' {
                        m[4] |= 0xFFFE;
                        m[5] |= 0x87FF;
                        m[6] |= 0xFFFE;
                        m[7] |= 0x07FF;
                    }
                    for n in 0..16 {
                        token.mask[n] |= if is_upper { !m[n] } else { m[n] };
                    }
                    token.kind = REMIMU_KIND_NORMAL;
                    state = STATE_QUANT;
                } else if c == b'b' {
                    token.kind = REMIMU_KIND_BOUND;
                    state = STATE_NORMAL;
                } else if c == b'B' {
                    token.kind = REMIMU_KIND_NBOUND;
                    state = STATE_NORMAL;
                } else {
                    return Err(-1);
                }
                i += 1;
                continue;
            } else {
                token.push_to_vec(tokens, tokens_len)?;

                if c == b'\\' {
                    esc_state = 1;
                } else if c == b'[' {
                    state = STATE_CC_INIT;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if pat_at(pat, i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if c == b'(' {
                    paren_count += 1;
                    state = STATE_NORMAL;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if pat_at(pat, i + 1) == b'?' && pat_at(pat, i + 2) == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if pat_at(pat, i + 1) == b'?' && pat_at(pat, i + 2) == b'>' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.push_to_vec(tokens, tokens_len)?;
                        state = STATE_NORMAL;
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.count_lo = 1;
                        token.count_hi = 2;
                        i += 2;
                    }
                } else if c == b')' {
                    paren_count -= 1;
                    let k = tokens.len();
                    if paren_count < 0 || k == 0 {
                        return Err(-1);
                    }
                    token.kind = REMIMU_KIND_CLOSE;
                    state = STATE_QUANT;

                    let mut balance: i32 = 0;
                    let mut found: i64 = -1;
                    let mut l: i64 = (k as i64) - 1;
                    while l >= 0 {
                        let li = l as usize;
                        let kind = tokens[li].kind;
                        if kind == REMIMU_KIND_NCOPEN || kind == REMIMU_KIND_OPEN {
                            if balance == 0 {
                                found = l;
                                break;
                            } else {
                                balance -= 1;
                            }
                        } else if kind == REMIMU_KIND_CLOSE {
                            balance += 1;
                        }
                        l -= 1;
                    }
                    if found == -1 {
                        return Err(-1);
                    }
                    let diff = (k as i64) - found;
                    if diff > 32767 {
                        return Err(-1);
                    }
                    token.pair_offset = -(diff as i16);
                    tokens[found as usize].pair_offset = diff as i16;

                    if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                        token.push_to_vec(tokens, tokens_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        let outer = (found as usize).saturating_sub(1);
                        tokens[outer].pair_offset = diff as i16 + 2;
                    }
                } else if c == b'?' || c == b'+' || c == b'*' || c == b'{' {
                    return Err(-1);
                } else if c == b'.' {
                    for n in 0..16 {
                        token.mask[n] = 0xFFFF;
                    }
                    if flags & REMIMU_FLAG_DOT_NO_NEWLINES != 0 {
                        token.mask[1] ^= 0x04;
                        token.mask[1] ^= 0x20;
                    }
                    state = STATE_QUANT;
                } else if c == b'^' {
                    token.kind = REMIMU_KIND_CARET;
                    state = STATE_NORMAL;
                } else if c == b'$' {
                    token.kind = REMIMU_KIND_DOLLAR;
                    state = STATE_NORMAL;
                } else if c == b'|' {
                    token.kind = REMIMU_KIND_OR;
                    state = STATE_NORMAL;
                } else {
                    token.set_mask(c);
                    state = STATE_QUANT;
                }
                i += 1;
                continue;
            }
        } else if state == STATE_CC_INIT || state == STATE_CC_NORMAL || state == STATE_CC_RANGE {
            if c == b'\\' && esc_state == 0 {
                esc_state = 1;
                i += 1;
                continue;
            }
            let mut esc_c: u8 = 0;
            let mut consumed_extra: usize = 0;
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' {
                    esc_c = b'\n';
                } else if c == b'r' {
                    esc_c = b'\r';
                } else if c == b't' {
                    esc_c = b'\t';
                } else if c == b'v' {
                    esc_c = 0x0B;
                } else if c == b'f' {
                    esc_c = 0x0C;
                } else if c == b'x' {
                    if pat_at(pat, i + 1) == 0 || pat_at(pat, i + 2) == 0 {
                        return Err(-1);
                    }
                    let mut n0 = pat_at(pat, i + 1);
                    let mut n1 = pat_at(pat, i + 1); // bug-compatible
                    if n0 < b'0'
                        || n0 > b'f'
                        || n1 < b'0'
                        || n1 > b'f'
                        || (n0 > b'9' && n0 < b'A')
                        || (n1 > b'9' && n1 < b'A')
                    {
                        return Err(-1);
                    }
                    if n0 > b'F' {
                        n0 -= 0x20;
                    }
                    if n1 > b'F' {
                        n1 -= 0x20;
                    }
                    if n0 >= b'A' {
                        n0 -= b'A' - 10;
                    }
                    if n1 >= b'A' {
                        n1 -= b'A' - 10;
                    }
                    n0 -= b'0';
                    n1 -= b'0';
                    esc_c = (n1 << 4) | n0;
                    consumed_extra = 2;
                } else if matches!(
                    c,
                    b'{' | b'}'
                        | b'['
                        | b']'
                        | b'-'
                        | b'('
                        | b')'
                        | b'|'
                        | b'^'
                        | b'$'
                        | b'*'
                        | b'+'
                        | b'?'
                        | b':'
                        | b'.'
                        | b'/'
                        | b'\\'
                ) {
                    esc_c = c;
                } else if matches!(c, b'd' | b's' | b'w' | b'D' | b'S' | b'W') {
                    if state == STATE_CC_RANGE {
                        return Err(-1);
                    }
                    let is_upper = c <= b'Z';
                    let mut m = [0u16; 16];
                    let lc = if is_upper { c + 0x20 } else { c };
                    if lc == b'd' || lc == b'w' {
                        m[3] |= 0x03FF;
                    }
                    if lc == b's' {
                        m[0] |= 0x3E00;
                        m[2] |= 1;
                    }
                    if lc == b'w' {
                        m[4] |= 0xFFFE;
                        m[5] |= 0x87FF;
                        m[6] |= 0xFFFE;
                        m[7] |= 0x07FF;
                    }
                    for n in 0..16 {
                        token.mask[n] |= if is_upper { !m[n] } else { m[n] };
                    }
                    char_class_mem = -1;
                    i += 1;
                    continue;
                } else {
                    return Err(-1);
                }
            }

            i += consumed_extra;

            if state == STATE_CC_INIT {
                char_class_mem = c as i32;
                token.set_mask(c);
                state = STATE_CC_NORMAL;
            } else if state == STATE_CC_NORMAL {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    state = STATE_QUANT;
                    i += 1;
                    continue;
                } else if c == b'-' && esc_c == 0 && char_class_mem >= 0 {
                    state = STATE_CC_RANGE;
                    i += 1;
                    continue;
                } else {
                    char_class_mem = c as i32;
                    token.set_mask(c);
                    state = STATE_CC_NORMAL;
                }
            } else if state == STATE_CC_RANGE {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    token.set_mask(b'-');
                    state = STATE_QUANT;
                    i += 1;
                    continue;
                } else {
                    if char_class_mem == -1 {
                        return Err(-1);
                    }
                    if (c as i32) < char_class_mem {
                        return Err(-1);
                    }
                    let mut x = c as i32;
                    while x > char_class_mem {
                        token.set_mask(x as u8);
                        x -= 1;
                    }
                    state = STATE_CC_NORMAL;
                    char_class_mem = -1;
                }
            }
            i += 1;
            continue;
        } else {
            return Err(-1);
        }
    }

    if paren_count > 0 {
        return Err(-1);
    }
    if esc_state != 0 {
        return Err(-1);
    }
    if state >= STATE_CC_INIT {
        return Err(-1);
    }

    token.push_to_vec(tokens, tokens_len)?;

    // add invisible non-capturing group close
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, tokens_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, tokens_len)?;

    let k = tokens.len();
    if k < 2 {
        return Err(-1);
    }
    tokens[0].pair_offset = (k - 2) as i16;
    tokens[k - 2].pair_offset = -((k - 2) as i16);

    *token_count = k as i16;

    // post-process: assign group indices, copy quantifiers
    let mut n: u32 = 0;
    let total = tokens.len();
    for k2 in 0..total {
        if tokens[k2].kind == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16;
            n += 1;
            let k3 = (k2 as i64 + tokens[k2].pair_offset as i64) as usize;
            tokens[k3].count_lo = tokens[k2].count_lo;
            tokens[k3].count_hi = tokens[k2].count_hi;
            tokens[k3].mask[0] = n as u16;
            n += 1;
            tokens[k3].mode = tokens[k2].mode;
            if n > 1024 {
                return Err(-1);
            }
        } else if tokens[k2].kind == REMIMU_KIND_OR
            || tokens[k2].kind == REMIMU_KIND_OPEN
            || tokens[k2].kind == REMIMU_KIND_NCOPEN
        {
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l = k2 + 1;
            while l < total {
                let kind = tokens[l].kind;
                if kind == REMIMU_KIND_OR && balance == 0 {
                    found = l as i64;
                    break;
                } else if kind == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = l as i64;
                        break;
                    } else {
                        balance -= 1;
                    }
                } else if kind == REMIMU_KIND_NCOPEN || kind == REMIMU_KIND_OPEN {
                    balance += 1;
                }
                l += 1;
            }
            if found == -1 {
                return Err(-1);
            }
            let diff = found - k2 as i64;
            if diff > 32767 {
                return Err(-1);
            }
            if tokens[k2].kind == REMIMU_KIND_OR {
                tokens[k2].pair_offset = diff as i16;
            } else {
                tokens[k2].mask[15] = diff as u16;
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum State {
    Normal,
    Quant,
    Mode,
    CharClassInit,
    CharClassNormal,
    CharClassRange,
}

#[inline]
fn is_w_byte(b: u8) -> bool {
    // a-z, A-Z, 0-9, _
    (b >= b'a' && b <= b'z')
        || (b >= b'A' && b <= b'Z')
        || (b >= b'0' && b <= b'9')
        || b == b'_'
}

pub fn regex_match(
    tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Option<usize> {
    let text_bytes = text.as_bytes();
    let text_len = text_bytes.len();

    let stack_size_max: usize = 1024;
    let aux_stats_size: usize = 1024;
    let mut cap_slots: usize = cap_slots as usize;
    if cap_slots > aux_stats_size {
        cap_slots = aux_stats_size;
    }

    let mut q_group_accepts_zero: Vec<u8> = vec![0u8; aux_stats_size];
    let mut q_group_state: Vec<u32> = vec![0u32; aux_stats_size];
    let mut q_group_stack: Vec<u32> = vec![0u32; aux_stats_size];
    let mut q_group_cap_index: Vec<u16> = vec![0xFFFFu16; aux_stats_size];

    // Find tokens_len and initialize per-group state for each OPEN/NCOPEN/CLOSE.
    let tokens_len: usize;
    {
        let mut k: usize = 0;
        let mut caps: usize = 0;
        // Prevent runaway loop on malformed token stream.
        while k < tokens.len() && tokens[k].kind != REMIMU_KIND_END {
            if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
                let open_idx = tokens[k].mask[0] as usize;
                if open_idx >= aux_stats_size {
                    return None;
                }
                let pair = (k as i64 + tokens[k].pair_offset as i64) as usize;
                if pair >= tokens.len() {
                    return None;
                }
                let close_idx = tokens[pair].mask[0] as usize;
                if close_idx >= aux_stats_size {
                    return None;
                }
                q_group_cap_index[open_idx] = caps as u16;
                q_group_cap_index[close_idx] = caps as u16;
                if caps < cap_pos.len() {
                    cap_pos[caps] = -1;
                }
                if caps < cap_span.len() {
                    cap_span[caps] = -1;
                }
                caps += 1;
            }
            k += 1;
            if k >= tokens.len() {
                return None;
            }
            let kind = tokens[k].kind;
            if kind == REMIMU_KIND_CLOSE
                || kind == REMIMU_KIND_OPEN
                || kind == REMIMU_KIND_NCOPEN
            {
                let idx = tokens[k].mask[0] as usize;
                if idx >= aux_stats_size {
                    return None;
                }
                q_group_state[idx] = 0;
                q_group_stack[idx] = 0;
                q_group_accepts_zero[idx] = 0;
            }
        }
        tokens_len = k;
    }
    let total_caps_used: usize = {
        // count cap groups for the final cleanup
        let mut count = 0usize;
        let mut k = 0usize;
        while k < tokens_len {
            if tokens[k].kind == REMIMU_KIND_OPEN && count < cap_slots {
                count += 1;
            }
            k += 1;
        }
        count
    };

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(stack_size_max);

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    let mut k: i64 = 0;

    'main: loop {
        if k < 0 {
            return None;
        }
        let kk = k as usize;
        if kk >= tokens_len {
            break;
        }

        let kind = tokens[kk].kind;

        // Token-level dispatch
        if kind == REMIMU_KIND_CARET {
            if i != 0 {
                // rewind or abort
                if !rewind_or_abort_real(
                    &mut rewind_stack,
                    &mut k,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                    &mut q_group_state,
                    &mut q_group_stack,
                    tokens,
                ) {
                    return None;
                }
                pop_rewind(&mut rewind_stack);
                k -= 1;
            }
            // continue
            k += 1;
            continue 'main;
        } else if kind == REMIMU_KIND_DOLLAR {
            if (i as usize) < text_len {
                if !rewind_or_abort_real(
                    &mut rewind_stack,
                    &mut k,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                    &mut q_group_state,
                    &mut q_group_stack,
                    tokens,
                ) {
                    return None;
                }
                pop_rewind(&mut rewind_stack);
                k -= 1;
            }
            k += 1;
            continue 'main;
        } else if kind == REMIMU_KIND_BOUND {
            let cur_byte = if (i as usize) < text_len {
                text_bytes[i as usize]
            } else {
                0
            };
            let do_abort = if i == 0 && (i as usize) < text_len {
                !is_w_byte(cur_byte)
            } else if i == 0 && (i as usize) >= text_len {
                // i==0 && text[i]==0
                !is_w_byte(0) // false, since 0 is not w
            } else if i != 0 && (i as usize) >= text_len {
                let prev = text_bytes[(i - 1) as usize];
                !is_w_byte(prev)
            } else {
                // i != 0 && text[i] != 0
                let prev = text_bytes[(i - 1) as usize];
                is_w_byte(prev) == is_w_byte(cur_byte)
            };
            if do_abort {
                if !rewind_or_abort_real(
                    &mut rewind_stack,
                    &mut k,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                    &mut q_group_state,
                    &mut q_group_stack,
                    tokens,
                ) {
                    return None;
                }
                pop_rewind(&mut rewind_stack);
                k -= 1;
            }
            k += 1;
            continue 'main;
        } else if kind == REMIMU_KIND_NBOUND {
            let cur_byte = if (i as usize) < text_len {
                text_bytes[i as usize]
            } else {
                0
            };
            let do_abort = if i == 0 && (i as usize) < text_len {
                is_w_byte(cur_byte)
            } else if i == 0 && (i as usize) >= text_len {
                is_w_byte(0)
            } else if i != 0 && (i as usize) >= text_len {
                let prev = text_bytes[(i - 1) as usize];
                is_w_byte(prev)
            } else {
                let prev = text_bytes[(i - 1) as usize];
                is_w_byte(prev) != is_w_byte(cur_byte)
            };
            if do_abort {
                if !rewind_or_abort_real(
                    &mut rewind_stack,
                    &mut k,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                    &mut q_group_state,
                    &mut q_group_stack,
                    tokens,
                ) {
                    return None;
                }
                pop_rewind(&mut rewind_stack);
                k -= 1;
            }
            k += 1;
            continue 'main;
        }

        // count_hi == 1 means deliberately unmatchable
        if tokens[kk].count_hi == 1 {
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                k += tokens[kk].pair_offset as i64;
            } else {
                k += 1;
            }
            k += 1;
            continue 'main;
        }

        if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
            if !just_rewinded {
                let pair_idx = (kk as i64 + tokens[kk].pair_offset as i64) as usize;
                let close_mask0 = tokens[pair_idx].mask[0] as usize;
                let lazy_zero_ok = (tokens[kk].mode & REMIMU_MODE_LAZY) != 0
                    && (tokens[kk].count_lo == 0 || q_group_accepts_zero[close_mask0] != 0);
                if lazy_zero_ok {
                    range_min = 0;
                    range_max = 0;
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        false,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                    k += tokens[kk].pair_offset as i64;
                } else {
                    range_min = 1;
                    range_max = 0;
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        false,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                }
            } else {
                just_rewinded = false;
                let orig_k = k as i64;
                if range_min != 0 {
                    k += range_min as i64;
                    if k - 1 >= 0 && (k - 1) as usize <= tokens_len {
                        let prev_kind = tokens[(k - 1) as usize].kind;
                        if prev_kind == REMIMU_KIND_OR {
                            k += tokens[(k - 1) as usize].pair_offset as i64 - 1;
                        } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN {
                            k += tokens[(k - 1) as usize].mask[15] as i64 - 1;
                        }
                    }

                    if k < 0 || (k as usize) >= tokens.len() {
                        return None;
                    }
                    if tokens[k as usize].kind == REMIMU_KIND_END {
                        return None;
                    }

                    if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                        let mask0 = tokens[k as usize].mask[0] as usize;
                        if tokens[k as usize].count_lo == 0
                            || q_group_accepts_zero[mask0] != 0
                        {
                            q_group_state[mask0] = 0;
                            if (tokens[k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                q_group_stack[mask0] = 0;
                            }
                            k += 1;
                            continue 'main;
                        } else {
                            if !rewind_or_abort_real(
                                &mut rewind_stack,
                                &mut k,
                                &mut i,
                                &mut range_min,
                                &mut range_max,
                                &mut just_rewinded,
                                &mut q_group_state,
                                &mut q_group_stack,
                                tokens,
                            ) {
                                return None;
                            }
                            pop_rewind(&mut rewind_stack);
                            k -= 1;
                            k += 1;
                            continue 'main;
                        }
                    }
                    // assert tokens[k].kind == OR; if not, it's a bug, but we tolerate
                }

                let k_diff = k - orig_k;
                range_min = (k_diff + 1) as u64;
                let save_k = (k - k_diff) as u32;
                if !do_save(
                    &mut rewind_stack,
                    stack_size_max,
                    save_k,
                    i,
                    range_min,
                    range_max,
                    false,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                ) {
                    return None;
                }
            }
            k += 1;
            continue 'main;
        } else if kind == REMIMU_KIND_CLOSE {
            if tokens[kk].count_lo == 1 && tokens[kk].count_hi == 2 {
                let cap_index = q_group_cap_index[tokens[kk].mask[0] as usize];
                if cap_index != 0xFFFF {
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        true,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                }
                k += 1;
                continue 'main;
            }
            // quantified close
            if !just_rewinded {
                let mask0 = tokens[kk].mask[0] as usize;
                let prev = q_group_stack[mask0];
                let count_hi = tokens[kk].count_hi;
                let count_lo = tokens[kk].count_lo;
                let pair_off = tokens[kk].pair_offset as i64;

                range_max = count_hi as u64;
                range_max = range_max.wrapping_sub(1);
                range_min = if q_group_accepts_zero[mask0] != 0 {
                    0
                } else {
                    count_lo as u64
                };

                if (q_group_state[mask0] as u64).wrapping_add(1) < range_min {
                    q_group_state[mask0] += 1;
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        false,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                    k += pair_off;
                    k -= 1;
                    k += 1;
                    continue 'main;
                } else if count_hi != 0
                    && (q_group_state[mask0] as u64).wrapping_add(1) > range_max
                {
                    range_max = range_max.wrapping_sub(1);
                    if !rewind_or_abort_real(
                        &mut rewind_stack,
                        &mut k,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                        &mut q_group_state,
                        &mut q_group_stack,
                        tokens,
                    ) {
                        return None;
                    }
                    pop_rewind(&mut rewind_stack);
                    k -= 1;
                    k += 1;
                    continue 'main;
                }

                let mut force_zero = false;
                if prev != 0 && rewind_stack[prev as usize].i as u32 > i as u32 {
                    let mut n: i64 = rewind_stack.len() as i64 - 1;
                    let target_k = k + pair_off;
                    while n > 0 && rewind_stack[n as usize].k as i64 != target_k {
                        n -= 1;
                    }
                    if n > 0 && rewind_stack[n as usize].i == i {
                        force_zero = true;
                    }
                }

                if force_zero
                    || (prev != 0 && rewind_stack[prev as usize].i as u32 == i as u32)
                {
                    q_group_accepts_zero[mask0] = 1;
                    if !rewind_or_abort_real(
                        &mut rewind_stack,
                        &mut k,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                        &mut q_group_state,
                        &mut q_group_stack,
                        tokens,
                    ) {
                        return None;
                    }
                    pop_rewind(&mut rewind_stack);
                    k -= 1;
                    k += 1;
                    continue 'main;
                } else if (tokens[kk].mode & REMIMU_MODE_LAZY) != 0 {
                    q_group_state[mask0] += 1;
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        false,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                    q_group_state[mask0] = 0;
                } else {
                    // greedy
                    if (tokens[kk].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                        let k2_target: u32 = if q_group_state[mask0] == 0 {
                            (k + pair_off) as u32
                        } else {
                            k as u32
                        };
                        if rewind_stack.is_empty() {
                            return None;
                        }
                        rewind_stack.pop();
                        while !rewind_stack.is_empty()
                            && rewind_stack[rewind_stack.len() - 1].k != k2_target
                        {
                            rewind_stack.pop();
                        }
                        if rewind_stack.is_empty() {
                            return None;
                        }
                    }
                    let pair_mask0 = tokens[(k + pair_off) as usize].mask[0] as usize;
                    if (q_group_state[pair_mask0] as u32) < (i as u32) {
                        q_group_state[mask0] += 1;
                        if !do_save(
                            &mut rewind_stack,
                            stack_size_max,
                            k as u32,
                            i,
                            range_min,
                            range_max,
                            false,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                        ) {
                            return None;
                        }
                        k += pair_off;
                        k -= 1;
                    }
                }
            } else {
                just_rewinded = false;
                if (tokens[kk].mode & REMIMU_MODE_LAZY) != 0 {
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        true,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                    let mask0 = tokens[kk].mask[0] as usize;
                    q_group_stack[mask0] = rewind_stack.len() as u32;
                    k += tokens[kk].pair_offset as i64;
                    k -= 1;
                } else {
                    let mask0 = tokens[kk].mask[0] as usize;
                    if (q_group_state[mask0] as u64) < range_min
                        && q_group_accepts_zero[mask0] == 0
                    {
                        if !rewind_or_abort_real(
                            &mut rewind_stack,
                            &mut k,
                            &mut i,
                            &mut range_min,
                            &mut range_max,
                            &mut just_rewinded,
                            &mut q_group_state,
                            &mut q_group_stack,
                            tokens,
                        ) {
                            return None;
                        }
                        pop_rewind(&mut rewind_stack);
                        k -= 1;
                    } else {
                        q_group_state[mask0] = 0;
                        let cap_index = q_group_cap_index[mask0];
                        if cap_index != 0xFFFF {
                            if !do_save(
                                &mut rewind_stack,
                                stack_size_max,
                                k as u32,
                                i,
                                range_min,
                                range_max,
                                true,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                            ) {
                                return None;
                            }
                        }
                    }
                }
            }
            k += 1;
            continue 'main;
        } else if kind == REMIMU_KIND_OR {
            k += tokens[kk].pair_offset as i64;
            k -= 1;
            k += 1;
            continue 'main;
        } else if kind == REMIMU_KIND_NORMAL {
            if !just_rewinded {
                let mut n: u64 = 0;
                let old_i = i;
                while (n as u16) < tokens[kk].count_lo
                    && (i as usize) < text_len
                    && tokens[kk].check_mask(text_bytes[i as usize])
                {
                    i += 1;
                    n += 1;
                }
                if (n as u16) < tokens[kk].count_lo {
                    i = old_i;
                    if !rewind_or_abort_real(
                        &mut rewind_stack,
                        &mut k,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                        &mut q_group_state,
                        &mut q_group_stack,
                        tokens,
                    ) {
                        return None;
                    }
                    pop_rewind(&mut rewind_stack);
                    k -= 1;
                    k += 1;
                    continue 'main;
                }

                if (tokens[kk].mode & REMIMU_MODE_LAZY) != 0 {
                    range_min = n;
                    range_max = (tokens[kk].count_hi as u64).wrapping_sub(1);
                    if !do_save(
                        &mut rewind_stack,
                        stack_size_max,
                        k as u32,
                        i,
                        range_min,
                        range_max,
                        false,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                    ) {
                        return None;
                    }
                } else {
                    let mut limit: u64 = tokens[kk].count_hi as u64;
                    if limit == 0 {
                        limit = !0u64;
                    }
                    range_min = n;
                    while (i as usize) < text_len
                        && tokens[kk].check_mask(text_bytes[i as usize])
                        && n + 1 < limit
                    {
                        i += 1;
                        n += 1;
                    }
                    range_max = n;
                    if (tokens[kk].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                        if !do_save(
                            &mut rewind_stack,
                            stack_size_max,
                            k as u32,
                            i,
                            range_min,
                            range_max,
                            false,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                        ) {
                            return None;
                        }
                    }
                }
            } else {
                just_rewinded = false;
                if (tokens[kk].mode & REMIMU_MODE_LAZY) != 0 {
                    let mut limit = range_max;
                    if limit == 0 {
                        limit = !0u64;
                    }
                    if (i as usize) < text_len
                        && tokens[kk].check_mask(text_bytes[i as usize])
                        && range_min < limit
                    {
                        i += 1;
                        range_min += 1;
                        if !do_save(
                            &mut rewind_stack,
                            stack_size_max,
                            k as u32,
                            i,
                            range_min,
                            range_max,
                            false,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                        ) {
                            return None;
                        }
                    } else {
                        if !rewind_or_abort_real(
                            &mut rewind_stack,
                            &mut k,
                            &mut i,
                            &mut range_min,
                            &mut range_max,
                            &mut just_rewinded,
                            &mut q_group_state,
                            &mut q_group_stack,
                            tokens,
                        ) {
                            return None;
                        }
                        pop_rewind(&mut rewind_stack);
                        k -= 1;
                    }
                } else {
                    if range_max > range_min {
                        i -= 1;
                        range_max -= 1;
                        if !do_save(
                            &mut rewind_stack,
                            stack_size_max,
                            k as u32,
                            i,
                            range_min,
                            range_max,
                            false,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                        ) {
                            return None;
                        }
                    } else {
                        if !rewind_or_abort_real(
                            &mut rewind_stack,
                            &mut k,
                            &mut i,
                            &mut range_min,
                            &mut range_max,
                            &mut just_rewinded,
                            &mut q_group_state,
                            &mut q_group_stack,
                            tokens,
                        ) {
                            return None;
                        }
                        pop_rewind(&mut rewind_stack);
                        k -= 1;
                    }
                }
            }
            k += 1;
            continue 'main;
        } else {
            // unknown token kind
            return None;
        }
    }

    // If captures are requested, scan rewind stack for OPEN/CLOSE entries.
    if total_caps_used != 0 {
        for n in 0..rewind_stack.len() {
            let s = &rewind_stack[n];
            if (s.k as usize) >= tokens.len() {
                continue;
            }
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let mask0 = tokens[s.k as usize].mask[0] as usize;
                let cap_index = q_group_cap_index[mask0];
                if cap_index == 0xFFFF {
                    continue;
                }
                let ci = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    if ci < cap_pos.len() {
                        cap_pos[ci] = s.i as i64;
                    }
                } else {
                    if ci < cap_pos.len() && cap_pos[ci] >= 0 {
                        if ci < cap_span.len() {
                            cap_span[ci] = s.i as i64 - cap_pos[ci];
                        }
                    }
                }
            }
        }
        for n in 0..total_caps_used {
            if n < cap_span.len() && cap_span[n] == -1 {
                if n < cap_pos.len() {
                    cap_pos[n] = -1;
                }
            }
        }
    }

    Some(i as usize)
}

fn pop_rewind(_rewind_stack: &mut Vec<RegexMatcherState>) {
    // No-op: actual truncation happens inside rewind_or_abort.
}

fn do_save(
    rewind_stack: &mut Vec<RegexMatcherState>,
    stack_size_max: usize,
    sk: u32,
    i: u64,
    range_min: u64,
    range_max: u64,
    is_dummy: bool,
    tokens: &[RegexToken],
    q_group_state: &mut [u32],
    q_group_stack: &mut [u32],
) -> bool {
    if rewind_stack.len() >= stack_size_max {
        return false;
    }
    let mut s = RegexMatcherState {
        k: sk,
        group_state: 0,
        prev: 0,
        i,
        range_min,
        range_max,
    };
    if is_dummy {
        s.prev = 0xFAC7;
    } else if (sk as usize) < tokens.len() && tokens[sk as usize].kind == REMIMU_KIND_CLOSE {
        let mask0 = tokens[sk as usize].mask[0] as usize;
        s.group_state = q_group_state[mask0];
        s.prev = q_group_stack[mask0];
        q_group_stack[mask0] = rewind_stack.len() as u32;
    }
    rewind_stack.push(s);
    true
}

fn rewind_or_abort_real(
    rewind_stack: &mut Vec<RegexMatcherState>,
    k: &mut i64,
    i: &mut u64,
    range_min: &mut u64,
    range_max: &mut u64,
    just_rewinded: &mut bool,
    q_group_state: &mut [u32],
    q_group_stack: &mut [u32],
    tokens: &[RegexToken],
) -> bool {
    if rewind_stack.is_empty() {
        return false;
    }
    let mut sn = rewind_stack.len() - 1;
    while sn > 0 && rewind_stack[sn].prev == 0xFAC7 {
        sn -= 1;
    }
    if rewind_stack[sn].prev == 0xFAC7 {
        // All entries are dummies; no valid rewind point.
        return false;
    }
    *just_rewinded = true;
    *range_min = rewind_stack[sn].range_min;
    *range_max = rewind_stack[sn].range_max;
    *i = rewind_stack[sn].i;
    *k = rewind_stack[sn].k as i64;
    let kk = *k as usize;
    if kk < tokens.len() && tokens[kk].kind == REMIMU_KIND_CLOSE {
        let mask0 = tokens[kk].mask[0] as usize;
        q_group_state[mask0] = rewind_stack[sn].group_state;
        q_group_stack[mask0] = rewind_stack[sn].prev;
    }
    // Truncate stack to sn (excluding the entry at sn, which is consumed).
    rewind_stack.truncate(sn);
    true
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];
    for k in 0..tokens.len() {
        let kind = tokens[k].kind as usize;
        let mode = tokens[k].mode as usize;
        let kind_s = if kind < kind_to_str.len() {
            kind_to_str[kind]
        } else {
            "?"
        };
        let mode_s = if mode < mode_to_str.len() {
            mode_to_str[mode]
        } else {
            "?"
        };
        print!("{}\t{}\t", kind_s, mode_s);

        let limit = if tokens[k].kind != 0 { 0 } else { 256 };
        let mut c_old: i32 = -1;
        for c in 0..limit {
            let cb = c as u8;
            if tokens[k].check_mask(cb) {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
                if c as i32 - 1 == c_old {
                    print_c_smart(c_old as u8);
                    c_old = -1;
                } else if c as i32 - 2 == c_old {
                    print_c_smart(c_old as u8);
                    print_c_smart((c_old + 1) as u8);
                    c_old = -1;
                } else {
                    print_c_smart(c_old as u8);
                    print!("-");
                    print_c_smart((c - 1) as u8);
                    c_old = -1;
                }
            }
        }

        println!(
            "\t{{{},{}}}\t({})",
            tokens[k].count_lo,
            tokens[k].count_hi.wrapping_sub(1),
            tokens[k].pair_offset
        );

        if tokens[k].kind == REMIMU_KIND_END {
            break;
        }
    }
}

fn print_c_smart(c: u8) {
    if c >= 0x20 && c <= 0x7E {
        print!("{}", c as char);
    } else {
        print!("\\x{:02x}", c);
    }
}
