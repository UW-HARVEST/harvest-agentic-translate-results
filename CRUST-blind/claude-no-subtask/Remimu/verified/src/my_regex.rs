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
        let idx = (byte >> 4) as usize;
        let bit = byte & 0xF;
        self.mask[idx] |= 1u16 << bit;
    }
    pub fn invert_mask(&mut self) {
        for n in 0..16 {
            self.mask[n] = !self.mask[n];
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        let idx = (byte >> 4) as usize;
        let bit = byte & 0xF;
        (self.mask[idx] & (1u16 << bit)) != 0
    }
    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        let should_push = if let Some(last) = tokens.last() {
            last.kind != self.kind
                || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND)
        } else {
            true
        };
        if should_push {
            if (self.mode & REMIMU_MODE_INVERTED) != 0 {
                self.invert_mask();
            }
            if tokens.len() >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            // clear token
            *self = RegexToken {
                kind: 0,
                mode: 0,
                count_lo: 1,
                count_hi: 2,
                mask: [0; 16],
                pair_offset: 0,
            };
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

fn set_mask_helper(mask: &mut [u16; 16], byte: u8) {
    let idx = (byte >> 4) as usize;
    let bit = byte & 0xF;
    mask[idx] |= 1u16 << bit;
}

fn set_mask_all(mask: &mut [u16; 16]) {
    for n in 0..16 {
        mask[n] = 0xFFFF;
    }
}

fn parse_hex_pair(pattern: &[u8], i: usize) -> Result<u8, i32> {
    if i + 2 >= pattern.len() {
        return Err(-1);
    }
    let n0_orig = pattern[i + 1];
    let n1_orig = pattern[i + 2];
    // The C code has a bug: it sets n1 = pattern[i+1] (same as n0). We replicate that bug
    // for behavior parity? Actually, looking again, the C code also reads pattern[i+1] twice
    // for n1. Let me replicate it faithfully.
    let n0_check = n0_orig;
    let n1_check = n0_orig; // C bug: uses pattern[i+1] for n1 too
    let _ = n1_orig;

    if n0_check < b'0'
        || n0_check > b'f'
        || n1_check < b'0'
        || n1_check > b'f'
        || (n0_check > b'9' && n0_check < b'A')
        || (n1_check > b'9' && n1_check < b'A')
    {
        return Err(-1);
    }
    let mut n0 = n0_check;
    let mut n1 = n1_check;
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
    Ok((n1 << 4) | n0)
}

fn apply_shorthand(token_mask: &mut [u16; 16], c: u8) {
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
    for i in 0..16 {
        if is_upper {
            token_mask[i] |= !m[i];
        } else {
            token_mask[i] |= m[i];
        }
    }
}

fn push_token_helper(
    token: &mut RegexToken,
    tokens: &mut Vec<RegexToken>,
    max_len: usize,
) -> Result<(), i32> {
    let should_push = if let Some(last) = tokens.last() {
        last.kind != token.kind
            || (token.kind != REMIMU_KIND_BOUND && token.kind != REMIMU_KIND_NBOUND)
    } else {
        true
    };
    if should_push {
        if (token.mode & REMIMU_MODE_INVERTED) != 0 {
            for n in 0..16 {
                token.mask[n] = !token.mask[n];
            }
            token.mode &= !REMIMU_MODE_INVERTED;
        }
        if tokens.len() >= max_len {
            return Err(-2);
        }
        tokens.push(*token);
        *token = RegexToken {
            kind: 0,
            mode: 0,
            count_lo: 1,
            count_hi: 2,
            mask: [0; 16],
            pair_offset: 0,
        };
    }
    Ok(())
}

pub fn regex_parse(
    pattern: &str,
    tokens: &mut Vec<RegexToken>,
    token_count: &mut i16,
    flags: i32,
) -> Result<(), i32> {
    let tokens_len = *token_count as usize;
    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    if tokens_len == 0 {
        // C code checks `if (token_count == 0) return -2;` - this checks pointer, but token_count
        // is a reference here. Actually C checks pointer non-null. Since we have *token_count, we
        // mimic by checking if the buffer itself is empty.
        // But actually the C check is on the pointer; the buffer length check happens at push.
        // We'll just make sure pushing fails correctly.
    }

    // helper closure to read pattern byte at index, or 0 if out of bounds (mimics C null-terminated string)
    let pat_at = |i: usize| -> u8 {
        if i < pattern_len {
            pattern_bytes[i]
        } else {
            0
        }
    };

    let mut esc_state = 0;

    const STATE_NORMAL: i32 = 1;
    const STATE_QUANT: i32 = 2;
    const STATE_MODE: i32 = 3;
    const STATE_CC_INIT: i32 = 4;
    const STATE_CC_NORMAL: i32 = 5;
    const STATE_CC_RANGE: i32 = 6;

    let mut state = STATE_NORMAL;
    let mut char_class_mem: i32 = -1;

    let mut token = RegexToken {
        kind: 0,
        mode: 0,
        count_lo: 1,
        count_hi: 2,
        mask: [0; 16],
        pair_offset: 0,
    };

    // start with an invisible group specifier
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;
    tokens.clear();

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pattern_bytes[i] as char;
        let cb = pattern_bytes[i];

        if state == STATE_QUANT {
            state = STATE_MODE;
            if c == '?' {
                token.count_lo = 0;
                token.count_hi = 2;
                i += 1;
                continue;
            } else if c == '+' {
                token.count_lo = 1;
                token.count_hi = 0;
                i += 1;
                continue;
            } else if c == '*' {
                token.count_lo = 0;
                token.count_hi = 0;
                i += 1;
                continue;
            } else if c == '{' {
                let next = pat_at(i + 1);
                if next == 0 || next < b'0' || next > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while pat_at(i) >= b'0' && pat_at(i) <= b'9' {
                        val *= 10;
                        val += (pat_at(i) - b'0') as u32;
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = (val + 1) as u16;
                    if pat_at(i) == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if pat_at(i) >= b'0' && pat_at(i) <= b'9' {
                            let mut val2: u32 = 0;
                            while pat_at(i) >= b'0' && pat_at(i) <= b'9' {
                                val2 *= 10;
                                val2 += (pat_at(i) - b'0') as u32;
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
                    if pat_at(i) == b'}' {
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
            if c == '?' {
                token.mode |= REMIMU_MODE_LAZY;
                i += 1;
                continue;
            } else if c == '+' {
                token.mode |= REMIMU_MODE_POSSESSIVE;
                i += 1;
                continue;
            }
        }

        if state == STATE_NORMAL {
            if esc_state == 1 {
                esc_state = 0;
                if c == 'n' {
                    set_mask_helper(&mut token.mask, b'\n');
                    state = STATE_QUANT;
                } else if c == 'r' {
                    set_mask_helper(&mut token.mask, b'\r');
                    state = STATE_QUANT;
                } else if c == 't' {
                    set_mask_helper(&mut token.mask, b'\t');
                    state = STATE_QUANT;
                } else if c == 'v' {
                    set_mask_helper(&mut token.mask, 0x0B);
                    state = STATE_QUANT;
                } else if c == 'f' {
                    set_mask_helper(&mut token.mask, 0x0C);
                    state = STATE_QUANT;
                } else if c == 'x' {
                    let byte = parse_hex_pair(pattern_bytes, i)?;
                    set_mask_helper(&mut token.mask, byte);
                    i += 2;
                    state = STATE_QUANT;
                } else if matches!(
                    c,
                    '{' | '}'
                        | '['
                        | ']'
                        | '-'
                        | '('
                        | ')'
                        | '|'
                        | '^'
                        | '$'
                        | '*'
                        | '+'
                        | '?'
                        | ':'
                        | '.'
                        | '/'
                        | '\\'
                ) {
                    set_mask_helper(&mut token.mask, cb);
                    state = STATE_QUANT;
                } else if matches!(c, 'd' | 's' | 'w' | 'D' | 'S' | 'W') {
                    apply_shorthand(&mut token.mask, cb);
                    token.kind = REMIMU_KIND_NORMAL;
                    state = STATE_QUANT;
                } else if c == 'b' {
                    token.kind = REMIMU_KIND_BOUND;
                    state = STATE_NORMAL;
                } else if c == 'B' {
                    token.kind = REMIMU_KIND_NBOUND;
                    state = STATE_NORMAL;
                } else {
                    return Err(-1);
                }
                i += 1;
                continue;
            } else {
                push_token_helper(&mut token, tokens, tokens_len)?;
                if c == '\\' {
                    esc_state = 1;
                } else if c == '[' {
                    state = STATE_CC_INIT;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if pat_at(i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if c == '(' {
                    paren_count += 1;
                    state = STATE_NORMAL;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if pat_at(i + 1) == b'?' && pat_at(i + 2) == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if pat_at(i + 1) == b'?' && pat_at(i + 2) == b'>' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        push_token_helper(&mut token, tokens, tokens_len)?;
                        state = STATE_NORMAL;
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.count_lo = 1;
                        token.count_hi = 2;
                        i += 2;
                    }
                } else if c == ')' {
                    paren_count -= 1;
                    if paren_count < 0 || tokens.is_empty() {
                        return Err(-1);
                    }
                    token.kind = REMIMU_KIND_CLOSE;
                    state = STATE_QUANT;

                    let k_cur = tokens.len();
                    let mut balance = 0i32;
                    let mut found: Option<usize> = None;
                    let mut l = (k_cur as isize) - 1;
                    while l >= 0 {
                        let lu = l as usize;
                        if tokens[lu].kind == REMIMU_KIND_NCOPEN
                            || tokens[lu].kind == REMIMU_KIND_OPEN
                        {
                            if balance == 0 {
                                found = Some(lu);
                                break;
                            } else {
                                balance -= 1;
                            }
                        } else if tokens[lu].kind == REMIMU_KIND_CLOSE {
                            balance += 1;
                        }
                        l -= 1;
                    }
                    let found = match found {
                        Some(f) => f,
                        None => return Err(-1),
                    };
                    let diff = (k_cur - found) as i64;
                    if diff > 32767 {
                        return Err(-1);
                    }
                    token.pair_offset = -(diff as i16);
                    tokens[found].pair_offset = diff as i16;

                    // phantom group for atomic group emulation
                    if tokens[found].mode == REMIMU_MODE_POSSESSIVE {
                        push_token_helper(&mut token, tokens, tokens_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -((diff + 2) as i16);
                        // tokens[found - 1].pair_offset = diff + 2
                        if found == 0 {
                            return Err(-1);
                        }
                        tokens[found - 1].pair_offset = (diff + 2) as i16;
                    }
                } else if matches!(c, '?' | '+' | '*' | '{') {
                    return Err(-1);
                } else if c == '.' {
                    set_mask_all(&mut token.mask);
                    if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
                        token.mask[1] ^= 0x04;
                        token.mask[1] ^= 0x20;
                    }
                    state = STATE_QUANT;
                } else if c == '^' {
                    token.kind = REMIMU_KIND_CARET;
                    state = STATE_NORMAL;
                } else if c == '$' {
                    token.kind = REMIMU_KIND_DOLLAR;
                    state = STATE_NORMAL;
                } else if c == '|' {
                    token.kind = REMIMU_KIND_OR;
                    state = STATE_NORMAL;
                } else {
                    set_mask_helper(&mut token.mask, cb);
                    state = STATE_QUANT;
                }
                i += 1;
                continue;
            }
        } else if state == STATE_CC_INIT
            || state == STATE_CC_NORMAL
            || state == STATE_CC_RANGE
        {
            if c == '\\' && esc_state == 0 {
                esc_state = 1;
                i += 1;
                continue;
            }
            let mut esc_c: u8 = 0;
            let mut consumed_extra = 0usize;
            let mut is_shorthand = false;
            if esc_state == 1 {
                esc_state = 0;
                if c == 'n' {
                    esc_c = b'\n';
                } else if c == 'r' {
                    esc_c = b'\r';
                } else if c == 't' {
                    esc_c = b'\t';
                } else if c == 'v' {
                    esc_c = 0x0B;
                } else if c == 'f' {
                    esc_c = 0x0C;
                } else if c == 'x' {
                    let byte = parse_hex_pair(pattern_bytes, i)?;
                    esc_c = byte;
                    consumed_extra = 2;
                } else if matches!(
                    c,
                    '{' | '}'
                        | '['
                        | ']'
                        | '-'
                        | '('
                        | ')'
                        | '|'
                        | '^'
                        | '$'
                        | '*'
                        | '+'
                        | '?'
                        | ':'
                        | '.'
                        | '/'
                        | '\\'
                ) {
                    esc_c = cb;
                } else if matches!(c, 'd' | 's' | 'w' | 'D' | 'S' | 'W') {
                    if state == STATE_CC_RANGE {
                        return Err(-1);
                    }
                    apply_shorthand(&mut token.mask, cb);
                    char_class_mem = -1;
                    is_shorthand = true;
                } else {
                    return Err(-1);
                }
            }

            if is_shorthand {
                i += 1 + consumed_extra;
                continue;
            }

            if state == STATE_CC_INIT {
                char_class_mem = if esc_c != 0 { esc_c as i32 } else { cb as i32 };
                let to_set = if esc_c != 0 { esc_c } else { cb };
                set_mask_helper(&mut token.mask, to_set);
                state = STATE_CC_NORMAL;
            } else if state == STATE_CC_NORMAL {
                if c == ']' && esc_c == 0 {
                    char_class_mem = -1;
                    state = STATE_QUANT;
                    i += 1 + consumed_extra;
                    continue;
                } else if c == '-' && esc_c == 0 && char_class_mem >= 0 {
                    state = STATE_CC_RANGE;
                    i += 1 + consumed_extra;
                    continue;
                } else {
                    let actual = if esc_c != 0 { esc_c } else { cb };
                    char_class_mem = actual as i32;
                    set_mask_helper(&mut token.mask, actual);
                    state = STATE_CC_NORMAL;
                }
            } else if state == STATE_CC_RANGE {
                if c == ']' && esc_c == 0 {
                    char_class_mem = -1;
                    set_mask_helper(&mut token.mask, b'-');
                    state = STATE_QUANT;
                    i += 1 + consumed_extra;
                    continue;
                } else {
                    if char_class_mem == -1 {
                        return Err(-1);
                    }
                    let upper = if esc_c != 0 { esc_c } else { cb };
                    if (upper as i32) < char_class_mem {
                        return Err(-1);
                    }
                    let mut bi = upper;
                    while (bi as i32) > char_class_mem {
                        set_mask_helper(&mut token.mask, bi);
                        bi -= 1;
                    }
                    state = STATE_CC_NORMAL;
                    char_class_mem = -1;
                }
            }
            i += 1 + consumed_extra;
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

    push_token_helper(&mut token, tokens, tokens_len)?;

    // add invisible non-capturing group close
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    push_token_helper(&mut token, tokens, tokens_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    push_token_helper(&mut token, tokens, tokens_len)?;

    let k = tokens.len();
    if k < 2 {
        return Err(-1);
    }
    tokens[0].pair_offset = (k - 2) as i16;
    tokens[k - 2].pair_offset = -((k - 2) as i16);

    *token_count = k as i16;

    // copy quantifiers from )s to (s; smuggle quantified group index
    let mut n: u32 = 0;
    let k_iter = k;
    for k2 in 0..k_iter {
        let kind2 = tokens[k2].kind;
        if kind2 == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16;
            n += 1;

            let pair_offset = tokens[k2].pair_offset;
            let k3 = (k2 as isize + pair_offset as isize) as usize;
            tokens[k3].count_lo = tokens[k2].count_lo;
            tokens[k3].count_hi = tokens[k2].count_hi;
            tokens[k3].mask[0] = n as u16;
            tokens[k3].mode = tokens[k2].mode;
            n += 1;

            if n > 1024 {
                return Err(-1);
            }
        } else if kind2 == REMIMU_KIND_OR
            || kind2 == REMIMU_KIND_OPEN
            || kind2 == REMIMU_KIND_NCOPEN
        {
            // find next | or ) and how far away it is
            let mut balance = 0i32;
            let mut found: Option<usize> = None;
            let mut l = k2 + 1;
            while l < tokens_len && l < tokens.len() {
                let kl = tokens[l].kind;
                if kl == REMIMU_KIND_OR && balance == 0 {
                    found = Some(l);
                    break;
                } else if kl == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = Some(l);
                        break;
                    } else {
                        balance -= 1;
                    }
                } else if kl == REMIMU_KIND_NCOPEN || kl == REMIMU_KIND_OPEN {
                    balance += 1;
                }
                l += 1;
            }
            let found = match found {
                Some(f) => f,
                None => return Err(-1),
            };
            let diff = (found - k2) as i64;
            if diff > 32767 {
                return Err(-1);
            }
            if kind2 == REMIMU_KIND_OR {
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
    let text_at = |i: u64| -> u8 {
        let i = i as usize;
        if i < text_len {
            text_bytes[i]
        } else {
            0
        }
    };

    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;
    let mut cap_slots = cap_slots as usize;
    if cap_slots > AUX_STATS_SIZE {
        cap_slots = AUX_STATS_SIZE;
    }

    let mut q_group_accepts_zero: Vec<u8> = vec![0u8; AUX_STATS_SIZE];
    let mut q_group_state: Vec<u32> = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_stack: Vec<u32> = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_cap_index: Vec<u16> = vec![0xFFFFu16; AUX_STATS_SIZE];

    let mut k: usize = 0;
    let mut caps: usize = 0;

    while tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let mask0 = tokens[k].mask[0] as usize;
            let pair_idx = (k as isize + tokens[k].pair_offset as isize) as usize;
            let mask0_close = tokens[pair_idx].mask[0] as usize;
            if mask0 < AUX_STATS_SIZE {
                q_group_cap_index[mask0] = caps as u16;
            }
            if mask0_close < AUX_STATS_SIZE {
                q_group_cap_index[mask0_close] = caps as u16;
            }
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
            break;
        }
        let kind = tokens[k].kind;
        if kind == REMIMU_KIND_CLOSE
            || kind == REMIMU_KIND_OPEN
            || kind == REMIMU_KIND_NCOPEN
        {
            let m0 = tokens[k].mask[0] as usize;
            if m0 >= AUX_STATS_SIZE {
                return None; // -2 OOM
            }
            q_group_state[m0] = 0;
            q_group_stack[m0] = 0;
            q_group_accepts_zero[m0] = 0;
        }
    }

    let tokens_len = k;

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(STACK_SIZE_MAX);
    // We'll push states with an extra slot at index 0 to make "prev=0 means nowhere" work like C.
    // In C, stack_n is the count, and rewind_stack[stack_n-1] is top. The "prev = 0 means nowhere" works
    // because index 0 is never used as a "valid prev" in C (since when q_group_stack[..] = 0, it means no
    // prior state, but actual stack indices are stack_n which starts at 1 after first push).
    // Wait: actually in C, q_group_stack[..] = stack_n at time of save, where stack_n is index of slot
    // BEFORE increment. Let me re-read.
    //
    // C code: rewind_stack[stack_n++] = s; with q_group_stack[mask0] = stack_n (BEFORE increment).
    // So stack_n at time of assignment is the slot index where s will be stored.
    // Then the check `if (prev != 0)` means "if there was a prior save". So prev=0 actually means
    // "saved at slot 0" if we're checking at slot 0 stored as prev. Hmm.
    //
    // Actually looking more carefully: q_group_stack[..] is initialized to 0 (meaning none). When CLOSE
    // is saved, q_group_stack[..] is set to stack_n (the slot just used). Then later we do
    // s.prev = q_group_stack[..] (the previous value, which could be 0 if first save).
    // If prev == 0, it means no prior. Otherwise, rewind_stack[prev] is the prior.
    // So position 0 in stack is reserved/unused, OR there's an off-by-one.
    //
    // Looking at C: q_group_stack[mask0] = stack_n; happens AFTER stack_n++. Wait no, let me reread:
    //
    // rewind_stack[stack_n++] = s; — this stores at stack_n then increments.
    // Before this line: q_group_stack[tokens[s.k].mask[0]] = stack_n; — this stores stack_n (BEFORE increment).
    // So q_group_stack stores the slot that's about to be filled.
    // So when we save, we record old prev as s.prev, then set q_group_stack to current slot.
    // Later when CLOSE is hit again, q_group_stack[mask0] is the slot of most recent save.
    // To check if there was a prior save: if (prev != 0), but if prev=0 was a valid slot... it could
    // be ambiguous. UNLESS: stack_n starts at 0 and slot 0 is the first save. Then after first save,
    // q_group_stack = 0 (set before increment). So prev for a SECOND save is 0, which means
    // "first save was at slot 0". So the C check `if (prev != 0)` would treat "first save" as "no prior".
    // That seems like a bug... or it means "prev=0 means no prior saves, but slot 0 can also be valid".
    //
    // Hmm. Actually re-reading: q_group_stack is initialized to 0 in the loop above CLOSE handling.
    // Then on first save of CLOSE, q_group_stack[mask0] = stack_n (= 0 if first ever save). Then
    // stack_n becomes 1. On second save, prev (read first) = q_group_stack[mask0] = 0, then we set
    // q_group_stack[mask0] = stack_n (= 1). So prev=0 effectively means "the first save".
    //
    // The check `if (prev != 0)` then would treat first save's slot (0) as if no prior existed.
    // This may be intentional: rewind_stack[0] could be reserved or never accessed.
    //
    // To replicate C behavior exactly, we'll push a dummy at index 0 so real saves start at 1.

    // Push dummy at index 0
    rewind_stack.push(RegexMatcherState {
        k: 0,
        group_state: 0,
        prev: 0,
        i: 0,
        range_min: 0,
        range_max: 0,
    });
    let mut stack_n: usize = 1; // matches C's stack_n; index 0 unused

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded = false;

    // word mask for boundary checks
    let mut w_mask = [0u64; 16];
    w_mask[3] = 0x03FF;
    w_mask[4] = 0xFFFE;
    w_mask[5] = 0x87FF;
    w_mask[6] = 0xFFFE;
    w_mask[7] = 0x07FF;
    let check_is_w = |byte: u8| -> bool {
        let idx = (byte >> 4) as usize;
        let bit = byte & 0xF;
        (w_mask[idx] & (1u64 << bit)) != 0
    };

    let check_mask = |k: usize, byte: u8| -> bool {
        let idx = (byte >> 4) as usize;
        let bit = byte & 0xF;
        (tokens[k].mask[idx] & (1u16 << bit)) != 0
    };

    // helper macros translated to closures via inline code
    // We can't easily use closures because they need mutable access to many vars. Use helper fns.

    // Save state to stack
    let save_state = |stack: &mut Vec<RegexMatcherState>,
                      stack_n: &mut usize,
                      i: u64,
                      k: u32,
                      range_min: u64,
                      range_max: u64,
                      tokens: &[RegexToken],
                      q_group_state: &mut [u32],
                      q_group_stack: &mut [u32],
                      is_dummy: bool|
     -> Result<(), ()> {
        if *stack_n >= STACK_SIZE_MAX {
            return Err(());
        }
        let mut s = RegexMatcherState {
            k,
            group_state: 0,
            prev: 0,
            i,
            range_min,
            range_max,
        };
        if is_dummy {
            s.prev = 0xFAC7;
        } else if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
            let m0 = tokens[k as usize].mask[0] as usize;
            s.group_state = q_group_state[m0];
            s.prev = q_group_stack[m0];
            q_group_stack[m0] = *stack_n as u32;
        }
        if stack.len() <= *stack_n {
            stack.push(s);
        } else {
            stack[*stack_n] = s;
        }
        *stack_n += 1;
        Ok(())
    };

    let mut iter_limit: usize = 10_000_000;

    loop {
        if k >= tokens_len {
            break;
        }
        if iter_limit == 0 {
            return None; // -2
        }
        iter_limit -= 1;

        let kind = tokens[k].kind;

        if kind == REMIMU_KIND_CARET {
            if i != 0 {
                // rewind
                if stack_n <= 1 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                if stack_n < 1 {
                    return None;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue; // k -= 1 in C; for loop adds 1; we use continue without k+=1
            } else {
                k += 1;
                continue;
            }
        } else if kind == REMIMU_KIND_DOLLAR {
            if text_at(i) != 0 {
                // rewind
                if stack_n <= 1 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                if stack_n < 1 {
                    return None;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue;
            } else {
                k += 1;
                continue;
            }
        } else if kind == REMIMU_KIND_BOUND {
            let cur = text_at(i);
            let mut should_rewind = false;
            if i == 0 && !check_is_w(cur) {
                should_rewind = true;
            } else if i != 0 && cur == 0 && !check_is_w(text_at(i - 1)) {
                should_rewind = true;
            } else if i != 0 && cur != 0 && check_is_w(text_at(i - 1)) == check_is_w(cur) {
                should_rewind = true;
            }
            if should_rewind {
                if stack_n <= 1 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                if stack_n < 1 {
                    return None;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue;
            } else {
                k += 1;
                continue;
            }
        } else if kind == REMIMU_KIND_NBOUND {
            let cur = text_at(i);
            let mut should_rewind = false;
            if i == 0 && check_is_w(cur) {
                should_rewind = true;
            } else if i != 0 && cur == 0 && check_is_w(text_at(i - 1)) {
                should_rewind = true;
            } else if i != 0 && cur != 0 && check_is_w(text_at(i - 1)) != check_is_w(cur) {
                should_rewind = true;
            }
            if should_rewind {
                if stack_n <= 1 {
                    return None;
                }
                stack_n -= 1;
                while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                    stack_n -= 1;
                }
                if stack_n < 1 {
                    return None;
                }
                just_rewinded = true;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue;
            } else {
                k += 1;
                continue;
            }
        } else {
            // unmatchable token
            if tokens[k].count_hi == 1 {
                if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                    k = (k as isize + tokens[k].pair_offset as isize) as usize;
                } else {
                    k += 1;
                }
                k += 1;
                continue;
            }

            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                if !just_rewinded {
                    let pair_idx = (k as isize + tokens[k].pair_offset as isize) as usize;
                    let pair_m0 = tokens[pair_idx].mask[0] as usize;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k].count_lo == 0 || q_group_accepts_zero[pair_m0] != 0)
                    {
                        range_min = 0;
                        range_max = 0;
                        if save_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            i,
                            k as u32,
                            range_min,
                            range_max,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                            false,
                        )
                        .is_err()
                        {
                            return None;
                        }
                        // jump past matching )
                        k = (k as isize + tokens[k].pair_offset as isize) as usize;
                        k += 1;
                        continue;
                    } else {
                        range_min = 1;
                        range_max = 0;
                        if save_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            i,
                            k as u32,
                            range_min,
                            range_max,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                            false,
                        )
                        .is_err()
                        {
                            return None;
                        }
                    }
                } else {
                    just_rewinded = false;
                    let orig_k = k;

                    if range_min != 0 {
                        k = (k as u64 + range_min) as usize;
                        if k == 0 {
                            return None;
                        }
                        if tokens[k - 1].kind == REMIMU_KIND_OR {
                            k = (k as isize + tokens[k - 1].pair_offset as isize - 1) as usize;
                        } else if tokens[k - 1].kind == REMIMU_KIND_OPEN
                            || tokens[k - 1].kind == REMIMU_KIND_NCOPEN
                        {
                            k = (k as i64 + tokens[k - 1].mask[15] as i64 - 1) as usize;
                        }

                        if tokens[k].kind == REMIMU_KIND_END {
                            return None; // -3
                        }

                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            let m0 = tokens[k].mask[0] as usize;
                            if tokens[k].count_lo == 0 || q_group_accepts_zero[m0] != 0 {
                                q_group_state[m0] = 0;
                                if (tokens[k].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[m0] = 0;
                                }
                                k += 1;
                                continue;
                            } else {
                                if stack_n <= 1 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                                    stack_n -= 1;
                                }
                                if stack_n < 1 {
                                    return None;
                                }
                                just_rewinded = true;
                                range_min = rewind_stack[stack_n].range_min;
                                range_max = rewind_stack[stack_n].range_max;
                                i = rewind_stack[stack_n].i;
                                k = rewind_stack[stack_n].k as usize;
                                if tokens[k].kind == REMIMU_KIND_CLOSE {
                                    let m0 = tokens[k].mask[0] as usize;
                                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                                }
                                continue;
                            }
                        }
                        // else assert OR
                    }

                    let k_diff = (k as i64) - (orig_k as i64);
                    range_min = (k_diff + 1) as u64;

                    let save_k = (k as i64 - k_diff) as u32;
                    if save_state(
                        &mut rewind_stack,
                        &mut stack_n,
                        i,
                        save_k,
                        range_min,
                        range_max,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                        false,
                    )
                    .is_err()
                    {
                        return None;
                    }
                }
                k += 1;
                continue;
            } else if kind == REMIMU_KIND_CLOSE {
                if tokens[k].count_lo == 1 && tokens[k].count_hi == 2 {
                    let cap_index = q_group_cap_index[tokens[k].mask[0] as usize];
                    if cap_index != 0xFFFF {
                        if save_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            i,
                            k as u32,
                            range_min,
                            range_max,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                            true,
                        )
                        .is_err()
                        {
                            return None;
                        }
                    }
                    k += 1;
                    continue;
                } else {
                    if !just_rewinded {
                        let m0 = tokens[k].mask[0] as usize;
                        let prev = q_group_stack[m0];

                        range_max = tokens[k].count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[m0] != 0 {
                            0
                        } else {
                            tokens[k].count_lo as u64
                        };

                        if (q_group_state[m0] as u64 + 1) < range_min {
                            q_group_state[m0] += 1;
                            if save_state(
                                &mut rewind_stack,
                                &mut stack_n,
                                i,
                                k as u32,
                                range_min,
                                range_max,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                                false,
                            )
                            .is_err()
                            {
                                return None;
                            }
                            k = (k as isize + tokens[k].pair_offset as isize) as usize;
                            // we want to actually hit the group node next
                            // (no k -= 1 needed since we don't k+=1 here)
                            continue;
                        } else if tokens[k].count_hi != 0
                            && (q_group_state[m0] as u64 + 1) > range_max
                        {
                            range_max = range_max.wrapping_sub(1);
                            if stack_n <= 1 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            if stack_n < 1 {
                                return None;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k as usize;
                            if tokens[k].kind == REMIMU_KIND_CLOSE {
                                let m0b = tokens[k].mask[0] as usize;
                                q_group_state[m0b] = rewind_stack[stack_n].group_state;
                                q_group_stack[m0b] = rewind_stack[stack_n].prev;
                            }
                            continue;
                        }

                        // detect zero-length matches
                        let mut force_zero = false;
                        if prev != 0 && (rewind_stack[prev as usize].i as u32) > (i as u32) {
                            let pair_idx =
                                (k as isize + tokens[k].pair_offset as isize) as usize;
                            let mut nfind = stack_n - 1;
                            while nfind > 0 && rewind_stack[nfind].k != pair_idx as u32 {
                                nfind -= 1;
                            }
                            if nfind > 0 && rewind_stack[nfind].i == i {
                                force_zero = true;
                            }
                        }

                        if force_zero
                            || (prev != 0
                                && (rewind_stack[prev as usize].i as u32) == (i as u32))
                        {
                            q_group_accepts_zero[m0] = 1;
                            if stack_n <= 1 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            if stack_n < 1 {
                                return None;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k as usize;
                            if tokens[k].kind == REMIMU_KIND_CLOSE {
                                let m0b = tokens[k].mask[0] as usize;
                                q_group_state[m0b] = rewind_stack[stack_n].group_state;
                                q_group_stack[m0b] = rewind_stack[stack_n].prev;
                            }
                            continue;
                        } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            q_group_state[m0] += 1;
                            if save_state(
                                &mut rewind_stack,
                                &mut stack_n,
                                i,
                                k as u32,
                                range_min,
                                range_max,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                                false,
                            )
                            .is_err()
                            {
                                return None;
                            }
                            q_group_state[m0] = 0;
                            k += 1;
                            continue;
                        } else {
                            // greedy
                            if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let mut k2 = k;
                                if q_group_state[m0] == 0 {
                                    k2 = (k as isize + tokens[k].pair_offset as isize) as usize;
                                }
                                if stack_n <= 1 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 1 && rewind_stack[stack_n].k != k2 as u32 {
                                    stack_n -= 1;
                                }
                                if stack_n < 1 {
                                    return None;
                                }
                            }
                            // continue to next match if sane
                            let pair_idx =
                                (k as isize + tokens[k].pair_offset as isize) as usize;
                            let pair_m0 = tokens[pair_idx].mask[0] as usize;
                            if (q_group_state[pair_m0] as u32) < (i as u32) {
                                q_group_state[m0] += 1;
                                if save_state(
                                    &mut rewind_stack,
                                    &mut stack_n,
                                    i,
                                    k as u32,
                                    range_min,
                                    range_max,
                                    tokens,
                                    &mut q_group_state,
                                    &mut q_group_stack,
                                    false,
                                )
                                .is_err()
                                {
                                    return None;
                                }
                                k = (k as isize + tokens[k].pair_offset as isize) as usize;
                                continue;
                            }
                            // else fall through
                            k += 1;
                            continue;
                        }
                    } else {
                        just_rewinded = false;
                        let m0 = tokens[k].mask[0] as usize;

                        if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            // dummy save
                            if save_state(
                                &mut rewind_stack,
                                &mut stack_n,
                                i,
                                k as u32,
                                range_min,
                                range_max,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                                true,
                            )
                            .is_err()
                            {
                                return None;
                            }
                            q_group_stack[m0] = stack_n as u32;
                            k = (k as isize + tokens[k].pair_offset as isize) as usize;
                            continue;
                        } else {
                            // greedy
                            if (q_group_state[m0] as u64) < range_min
                                && q_group_accepts_zero[m0] == 0
                            {
                                if stack_n <= 1 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                                    stack_n -= 1;
                                }
                                if stack_n < 1 {
                                    return None;
                                }
                                just_rewinded = true;
                                range_min = rewind_stack[stack_n].range_min;
                                range_max = rewind_stack[stack_n].range_max;
                                i = rewind_stack[stack_n].i;
                                k = rewind_stack[stack_n].k as usize;
                                if tokens[k].kind == REMIMU_KIND_CLOSE {
                                    let m0b = tokens[k].mask[0] as usize;
                                    q_group_state[m0b] = rewind_stack[stack_n].group_state;
                                    q_group_stack[m0b] = rewind_stack[stack_n].prev;
                                }
                                continue;
                            } else {
                                q_group_state[m0] = 0;
                                let cap_index = q_group_cap_index[m0];
                                if cap_index != 0xFFFF {
                                    if save_state(
                                        &mut rewind_stack,
                                        &mut stack_n,
                                        i,
                                        k as u32,
                                        range_min,
                                        range_max,
                                        tokens,
                                        &mut q_group_state,
                                        &mut q_group_stack,
                                        true,
                                    )
                                    .is_err()
                                    {
                                        return None;
                                    }
                                }
                                k += 1;
                                continue;
                            }
                        }
                    }
                }
            } else if kind == REMIMU_KIND_OR {
                k = (k as isize + tokens[k].pair_offset as isize) as usize;
                continue;
            } else if kind == REMIMU_KIND_NORMAL {
                if !just_rewinded {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k].count_lo as u64
                        && text_at(i) != 0
                        && check_mask(k, text_at(i))
                    {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k].count_lo as u64 {
                        i = old_i;
                        if stack_n <= 1 {
                            return None;
                        }
                        stack_n -= 1;
                        while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                            stack_n -= 1;
                        }
                        if stack_n < 1 {
                            return None;
                        }
                        just_rewinded = true;
                        range_min = rewind_stack[stack_n].range_min;
                        range_max = rewind_stack[stack_n].range_max;
                        i = rewind_stack[stack_n].i;
                        k = rewind_stack[stack_n].k as usize;
                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            let m0b = tokens[k].mask[0] as usize;
                            q_group_state[m0b] = rewind_stack[stack_n].group_state;
                            q_group_stack[m0b] = rewind_stack[stack_n].prev;
                        }
                        continue;
                    }

                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = (tokens[k].count_hi as u64).wrapping_sub(1);
                        if save_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            i,
                            k as u32,
                            range_min,
                            range_max,
                            tokens,
                            &mut q_group_state,
                            &mut q_group_stack,
                            false,
                        )
                        .is_err()
                        {
                            return None;
                        }
                    } else {
                        let mut limit = tokens[k].count_hi as u64;
                        if limit == 0 {
                            limit = !0u64;
                        }
                        range_min = n;
                        while text_at(i) != 0 && check_mask(k, text_at(i)) && n + 1 < limit {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            if save_state(
                                &mut rewind_stack,
                                &mut stack_n,
                                i,
                                k as u32,
                                range_min,
                                range_max,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                                false,
                            )
                            .is_err()
                            {
                                return None;
                            }
                        }
                    }
                    k += 1;
                    continue;
                } else {
                    just_rewinded = false;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit = range_max;
                        if limit == 0 {
                            limit = !0u64;
                        }
                        if check_mask(k, text_at(i)) && text_at(i) != 0 && range_min < limit {
                            i += 1;
                            range_min += 1;
                            if save_state(
                                &mut rewind_stack,
                                &mut stack_n,
                                i,
                                k as u32,
                                range_min,
                                range_max,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                                false,
                            )
                            .is_err()
                            {
                                return None;
                            }
                            k += 1;
                            continue;
                        } else {
                            if stack_n <= 1 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            if stack_n < 1 {
                                return None;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k as usize;
                            if tokens[k].kind == REMIMU_KIND_CLOSE {
                                let m0b = tokens[k].mask[0] as usize;
                                q_group_state[m0b] = rewind_stack[stack_n].group_state;
                                q_group_stack[m0b] = rewind_stack[stack_n].prev;
                            }
                            continue;
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            if save_state(
                                &mut rewind_stack,
                                &mut stack_n,
                                i,
                                k as u32,
                                range_min,
                                range_max,
                                tokens,
                                &mut q_group_state,
                                &mut q_group_stack,
                                false,
                            )
                            .is_err()
                            {
                                return None;
                            }
                            k += 1;
                            continue;
                        } else {
                            if stack_n <= 1 {
                                return None;
                            }
                            stack_n -= 1;
                            while stack_n > 1 && rewind_stack[stack_n].prev == 0xFAC7 {
                                stack_n -= 1;
                            }
                            if stack_n < 1 {
                                return None;
                            }
                            just_rewinded = true;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k as usize;
                            if tokens[k].kind == REMIMU_KIND_CLOSE {
                                let m0b = tokens[k].mask[0] as usize;
                                q_group_state[m0b] = rewind_stack[stack_n].group_state;
                                q_group_stack[m0b] = rewind_stack[stack_n].prev;
                            }
                            continue;
                        }
                    }
                }
            } else {
                return None;
            }
        }
    }

    if caps != 0 {
        for n in 1..stack_n {
            let s = &rewind_stack[n];
            let kk = s.k as usize;
            if kk >= tokens.len() {
                continue;
            }
            let kind = tokens[kk].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[kk].mask[0] as usize;
                if m0 >= AUX_STATS_SIZE {
                    continue;
                }
                let cap_index = q_group_cap_index[m0];
                if cap_index == 0xFFFF {
                    continue;
                }
                let ci = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    if ci < cap_pos.len() {
                        cap_pos[ci] = s.i as i64;
                    }
                } else {
                    if ci < cap_pos.len() && cap_pos[ci] >= 0 && ci < cap_span.len() {
                        cap_span[ci] = s.i as i64 - cap_pos[ci];
                    }
                }
            }
        }
        for n in 0..caps {
            if n < cap_span.len() && cap_span[n] == -1 && n < cap_pos.len() {
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
    let mut k = 0;
    loop {
        if k >= tokens.len() {
            break;
        }
        let tk = &tokens[k];
        let kind_idx = tk.kind as usize;
        let mode_idx = tk.mode as usize;
        let kind_str = if kind_idx < kind_to_str.len() {
            kind_to_str[kind_idx]
        } else {
            "?"
        };
        let mode_str = if mode_idx < mode_to_str.len() {
            mode_to_str[mode_idx]
        } else {
            "?"
        };
        print!("{}\t{}\t", kind_str, mode_str);

        let mut c_old: i32 = -1;
        let n_chars = if tk.kind != 0 { 0 } else { 256 };
        for c in 0..n_chars {
            let cb = c as u8;
            let in_mask = (tk.mask[(cb >> 4) as usize] & (1u16 << (cb & 0xF))) != 0;
            if in_mask {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
                let cu = c as i32;
                if cu - 1 == c_old {
                    print_char_smart(c_old as u8);
                    c_old = -1;
                } else if cu - 2 == c_old {
                    print_char_smart(c_old as u8);
                    print_char_smart((c_old + 1) as u8);
                    c_old = -1;
                } else {
                    print_char_smart(c_old as u8);
                    print!("-");
                    print_char_smart((cu - 1) as u8);
                    c_old = -1;
                }
            }
        }

        println!(
            "\t{{{},{}}}\t({})",
            tk.count_lo,
            tk.count_hi.wrapping_sub(1),
            tk.pair_offset
        );

        if tk.kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}

fn print_char_smart(c: u8) {
    if c >= 0x20 && c <= 0x7E {
        print!("{}", c as char);
    } else {
        print!("\\x{:02x}", c);
    }
}
