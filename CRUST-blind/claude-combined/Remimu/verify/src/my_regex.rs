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
        // mirror the _REGEX_PUSH_TOKEN macro:
        // if k == 0 OR last token's kind != current token's kind OR (kind != BOUND && kind != NBOUND)
        let k = tokens.len();
        let should_push = if k == 0 {
            true
        } else if tokens[k - 1].kind != self.kind {
            true
        } else if self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND {
            true
        } else {
            false
        };

        if should_push {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
            }
            if k >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            *self = RegexToken::new(REMIMU_KIND_NORMAL, 0);
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

pub fn regex_parse(
    pattern: &str,
    tokens: &mut Vec<RegexToken>,
    token_count: &mut i16,
    flags: i32,
) -> Result<(), i32> {
    let tokens_len = *token_count as i64;
    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    // Helper: get byte at index, or 0 if out of bounds (mimics null terminator)
    let getb = |idx: usize| -> u8 {
        if idx >= pattern_len {
            0
        } else {
            pattern_bytes[idx]
        }
    };

    if tokens_len <= 0 {
        // Mirror C: token_count == 0 returns -2
        // (and any negative is also problematic)
        if tokens_len == 0 {
            return Err(-2);
        }
    }

    tokens.clear();

    let mut esc_state: i32 = 0;

    const STATE_NORMAL: i32 = 1;
    const STATE_QUANT: i32 = 2;
    const STATE_MODE: i32 = 3;
    const STATE_CC_INIT: i32 = 4;
    const STATE_CC_NORMAL: i32 = 5;
    const STATE_CC_RANGE: i32 = 6;
    let mut state: i32 = STATE_NORMAL;

    let mut char_class_mem: i32 = -1;

    let mut token = RegexToken::new(REMIMU_KIND_NORMAL, 0);
    token.count_lo = 1;
    token.count_hi = 2;

    let max_len = tokens_len as usize;

    // start with an invisible group specifier
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pattern_bytes[i];
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
                let next = getb(i + 1);
                if next == 0 || next < b'0' || next > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while i < pattern_len && pattern_bytes[i] >= b'0' && pattern_bytes[i] <= b'9' {
                        val = val.wrapping_mul(10);
                        val = val.wrapping_add((pattern_bytes[i] - b'0') as u32);
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = (val + 1) as u16;
                    if i < pattern_len && pattern_bytes[i] == b',' {
                        token.count_hi = 0;
                        i += 1;

                        if i < pattern_len && pattern_bytes[i] >= b'0' && pattern_bytes[i] <= b'9'
                        {
                            let mut val2: u32 = 0;
                            while i < pattern_len
                                && pattern_bytes[i] >= b'0'
                                && pattern_bytes[i] <= b'9'
                            {
                                val2 = val2.wrapping_mul(10);
                                val2 = val2.wrapping_add((pattern_bytes[i] - b'0') as u32);
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

                    if i < pattern_len && pattern_bytes[i] == b'}' {
                        i += 1;
                        continue;
                    } else {
                        return Err(-1);
                    }
                }
            }
        }

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
                } else if c == b'r' {
                    token.set_mask(b'\r');
                } else if c == b't' {
                    token.set_mask(b'\t');
                } else if c == b'v' {
                    token.set_mask(0x0b);
                } else if c == b'f' {
                    token.set_mask(0x0c);
                } else if c == b'x' {
                    let p1 = getb(i + 1);
                    let p2 = getb(i + 2);
                    if p1 == 0 || p2 == 0 {
                        return Err(-1);
                    }
                    // C bug: uses pattern[i+1] for both n0 and n1
                    let mut n0 = p1;
                    let mut n1 = p1;
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
                        n0 = n0.wrapping_sub(0x20);
                    }
                    if n1 > b'F' {
                        n1 = n1.wrapping_sub(0x20);
                    }
                    if n0 >= b'A' {
                        n0 = n0.wrapping_sub(b'A' - 10);
                    }
                    if n1 >= b'A' {
                        n1 = n1.wrapping_sub(b'A' - 10);
                    }
                    n0 = n0.wrapping_sub(b'0');
                    n1 = n1.wrapping_sub(b'0');
                    token.set_mask((n1 << 4) | n0);
                    i += 2;
                } else if c == b'{'
                    || c == b'}'
                    || c == b'['
                    || c == b']'
                    || c == b'-'
                    || c == b'('
                    || c == b')'
                    || c == b'|'
                    || c == b'^'
                    || c == b'$'
                    || c == b'*'
                    || c == b'+'
                    || c == b'?'
                    || c == b':'
                    || c == b'.'
                    || c == b'/'
                    || c == b'\\'
                {
                    token.set_mask(c);
                    state = STATE_QUANT;
                } else if c == b'd'
                    || c == b's'
                    || c == b'w'
                    || c == b'D'
                    || c == b'S'
                    || c == b'W'
                {
                    let is_upper = c <= b'Z';
                    let mut m: [u16; 16] = [0; 16];
                    let mut cc = c;
                    if is_upper {
                        cc += 0x20;
                    }
                    if cc == b'd' || cc == b'w' {
                        m[3] |= 0x03FF;
                    }
                    if cc == b's' {
                        m[0] |= 0x3E00;
                        m[2] |= 1;
                    }
                    if cc == b'w' {
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
            } else {
                token.push_to_vec(tokens, max_len)?;
                if c == b'\\' {
                    esc_state = 1;
                } else if c == b'[' {
                    state = STATE_CC_INIT;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if getb(i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if c == b'(' {
                    paren_count += 1;
                    state = STATE_NORMAL;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if getb(i + 1) == b'?' && getb(i + 2) == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if getb(i + 1) == b'?' && getb(i + 2) == b'>' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.push_to_vec(tokens, max_len)?;

                        state = STATE_NORMAL;
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.count_lo = 1;
                        token.count_hi = 2;

                        i += 2;
                    }
                } else if c == b')' {
                    paren_count -= 1;
                    let k = tokens.len() as i64;
                    if paren_count < 0 || k == 0 {
                        return Err(-1);
                    }
                    token.kind = REMIMU_KIND_CLOSE;
                    state = STATE_QUANT;

                    let mut balance = 0i32;
                    let mut found: i64 = -1;
                    let mut l = k - 1;
                    while l >= 0 {
                        let tk = tokens[l as usize].kind;
                        if tk == REMIMU_KIND_NCOPEN || tk == REMIMU_KIND_OPEN {
                            if balance == 0 {
                                found = l;
                                break;
                            } else {
                                balance -= 1;
                            }
                        } else if tk == REMIMU_KIND_CLOSE {
                            balance += 1;
                        }
                        l -= 1;
                    }
                    if found == -1 {
                        return Err(-1);
                    }
                    let diff = k - found;
                    if diff > 32767 {
                        return Err(-1);
                    }
                    token.pair_offset = -(diff as i16);
                    tokens[found as usize].pair_offset = diff as i16;

                    if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                        token.push_to_vec(tokens, max_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        // tokens[found - 1].pair_offset = diff + 2
                        // Note: found - 1 should be the NCOPEN we added before the OPEN
                        if found - 1 >= 0 {
                            tokens[(found - 1) as usize].pair_offset = (diff + 2) as i16;
                        }
                    }
                } else if c == b'?' || c == b'+' || c == b'*' || c == b'{' {
                    return Err(-1);
                } else if c == b'.' {
                    for n in 0..16 {
                        token.mask[n] = 0xFFFF;
                    }
                    if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
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
            }
        } else if state == STATE_CC_INIT
            || state == STATE_CC_NORMAL
            || state == STATE_CC_RANGE
        {
            if c == b'\\' && esc_state == 0 {
                esc_state = 1;
                i += 1;
                continue;
            }
            let mut esc_c: u8 = 0;
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' {
                    esc_c = b'\n';
                } else if c == b'r' {
                    esc_c = b'\r';
                } else if c == b't' {
                    esc_c = b'\t';
                } else if c == b'v' {
                    esc_c = 0x0b;
                } else if c == b'f' {
                    esc_c = 0x0c;
                } else if c == b'x' {
                    let p1 = getb(i + 1);
                    let p2 = getb(i + 2);
                    if p1 == 0 || p2 == 0 {
                        return Err(-1);
                    }
                    let mut n0 = p1;
                    let mut n1 = p1;
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
                        n0 = n0.wrapping_sub(0x20);
                    }
                    if n1 > b'F' {
                        n1 = n1.wrapping_sub(0x20);
                    }
                    if n0 >= b'A' {
                        n0 = n0.wrapping_sub(b'A' - 10);
                    }
                    if n1 >= b'A' {
                        n1 = n1.wrapping_sub(b'A' - 10);
                    }
                    n0 = n0.wrapping_sub(b'0');
                    n1 = n1.wrapping_sub(b'0');
                    esc_c = (n1 << 4) | n0;
                    i += 2;
                } else if c == b'{'
                    || c == b'}'
                    || c == b'['
                    || c == b']'
                    || c == b'-'
                    || c == b'('
                    || c == b')'
                    || c == b'|'
                    || c == b'^'
                    || c == b'$'
                    || c == b'*'
                    || c == b'+'
                    || c == b'?'
                    || c == b':'
                    || c == b'.'
                    || c == b'/'
                    || c == b'\\'
                {
                    esc_c = c;
                } else if c == b'd'
                    || c == b's'
                    || c == b'w'
                    || c == b'D'
                    || c == b'S'
                    || c == b'W'
                {
                    if state == STATE_CC_RANGE {
                        return Err(-1);
                    }
                    let is_upper = c <= b'Z';
                    let mut m: [u16; 16] = [0; 16];
                    let mut cc = c;
                    if is_upper {
                        cc += 0x20;
                    }
                    if cc == b'd' || cc == b'w' {
                        m[3] |= 0x03FF;
                    }
                    if cc == b's' {
                        m[0] |= 0x3E00;
                        m[2] |= 1;
                    }
                    if cc == b'w' {
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
                    let mut x = c;
                    while (x as i32) > char_class_mem {
                        token.set_mask(x);
                        if x == 0 {
                            break;
                        }
                        x = x.wrapping_sub(1);
                    }
                    state = STATE_CC_NORMAL;
                    char_class_mem = -1;
                }
            }
        } else {
            // unreachable
            return Err(-1);
        }
        i += 1;
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

    token.push_to_vec(tokens, max_len)?;

    // add invisible non-capturing group specifier (CLOSE)
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, max_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len)?;

    let k = tokens.len() as i16;
    if k < 2 {
        return Err(-1);
    }
    tokens[0].pair_offset = k - 2;
    tokens[(k - 2) as usize].pair_offset = -(k - 2);

    *token_count = k;

    // copy quantifiers from )s to (s
    let mut n: u64 = 0;
    let mut k2: i16 = 0;
    while k2 < k {
        let kind = tokens[k2 as usize].kind;
        if kind == REMIMU_KIND_CLOSE {
            tokens[k2 as usize].mask[0] = n as u16;
            n += 1;

            let k3 = k2 + tokens[k2 as usize].pair_offset;
            tokens[k3 as usize].count_lo = tokens[k2 as usize].count_lo;
            tokens[k3 as usize].count_hi = tokens[k2 as usize].count_hi;
            tokens[k3 as usize].mask[0] = n as u16;
            n += 1;
            tokens[k3 as usize].mode = tokens[k2 as usize].mode;

            if n > 1024 {
                return Err(-1);
            }
        } else if kind == REMIMU_KIND_OR
            || kind == REMIMU_KIND_OPEN
            || kind == REMIMU_KIND_NCOPEN
        {
            let mut balance = 0i32;
            let mut found: i64 = -1;
            let mut l = (k2 + 1) as i64;
            while l < tokens_len {
                let lkind = tokens[l as usize].kind;
                if lkind == REMIMU_KIND_OR && balance == 0 {
                    found = l;
                    break;
                } else if lkind == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = l;
                        break;
                    } else {
                        balance -= 1;
                    }
                } else if lkind == REMIMU_KIND_NCOPEN || lkind == REMIMU_KIND_OPEN {
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

            if kind == REMIMU_KIND_OR {
                tokens[k2 as usize].pair_offset = diff as i16;
            } else {
                tokens[k2 as usize].mask[15] = diff as u16;
            }
        }
        k2 += 1;
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

fn check_mask_token(tokens: &[RegexToken], k: usize, byte: u8) -> bool {
    (tokens[k].mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
}

fn check_is_w(byte: u8) -> bool {
    let w_mask: [u64; 16] = {
        let mut m = [0u64; 16];
        m[3] = 0x03FF;
        m[4] = 0xFFFE;
        m[5] = 0x87FF;
        m[6] = 0xFFFE;
        m[7] = 0x07FF;
        m
    };
    (w_mask[(byte >> 4) as usize] & (1u64 << (byte & 0xF))) != 0
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
    // C uses null-termination; text[i] returns 0 at end. We mimic that with explicit check.
    let getc = |idx: u64| -> u8 {
        if (idx as usize) >= text_bytes.len() {
            0
        } else {
            text_bytes[idx as usize]
        }
    };

    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;
    let mut cap_slots = cap_slots;
    if (cap_slots as usize) > AUX_STATS_SIZE {
        cap_slots = AUX_STATS_SIZE as u16;
    }

    let mut q_group_accepts_zero: [u8; AUX_STATS_SIZE] = [0; AUX_STATS_SIZE];
    let mut q_group_state: [u32; AUX_STATS_SIZE] = [0; AUX_STATS_SIZE];
    let mut q_group_stack: [u32; AUX_STATS_SIZE] = [0; AUX_STATS_SIZE];

    let mut q_group_cap_index: [u16; AUX_STATS_SIZE] = [0xFFFF; AUX_STATS_SIZE];

    let mut k: u32 = 0;
    let mut caps: u16 = 0;

    // Mirror C's preprocessing loop. Note C uses `while (tokens[k].kind != END)`,
    // then checks tokens[k+1] kind in the body. The body increments k once and then
    // checks the new tokens[k] kind for OPEN/NCOPEN/CLOSE handling.
    while tokens[k as usize].kind != REMIMU_KIND_END {
        if tokens[k as usize].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let idx = tokens[k as usize].mask[0] as usize;
            q_group_cap_index[idx] = caps;
            let pair_idx = (k as i32 + tokens[k as usize].pair_offset as i32) as usize;
            q_group_cap_index[tokens[pair_idx].mask[0] as usize] = caps;
            cap_pos[caps as usize] = -1;
            cap_span[caps as usize] = -1;
            caps += 1;
        }
        k += 1;
        let kk = tokens[k as usize].kind;
        if kk == REMIMU_KIND_CLOSE || kk == REMIMU_KIND_OPEN || kk == REMIMU_KIND_NCOPEN {
            let idx = tokens[k as usize].mask[0] as usize;
            if idx >= AUX_STATS_SIZE {
                // OOM
                return None;
            }
            q_group_state[idx] = 0;
            q_group_stack[idx] = 0;
            q_group_accepts_zero[idx] = 0;
        }
    }

    let tokens_len: u64 = k as u64;

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(STACK_SIZE_MAX);
    for _ in 0..STACK_SIZE_MAX {
        rewind_stack.push(RegexMatcherState::new(0, 0));
    }
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    let mut k: u32 = 0;

    macro_rules! rewind_save_raw {
        ($K:expr, $is_dummy:expr, $stack_n:expr, $rewind_stack:expr, $tokens:expr, $i:expr, $range_min:expr, $range_max:expr, $q_group_state:expr, $q_group_stack:expr) => {{
            if $stack_n >= STACK_SIZE_MAX {
                return None;
            }
            let mut s = RegexMatcherState::new($K, $i);
            s.range_min = $range_min;
            s.range_max = $range_max;
            s.prev = 0;
            if $is_dummy {
                s.prev = 0xFAC7;
            } else if $tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                let idx = $tokens[s.k as usize].mask[0] as usize;
                s.group_state = $q_group_state[idx];
                s.prev = $q_group_stack[idx];
                $q_group_stack[idx] = $stack_n as u32;
            }
            $rewind_stack[$stack_n] = s;
            $stack_n += 1;
        }};
    }

    'outer: loop {
        if k >= tokens_len as u32 {
            break;
        }
        // Note: in the C code, the for-loop condition is k < tokens_len, but inside
        // the rewind macro, k is decremented (k -= 1) so the next k++ from for loop
        // brings it back to where we want.
        // We translate the for loop with explicit "increment at end".

        let kind = tokens[k as usize].kind;

        if kind == REMIMU_KIND_CARET {
            if i != 0 {
                // rewind or abort
                if stack_n == 0 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let idx = tokens[k as usize].mask[0] as usize;
                    q_group_state[idx] = rewind_stack[stack_n].group_state;
                    q_group_stack[idx] = rewind_stack[stack_n].prev;
                }
                // k -= 1 then continue outer (for loop k++ brings it back)
                continue 'outer;
            }
        } else if kind == REMIMU_KIND_DOLLAR {
            if getc(i) != 0 {
                if stack_n == 0 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let idx = tokens[k as usize].mask[0] as usize;
                    q_group_state[idx] = rewind_stack[stack_n].group_state;
                    q_group_stack[idx] = rewind_stack[stack_n].prev;
                }
                continue 'outer;
            }
        } else if kind == REMIMU_KIND_BOUND {
            let need_rewind = if i == 0 && !check_is_w(getc(i)) {
                true
            } else if i != 0 && getc(i) == 0 && !check_is_w(getc(i - 1)) {
                true
            } else if i != 0 && getc(i) != 0 && check_is_w(getc(i - 1)) == check_is_w(getc(i)) {
                true
            } else {
                false
            };
            if need_rewind {
                if stack_n == 0 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let idx = tokens[k as usize].mask[0] as usize;
                    q_group_state[idx] = rewind_stack[stack_n].group_state;
                    q_group_stack[idx] = rewind_stack[stack_n].prev;
                }
                continue 'outer;
            }
        } else if kind == REMIMU_KIND_NBOUND {
            let need_rewind = if i == 0 && check_is_w(getc(i)) {
                true
            } else if i != 0 && getc(i) == 0 && check_is_w(getc(i - 1)) {
                true
            } else if i != 0 && getc(i) != 0 && check_is_w(getc(i - 1)) != check_is_w(getc(i)) {
                true
            } else {
                false
            };
            if need_rewind {
                if stack_n == 0 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let idx = tokens[k as usize].mask[0] as usize;
                    q_group_state[idx] = rewind_stack[stack_n].group_state;
                    q_group_stack[idx] = rewind_stack[stack_n].prev;
                }
                continue 'outer;
            }
        } else {
            // deliberately unmatchable token (count_hi == 1)
            if tokens[k as usize].count_hi == 1 {
                if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                    k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                } else {
                    k += 1;
                }
                k += 1; // simulate for-loop increment
                continue 'outer;
            }

            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                if !just_rewinded {
                    let pair_idx =
                        (k as i32 + tokens[k as usize].pair_offset as i32) as usize;
                    let pair_mask0 = tokens[pair_idx].mask[0] as usize;
                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k as usize].count_lo == 0
                            || q_group_accepts_zero[pair_mask0] != 0)
                    {
                        range_min = 0;
                        range_max = 0;
                        rewind_save_raw!(
                            k,
                            false,
                            stack_n,
                            rewind_stack,
                            tokens,
                            i,
                            range_min,
                            range_max,
                            q_group_state,
                            q_group_stack
                        );
                        k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                    } else {
                        range_min = 1;
                        range_max = 0;
                        rewind_save_raw!(
                            k,
                            false,
                            stack_n,
                            rewind_stack,
                            tokens,
                            i,
                            range_min,
                            range_max,
                            q_group_state,
                            q_group_stack
                        );
                    }
                } else {
                    just_rewinded = false;
                    let orig_k = k as i64;

                    if range_min != 0 {
                        k = (k as u64 + range_min) as u32;
                        let prev_kind = tokens[(k - 1) as usize].kind;
                        if prev_kind == REMIMU_KIND_OR {
                            k = (k as i32 + tokens[(k - 1) as usize].pair_offset as i32 - 1)
                                as u32;
                        } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN
                        {
                            k = (k as i32 + tokens[(k - 1) as usize].mask[15] as i32 - 1) as u32;
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_END {
                            return None; // -3 invalid; treat as no match in Option API
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let idx = tokens[k as usize].mask[0] as usize;
                            if tokens[k as usize].count_lo == 0
                                || q_group_accepts_zero[idx] != 0
                            {
                                q_group_state[idx] = 0;
                                if (tokens[k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[idx] = 0;
                                }
                                k += 1;
                                continue 'outer;
                            } else {
                                if stack_n == 0 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                                    stack_n -= 1;
                                }
                                just_rewinded = true;
                                range_min = rewind_stack[stack_n].range_min;
                                range_max = rewind_stack[stack_n].range_max;
                                i = rewind_stack[stack_n].i;
                                k = rewind_stack[stack_n].k;
                                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                    let idx2 = tokens[k as usize].mask[0] as usize;
                                    q_group_state[idx2] = rewind_stack[stack_n].group_state;
                                    q_group_stack[idx2] = rewind_stack[stack_n].prev;
                                }
                                continue 'outer;
                            }
                        }
                        // assert OR
                    }

                    let k_diff = k as i64 - orig_k;
                    range_min = (k_diff + 1) as u64;

                    let save_k = (k as i64 - k_diff) as u32;
                    rewind_save_raw!(
                        save_k,
                        false,
                        stack_n,
                        rewind_stack,
                        tokens,
                        i,
                        range_min,
                        range_max,
                        q_group_state,
                        q_group_stack
                    );
                }
            } else if kind == REMIMU_KIND_CLOSE {
                if tokens[k as usize].count_lo == 1 && tokens[k as usize].count_hi == 2 {
                    // unquantified
                    let cap_index = q_group_cap_index[tokens[k as usize].mask[0] as usize];
                    if cap_index != 0xFFFF {
                        rewind_save_raw!(
                            k,
                            true,
                            stack_n,
                            rewind_stack,
                            tokens,
                            i,
                            range_min,
                            range_max,
                            q_group_state,
                            q_group_stack
                        );
                    }
                } else {
                    if !just_rewinded {
                        let prev = q_group_stack[tokens[k as usize].mask[0] as usize];
                        let group_idx = tokens[k as usize].mask[0] as usize;

                        range_max = tokens[k as usize].count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[group_idx] != 0 {
                            0
                        } else {
                            tokens[k as usize].count_lo as u64
                        };

                        // minimum requirement not yet met
                        if (q_group_state[group_idx] as u64 + 1) < range_min {
                            q_group_state[group_idx] += 1;
                            rewind_save_raw!(
                                k,
                                false,
                                stack_n,
                                rewind_stack,
                                tokens,
                                i,
                                range_min,
                                range_max,
                                q_group_state,
                                q_group_stack
                            );
                            k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                            // k -= 1 => simulate by NOT incrementing at loop end
                            continue 'outer; // continue without incrementing
                        }
                        // maximum allowance exceeded
                        else if tokens[k as usize].count_hi != 0
                            && (q_group_state[group_idx] as u64 + 1) > range_max
                        {
                            range_max = range_max.wrapping_sub(1);
                            // rewind or abort
                            if stack_n == 0 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let idx = tokens[k as usize].mask[0] as usize;
                                q_group_state[idx] = rewind_stack[stack_n].group_state;
                                q_group_stack[idx] = rewind_stack[stack_n].prev;
                            }
                            continue 'outer;
                        }

                        // detect zero-length
                        let mut force_zero: bool = false;
                        if prev != 0
                            && (rewind_stack[prev as usize].i as u32) > (i as u32)
                        {
                            // find matching open paren
                            let mut n = stack_n - 1;
                            let pair_target = (k as i32
                                + tokens[k as usize].pair_offset as i32)
                                as u32;
                            while n > 0 && rewind_stack[n].k != pair_target {
                                n -= 1;
                            }
                            if rewind_stack[n].i == i {
                                force_zero = true;
                            }
                        }

                        if force_zero
                            || (prev != 0
                                && (rewind_stack[prev as usize].i as u32) == (i as u32))
                        {
                            q_group_accepts_zero[group_idx] = 1;
                            // rewind or abort
                            if stack_n == 0 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let idx = tokens[k as usize].mask[0] as usize;
                                q_group_state[idx] = rewind_stack[stack_n].group_state;
                                q_group_stack[idx] = rewind_stack[stack_n].prev;
                            }
                            continue 'outer;
                        } else if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                            // lazy
                            q_group_state[group_idx] += 1;
                            rewind_save_raw!(
                                k,
                                false,
                                stack_n,
                                rewind_stack,
                                tokens,
                                i,
                                range_min,
                                range_max,
                                q_group_state,
                                q_group_stack
                            );
                            q_group_state[group_idx] = 0;
                        } else {
                            // greedy
                            if (tokens[k as usize].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let mut k2 = k;
                                if q_group_state[group_idx] == 0 {
                                    k2 = (k as i32 + tokens[k as usize].pair_offset as i32)
                                        as u32;
                                }

                                if stack_n == 0 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].k != k2 {
                                    stack_n -= 1;
                                }

                                if stack_n == 0 {
                                    // rewind or abort but actually return -1
                                    // Check if the current top is what we need
                                    if rewind_stack[0].k != k2 {
                                        return None;
                                    }
                                }
                            }
                            // continue to next match if sane
                            let pair_idx =
                                (k as i32 + tokens[k as usize].pair_offset as i32) as usize;
                            let pair_mask0 = tokens[pair_idx].mask[0] as usize;
                            if (q_group_state[pair_mask0] as u32) < (i as u32) {
                                q_group_state[group_idx] += 1;
                                rewind_save_raw!(
                                    k,
                                    false,
                                    stack_n,
                                    rewind_stack,
                                    tokens,
                                    i,
                                    range_min,
                                    range_max,
                                    q_group_state,
                                    q_group_stack
                                );
                                k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                                // k -= 1 -> we should NOT increment k at end
                                continue 'outer;
                            }
                        }
                    } else {
                        just_rewinded = false;
                        let group_idx = tokens[k as usize].mask[0] as usize;

                        if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                            // lazy rewind: try matching the group again
                            rewind_save_raw!(
                                k,
                                true,
                                stack_n,
                                rewind_stack,
                                tokens,
                                i,
                                range_min,
                                range_max,
                                q_group_state,
                                q_group_stack
                            );
                            q_group_stack[group_idx] = stack_n as u32;
                            k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                            continue 'outer;
                        } else {
                            // greedy
                            if (q_group_state[group_idx] as u64) < range_min
                                && q_group_accepts_zero[group_idx] == 0
                            {
                                if stack_n == 0 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                                    stack_n -= 1;
                                }
                                just_rewinded = true;
                                range_min = rewind_stack[stack_n].range_min;
                                range_max = rewind_stack[stack_n].range_max;
                                i = rewind_stack[stack_n].i;
                                k = rewind_stack[stack_n].k;
                                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                    let idx = tokens[k as usize].mask[0] as usize;
                                    q_group_state[idx] = rewind_stack[stack_n].group_state;
                                    q_group_stack[idx] = rewind_stack[stack_n].prev;
                                }
                                continue 'outer;
                            } else {
                                q_group_state[group_idx] = 0;
                                let cap_index = q_group_cap_index[group_idx];
                                if cap_index != 0xFFFF {
                                    rewind_save_raw!(
                                        k,
                                        true,
                                        stack_n,
                                        rewind_stack,
                                        tokens,
                                        i,
                                        range_min,
                                        range_max,
                                        q_group_state,
                                        q_group_stack
                                    );
                                }
                            }
                        }
                    }
                }
            } else if kind == REMIMU_KIND_OR {
                k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                // k -= 1 => continue without increment
                continue 'outer;
            } else if kind == REMIMU_KIND_NORMAL {
                if !just_rewinded {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k as usize].count_lo as u64
                        && getc(i) != 0
                        && check_mask_token(tokens, k as usize, getc(i))
                    {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k as usize].count_lo as u64 {
                        i = old_i;
                        if stack_n == 0 {
                            return None;
                        }
                        stack_n -= 1;
                        while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                            stack_n -= 1;
                        }
                        just_rewinded = true;
                        range_min = rewind_stack[stack_n].range_min;
                        range_max = rewind_stack[stack_n].range_max;
                        i = rewind_stack[stack_n].i;
                        k = rewind_stack[stack_n].k;
                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let idx = tokens[k as usize].mask[0] as usize;
                            q_group_state[idx] = rewind_stack[stack_n].group_state;
                            q_group_stack[idx] = rewind_stack[stack_n].prev;
                        }
                        continue 'outer;
                    }

                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = (tokens[k as usize].count_hi as u64).wrapping_sub(1);
                        rewind_save_raw!(
                            k,
                            false,
                            stack_n,
                            rewind_stack,
                            tokens,
                            i,
                            range_min,
                            range_max,
                            q_group_state,
                            q_group_stack
                        );
                    } else {
                        let mut limit = tokens[k as usize].count_hi as u64;
                        if limit == 0 {
                            limit = !limit;
                        }
                        range_min = n;
                        while getc(i) != 0
                            && check_mask_token(tokens, k as usize, getc(i))
                            && n + 1 < limit
                        {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k as usize].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            rewind_save_raw!(
                                k,
                                false,
                                stack_n,
                                rewind_stack,
                                tokens,
                                i,
                                range_min,
                                range_max,
                                q_group_state,
                                q_group_stack
                            );
                        }
                    }
                } else {
                    just_rewinded = false;
                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit = range_max;
                        if limit == 0 {
                            limit = !limit;
                        }
                        if check_mask_token(tokens, k as usize, getc(i))
                            && getc(i) != 0
                            && range_min < limit
                        {
                            i += 1;
                            range_min += 1;
                            rewind_save_raw!(
                                k,
                                false,
                                stack_n,
                                rewind_stack,
                                tokens,
                                i,
                                range_min,
                                range_max,
                                q_group_state,
                                q_group_stack
                            );
                        } else {
                            if stack_n == 0 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let idx = tokens[k as usize].mask[0] as usize;
                                q_group_state[idx] = rewind_stack[stack_n].group_state;
                                q_group_stack[idx] = rewind_stack[stack_n].prev;
                            }
                            continue 'outer;
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            rewind_save_raw!(
                                k,
                                false,
                                stack_n,
                                rewind_stack,
                                tokens,
                                i,
                                range_min,
                                range_max,
                                q_group_state,
                                q_group_stack
                            );
                        } else {
                            if stack_n == 0 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let idx = tokens[k as usize].mask[0] as usize;
                                q_group_state[idx] = rewind_stack[stack_n].group_state;
                                q_group_stack[idx] = rewind_stack[stack_n].prev;
                            }
                            continue 'outer;
                        }
                    }
                }
            } else {
                // unimplemented
                return None;
            }
        }
        k += 1;
    }

    // captures
    if caps != 0 {
        for n in 0..stack_n {
            let s = &rewind_stack[n];
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[s.k as usize].mask[0] as usize];
                if cap_index == 0xFFFF {
                    continue;
                }
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[cap_index as usize] = s.i as i64;
                } else if cap_pos[cap_index as usize] >= 0 {
                    cap_span[cap_index as usize] =
                        s.i as i64 - cap_pos[cap_index as usize];
                }
            }
        }
        for n in 0..(caps as usize) {
            if cap_span[n] == -1 {
                cap_pos[n] = -1;
            }
        }
    }

    Some(i as usize)
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];
    let mut k = 0usize;
    loop {
        let kind = tokens[k].kind as usize;
        let mode = tokens[k].mode as usize;
        let mode_str = if mode < mode_to_str.len() {
            mode_to_str[mode]
        } else {
            mode_to_str[0]
        };
        print!("{}\t{}\t", kind_to_str[kind], mode_str);

        let mut c_old: i32 = -1;
        let upper = if tokens[k].kind != 0 { 0 } else { 256 };
        for c in 0..upper {
            let cb = c as u8;
            let print_c = |c: i32| {
                if (0x20..=0x7E).contains(&c) {
                    print!("{}", c as u8 as char);
                } else {
                    print!("\\x{:02x}", c as u8);
                }
            };
            if check_mask_token(tokens, k, cb) {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
                if c as i32 - 1 == c_old {
                    print_c(c_old);
                    c_old = -1;
                } else if c as i32 - 2 == c_old {
                    print_c(c_old);
                    print_c(c_old + 1);
                    c_old = -1;
                } else {
                    print_c(c_old);
                    print!("-");
                    print_c(c as i32 - 1);
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
        k += 1;
    }
}
