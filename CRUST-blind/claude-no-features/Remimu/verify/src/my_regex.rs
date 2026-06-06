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
        for i in 0..16 {
            self.mask[i] = !self.mask[i];
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }

    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
    }

    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        let len = tokens.len();
        // Skip pushing if previous token has the same kind AND the current kind is BOUND/NBOUND
        let skip = len > 0
            && tokens[len - 1].kind == self.kind
            && (self.kind == REMIMU_KIND_BOUND || self.kind == REMIMU_KIND_NBOUND);

        if !skip {
            if (self.mode & REMIMU_MODE_INVERTED) != 0 {
                self.invert_mask();
            }
            if len >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            // Reset token to default
            *self = RegexToken {
                kind: REMIMU_KIND_NORMAL,
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

pub fn regex_parse(
    pattern: &str,
    tokens: &mut Vec<RegexToken>,
    token_count: &mut i16,
    flags: i32,
) -> Result<(), i32> {
    let tokens_len = *token_count as usize;
    tokens.clear();

    if tokens_len == 0 {
        return Err(-2);
    }

    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    let pat = |idx: usize, bytes: &[u8]| -> u8 {
        if idx < bytes.len() {
            bytes[idx]
        } else {
            0
        }
    };

    let mut esc_state: i32 = 0;

    const STATE_NORMAL: i32 = 1;
    const STATE_QUANT: i32 = 2;
    const STATE_MODE: i32 = 3;
    const STATE_CC_INIT: i32 = 4;
    const STATE_CC_NORMAL: i32 = 5;
    const STATE_CC_RANGE: i32 = 6;

    let mut state: i32 = STATE_NORMAL;
    let mut char_class_mem: i32 = -1;

    // initial token: invisible OPEN
    let mut token = RegexToken {
        kind: REMIMU_KIND_OPEN,
        mode: 0,
        count_lo: 0,
        count_hi: 0,
        mask: [0; 16],
        pair_offset: 0,
    };

    let mut paren_count: i32 = 0;

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pattern_bytes[i];

        'iter: {
            if state == STATE_QUANT {
                state = STATE_MODE;
                if c == b'?' {
                    token.count_lo = 0;
                    token.count_hi = 2;
                    break 'iter;
                } else if c == b'+' {
                    token.count_lo = 1;
                    token.count_hi = 0;
                    break 'iter;
                } else if c == b'*' {
                    token.count_lo = 0;
                    token.count_hi = 0;
                    break 'iter;
                } else if c == b'{' {
                    let next = pat(i + 1, pattern_bytes);
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
                            if i < pattern_len && pattern_bytes[i] >= b'0' && pattern_bytes[i] <= b'9' {
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
                            // quantifier range parsed successfully
                            break 'iter;
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
                    break 'iter;
                } else if c == b'+' {
                    token.mode |= REMIMU_MODE_POSSESSIVE;
                    break 'iter;
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
                    } else if c == 0x0B {
                        // \v
                        // (the literal char 'v')
                        token.set_mask(0x0B);
                        state = STATE_QUANT;
                    } else if c == b'v' {
                        token.set_mask(0x0B);
                        state = STATE_QUANT;
                    } else if c == b'f' {
                        token.set_mask(0x0C);
                        state = STATE_QUANT;
                    } else if c == b'x' {
                        let p1 = pat(i + 1, pattern_bytes);
                        let p2 = pat(i + 2, pattern_bytes);
                        if p1 == 0 || p2 == 0 {
                            return Err(-1);
                        }
                        // NOTE: matching the C bug — uses p1 for both n0 and n1
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
                        token.set_mask((n1 << 4) | n0);
                        i += 2;
                        state = STATE_QUANT;
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
                    token.push_to_vec(tokens, tokens_len)?;
                    if c == b'\\' {
                        esc_state = 1;
                    } else if c == b'[' {
                        state = STATE_CC_INIT;
                        char_class_mem = -1;
                        token.kind = REMIMU_KIND_NORMAL;
                        if pat(i + 1, pattern_bytes) == b'^' {
                            token.mode |= REMIMU_MODE_INVERTED;
                            i += 1;
                        }
                    } else if c == b'(' {
                        paren_count += 1;
                        state = STATE_NORMAL;
                        token.kind = REMIMU_KIND_OPEN;
                        token.count_lo = 0;
                        token.count_hi = 1;
                        let p1 = pat(i + 1, pattern_bytes);
                        let p2 = pat(i + 2, pattern_bytes);
                        if p1 == b'?' && p2 == b':' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            i += 2;
                        } else if p1 == b'?' && p2 == b'>' {
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
                        if paren_count < 0 || tokens.is_empty() {
                            return Err(-1);
                        }
                        token.kind = REMIMU_KIND_CLOSE;
                        state = STATE_QUANT;

                        let mut balance: i32 = 0;
                        let mut found: i64 = -1;
                        let k = tokens.len() as i64;
                        let mut l = k - 1;
                        while l >= 0 {
                            let lk = tokens[l as usize].kind;
                            if lk == REMIMU_KIND_NCOPEN || lk == REMIMU_KIND_OPEN {
                                if balance == 0 {
                                    found = l;
                                    break;
                                } else {
                                    balance -= 1;
                                }
                            } else if lk == REMIMU_KIND_CLOSE {
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
                        // phantom group for atomic group emulation
                        if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                            token.push_to_vec(tokens, tokens_len)?;
                            token.kind = REMIMU_KIND_CLOSE;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.pair_offset = -(diff as i16) - 2;
                            tokens[(found - 1) as usize].pair_offset = (diff as i16) + 2;
                        }
                    } else if c == b'?' || c == b'+' || c == b'*' || c == b'{' {
                        return Err(-1);
                    } else if c == b'.' {
                        for n in 0..16 {
                            token.mask[n] = 0xFFFF;
                        }
                        if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
                            token.mask[1] ^= 0x04; // \n
                            token.mask[1] ^= 0x20; // \r
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
                let mut c_local = c;
                if c == b'\\' && esc_state == 0 {
                    esc_state = 1;
                    break 'iter;
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
                        esc_c = 0x0B;
                    } else if c == b'f' {
                        esc_c = 0x0C;
                    } else if c == b'x' {
                        let p1 = pat(i + 1, pattern_bytes);
                        let p2 = pat(i + 2, pattern_bytes);
                        if p1 == 0 || p2 == 0 {
                            return Err(-1);
                        }
                        // NOTE: matching the C bug — uses p1 for both n0 and n1
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
                        break 'iter;
                    } else {
                        return Err(-1);
                    }
                    // Replace c with esc_c so subsequent state-handling treats this as a literal
                    c_local = esc_c;
                }

                if state == STATE_CC_INIT {
                    char_class_mem = c_local as i32;
                    token.set_mask(c_local);
                    state = STATE_CC_NORMAL;
                } else if state == STATE_CC_NORMAL {
                    if c == b']' && esc_c == 0 {
                        char_class_mem = -1;
                        state = STATE_QUANT;
                        break 'iter;
                    } else if c == b'-' && esc_c == 0 && char_class_mem >= 0 {
                        state = STATE_CC_RANGE;
                        break 'iter;
                    } else {
                        char_class_mem = c_local as i32;
                        token.set_mask(c_local);
                        state = STATE_CC_NORMAL;
                    }
                } else if state == STATE_CC_RANGE {
                    if c == b']' && esc_c == 0 {
                        char_class_mem = -1;
                        token.set_mask(b'-');
                        state = STATE_QUANT;
                        break 'iter;
                    } else {
                        if char_class_mem == -1 {
                            return Err(-1);
                        }
                        if (c_local as i32) < char_class_mem {
                            return Err(-1);
                        }
                        // Set bits for [char_class_mem .. c_local]
                        let lo = char_class_mem as u8;
                        let hi = c_local;
                        // C: for (uint8_t i = c; i > char_class_mem; i--) set_mask(i); — note this loop sets [mem+1 .. c]
                        // and the initial set_mask(char_class_mem) was already done when we were in STATE_CC_NORMAL.
                        // Reproduce: set everything > char_class_mem and <= c
                        let mut idx = hi;
                        while idx > lo {
                            token.set_mask(idx);
                            if idx == 0 {
                                break;
                            }
                            idx = idx.wrapping_sub(1);
                        }
                        state = STATE_CC_NORMAL;
                        char_class_mem = -1;
                    }
                }
            } else {
                // unreachable in correct execution
                return Err(-1);
            }
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

    token.push_to_vec(tokens, tokens_len)?;

    // add invisible non-capturing group close
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, tokens_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, tokens_len)?;

    let k = tokens.len() as i16;
    tokens[0].pair_offset = k - 2;
    tokens[(k - 2) as usize].pair_offset = -(k - 2);

    *token_count = k;

    // copy quantifiers from )s to (s and assign group indices
    let mut n: u64 = 0;
    let mut k2: usize = 0;
    while k2 < tokens.len() {
        let tk_kind = tokens[k2].kind;
        if tk_kind == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16;
            n += 1;

            let pair_off = tokens[k2].pair_offset;
            let k3 = (k2 as i64) + (pair_off as i64);
            let k3u = k3 as usize;
            tokens[k3u].count_lo = tokens[k2].count_lo;
            tokens[k3u].count_hi = tokens[k2].count_hi;
            tokens[k3u].mask[0] = n as u16;
            tokens[k3u].mode = tokens[k2].mode;
            n += 1;

            if n > 1024 {
                return Err(-1);
            }
        } else if tk_kind == REMIMU_KIND_OR
            || tk_kind == REMIMU_KIND_OPEN
            || tk_kind == REMIMU_KIND_NCOPEN
        {
            // find next | or ) at the same nesting depth and how far away it is
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l: usize = k2 + 1;
            while l < tokens_len && l < tokens.len() {
                let lk = tokens[l].kind;
                if lk == REMIMU_KIND_OR && balance == 0 {
                    found = l as i64;
                    break;
                } else if lk == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = l as i64;
                        break;
                    } else {
                        balance -= 1;
                    }
                } else if lk == REMIMU_KIND_NCOPEN || lk == REMIMU_KIND_OPEN {
                    balance += 1;
                }
                l += 1;
            }
            if found == -1 {
                return Err(-1);
            }
            let diff = found - (k2 as i64);
            if diff > 32767 {
                return Err(-1);
            }

            if tk_kind == REMIMU_KIND_OR {
                tokens[k2].pair_offset = diff as i16;
            } else {
                tokens[k2].mask[15] = diff as u16;
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

#[derive(Clone, Copy)]
struct InternalMatcherState {
    k: u32,
    group_state: u32,
    prev: u32,
    i: u64,
    range_min: u64,
    range_max: u64,
}

fn check_is_w(byte: u8) -> bool {
    let w_mask: [u16; 16] = {
        let mut m = [0u16; 16];
        m[3] = 0x03FF;
        m[4] = 0xFFFE;
        m[5] = 0x87FF;
        m[6] = 0xFFFE;
        m[7] = 0x07FF;
        m
    };
    (w_mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
}

pub fn regex_match(
    tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Option<usize> {
    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;

    let text_bytes = text.as_bytes();
    let text_byte = |i: u64, tb: &[u8]| -> u8 {
        let idx = i as usize;
        if idx < tb.len() {
            tb[idx]
        } else {
            0
        }
    };

    let mut cap_slots = cap_slots as usize;
    if cap_slots > AUX_STATS_SIZE {
        cap_slots = AUX_STATS_SIZE;
    }

    // quantified group state
    let mut q_group_accepts_zero = vec![0u8; AUX_STATS_SIZE];
    let mut q_group_state = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_stack = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_cap_index = vec![0xFFFFu16; AUX_STATS_SIZE];

    let mut k: usize = 0;
    let mut caps: usize = 0;

    while tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let m0 = tokens[k].mask[0] as usize;
            let pair_idx = (k as i64 + tokens[k].pair_offset as i64) as usize;
            let m_pair = tokens[pair_idx].mask[0] as usize;
            q_group_cap_index[m0] = caps as u16;
            q_group_cap_index[m_pair] = caps as u16;
            cap_pos[caps] = -1;
            cap_span[caps] = -1;
            caps += 1;
        }
        k += 1;
        let tk_kind = tokens[k].kind;
        if tk_kind == REMIMU_KIND_CLOSE
            || tk_kind == REMIMU_KIND_OPEN
            || tk_kind == REMIMU_KIND_NCOPEN
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

    let tokens_len: usize = k;

    let mut rewind_stack: Vec<InternalMatcherState> =
        vec![
            InternalMatcherState {
                k: 0,
                group_state: 0,
                prev: 0,
                i: 0,
                range_min: 0,
                range_max: 0,
            };
            STACK_SIZE_MAX
        ];
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;

    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: u8 = 0;

    // helper macros translated to closures/inline code
    // _REWIND_DO_SAVE, _REWIND_DO_SAVE_DUMMY, _REWIND_OR_ABORT

    k = 0;
    let mut should_rewind: bool;
    let mut should_abort: bool;

    while k < tokens_len {
        // (verbose printing skipped)
        let tk_kind = tokens[k].kind;

        let mut continue_loop = false;
        should_rewind = false;
        should_abort = false;

        if tk_kind == REMIMU_KIND_CARET {
            if i != 0 {
                should_rewind = true;
            } else {
                continue_loop = true;
            }
        } else if tk_kind == REMIMU_KIND_DOLLAR {
            if (i as usize) < text_bytes.len() {
                should_rewind = true;
            } else {
                continue_loop = true;
            }
        } else if tk_kind == REMIMU_KIND_BOUND {
            let cur = text_byte(i, text_bytes);
            let prev_b = if i == 0 { 0 } else { text_byte(i - 1, text_bytes) };
            if i == 0 && !check_is_w(cur) {
                should_rewind = true;
            } else if i != 0 && cur == 0 && !check_is_w(prev_b) {
                should_rewind = true;
            } else if i != 0 && cur != 0 && check_is_w(prev_b) == check_is_w(cur) {
                should_rewind = true;
            }
        } else if tk_kind == REMIMU_KIND_NBOUND {
            let cur = text_byte(i, text_bytes);
            let prev_b = if i == 0 { 0 } else { text_byte(i - 1, text_bytes) };
            if i == 0 && check_is_w(cur) {
                should_rewind = true;
            } else if i != 0 && cur == 0 && check_is_w(prev_b) {
                should_rewind = true;
            } else if i != 0 && cur != 0 && check_is_w(prev_b) != check_is_w(cur) {
                should_rewind = true;
            }
        } else {
            // deliberately unmatchable token (e.g. a{0}, a{0,0})
            if tokens[k].count_hi == 1 {
                if tk_kind == REMIMU_KIND_OPEN || tk_kind == REMIMU_KIND_NCOPEN {
                    let new_k = k as i64 + tokens[k].pair_offset as i64;
                    k = new_k as usize;
                } else {
                    k += 1;
                }
                continue_loop = true;
            } else if tk_kind == REMIMU_KIND_OPEN || tk_kind == REMIMU_KIND_NCOPEN {
                if just_rewinded == 0 {
                    let m_pair = tokens
                        [(k as i64 + tokens[k].pair_offset as i64) as usize]
                        .mask[0] as usize;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k].count_lo == 0 || q_group_accepts_zero[m_pair] != 0)
                    {
                        range_min = 0;
                        range_max = 0;
                        // _REWIND_DO_SAVE(k)
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = InternalMatcherState {
                            k: k as u32,
                            group_state: 0,
                            prev: 0,
                            i,
                            range_min,
                            range_max,
                        };
                        if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[s.k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;

                        let new_k = k as i64 + tokens[k].pair_offset as i64;
                        k = new_k as usize;
                    } else {
                        range_min = 1;
                        range_max = 0;
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = InternalMatcherState {
                            k: k as u32,
                            group_state: 0,
                            prev: 0,
                            i,
                            range_min,
                            range_max,
                        };
                        if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[s.k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                } else {
                    just_rewinded = 0;
                    let orig_k = k;

                    if range_min != 0 {
                        k += range_min as usize;
                        if tokens[k - 1].kind == REMIMU_KIND_OR {
                            k = (k as i64 + tokens[k - 1].pair_offset as i64 - 1) as usize;
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
                                continue_loop = true;
                            } else {
                                should_rewind = true;
                            }
                        } else {
                            // assert(tokens[k].kind == REMIMU_KIND_OR);
                        }
                    }

                    if !continue_loop && !should_rewind {
                        let k_diff = k as i64 - orig_k as i64;
                        range_min = (k_diff + 1) as u64;

                        let save_k = (k as i64 - k_diff) as usize;
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = InternalMatcherState {
                            k: save_k as u32,
                            group_state: 0,
                            prev: 0,
                            i,
                            range_min,
                            range_max,
                        };
                        if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[s.k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                }
            } else if tk_kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[k].mask[0] as usize;
                if tokens[k].count_lo == 1 && tokens[k].count_hi == 2 {
                    // unquantified
                    let cap_index = q_group_cap_index[m0];
                    if cap_index != 0xFFFF {
                        // _REWIND_DO_SAVE_DUMMY
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let s = InternalMatcherState {
                            k: k as u32,
                            group_state: 0,
                            prev: 0xFAC7,
                            i,
                            range_min,
                            range_max,
                        };
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                } else {
                    // quantified
                    if just_rewinded == 0 {
                        let prev_idx = q_group_stack[m0];

                        range_max = tokens[k].count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[m0] != 0 {
                            0
                        } else {
                            tokens[k].count_lo as u64
                        };

                        // minimum requirement not yet met
                        if (q_group_state[m0] as u64 + 1) < range_min {
                            q_group_state[m0] += 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = InternalMatcherState {
                                k: k as u32,
                                group_state: 0,
                                prev: 0,
                                i,
                                range_min,
                                range_max,
                            };
                            if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                                let m = tokens[s.k as usize].mask[0] as usize;
                                s.group_state = q_group_state[m];
                                s.prev = q_group_stack[m];
                                q_group_stack[m] = stack_n as u32;
                            }
                            rewind_stack[stack_n] = s;
                            stack_n += 1;

                            let new_k =
                                k as i64 + tokens[k].pair_offset as i64 - 1;
                            k = new_k as usize;
                            continue_loop = true;
                        } else if tokens[k].count_hi != 0
                            && (q_group_state[m0] as u64 + 1) > range_max
                        {
                            range_max = range_max.wrapping_sub(1);
                            should_rewind = true;
                            // continue_loop = true after rewind
                        } else {
                            // fallback case to detect zero-length matches
                            let mut force_zero: u8 = 0;
                            if prev_idx != 0
                                && (rewind_stack[prev_idx as usize].i as u32) > (i as u32)
                            {
                                let mut n_idx = stack_n.wrapping_sub(1);
                                let target = (k as i64 + tokens[k].pair_offset as i64) as u32;
                                while n_idx > 0 && rewind_stack[n_idx].k != target {
                                    n_idx = n_idx.wrapping_sub(1);
                                }
                                // assert(n_idx > 0)
                                if rewind_stack[n_idx].i == i {
                                    force_zero = 1;
                                }
                            }

                            if force_zero != 0
                                || (prev_idx != 0
                                    && (rewind_stack[prev_idx as usize].i as u32) == (i as u32))
                            {
                                q_group_accepts_zero[m0] = 1;
                                should_rewind = true;
                            } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                                q_group_state[m0] += 1;
                                if stack_n >= STACK_SIZE_MAX {
                                    return None;
                                }
                                let mut s = InternalMatcherState {
                                    k: k as u32,
                                    group_state: 0,
                                    prev: 0,
                                    i,
                                    range_min,
                                    range_max,
                                };
                                if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                                    let m = tokens[s.k as usize].mask[0] as usize;
                                    s.group_state = q_group_state[m];
                                    s.prev = q_group_stack[m];
                                    q_group_stack[m] = stack_n as u32;
                                }
                                rewind_stack[stack_n] = s;
                                stack_n += 1;
                                q_group_state[m0] = 0;
                            } else {
                                // greedy
                                if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                    let mut k2 = k as u32;
                                    if q_group_state[m0] == 0 {
                                        k2 = (k as i64 + tokens[k].pair_offset as i64) as u32;
                                    }
                                    if stack_n == 0 {
                                        return None;
                                    }
                                    stack_n -= 1;
                                    while stack_n > 0 && rewind_stack[stack_n].k != k2 {
                                        stack_n -= 1;
                                    }
                                    if stack_n == 0 {
                                        return None;
                                    }
                                }
                                let m_pair = tokens
                                    [(k as i64 + tokens[k].pair_offset as i64) as usize]
                                    .mask[0] as usize;
                                if (q_group_state[m_pair] as u32) < (i as u32) {
                                    q_group_state[m0] += 1;
                                    if stack_n >= STACK_SIZE_MAX {
                                        return None;
                                    }
                                    let mut s = InternalMatcherState {
                                        k: k as u32,
                                        group_state: 0,
                                        prev: 0,
                                        i,
                                        range_min,
                                        range_max,
                                    };
                                    if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                                        let m = tokens[s.k as usize].mask[0] as usize;
                                        s.group_state = q_group_state[m];
                                        s.prev = q_group_stack[m];
                                        q_group_stack[m] = stack_n as u32;
                                    }
                                    rewind_stack[stack_n] = s;
                                    stack_n += 1;
                                    let new_k =
                                        k as i64 + tokens[k].pair_offset as i64 - 1;
                                    k = new_k as usize;
                                }
                            }
                        }
                    } else {
                        just_rewinded = 0;

                        if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            // dummy save
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let s = InternalMatcherState {
                                k: k as u32,
                                group_state: 0,
                                prev: 0xFAC7,
                                i,
                                range_min,
                                range_max,
                            };
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                            q_group_stack[m0] = stack_n as u32;
                            let new_k =
                                k as i64 + tokens[k].pair_offset as i64 - 1;
                            k = new_k as usize;
                        } else {
                            if (q_group_state[m0] as u64) < range_min
                                && q_group_accepts_zero[m0] == 0
                            {
                                should_rewind = true;
                            } else {
                                q_group_state[m0] = 0;
                                let cap_index = q_group_cap_index[m0];
                                if cap_index != 0xFFFF {
                                    if stack_n >= STACK_SIZE_MAX {
                                        return None;
                                    }
                                    let s = InternalMatcherState {
                                        k: k as u32,
                                        group_state: 0,
                                        prev: 0xFAC7,
                                        i,
                                        range_min,
                                        range_max,
                                    };
                                    rewind_stack[stack_n] = s;
                                    stack_n += 1;
                                }
                            }
                        }
                    }
                }
            } else if tk_kind == REMIMU_KIND_OR {
                let new_k = k as i64 + tokens[k].pair_offset as i64 - 1;
                k = new_k as usize;
            } else if tk_kind == REMIMU_KIND_NORMAL {
                if just_rewinded == 0 {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k].count_lo as u64
                        && (i as usize) < text_bytes.len()
                        && tokens[k].check_mask(text_byte(i, text_bytes))
                    {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k].count_lo as u64 {
                        i = old_i;
                        should_rewind = true;
                    } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = (tokens[k].count_hi as u64).wrapping_sub(1);
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = InternalMatcherState {
                            k: k as u32,
                            group_state: 0,
                            prev: 0,
                            i,
                            range_min,
                            range_max,
                        };
                        if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[s.k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    } else {
                        let mut limit = tokens[k].count_hi as u64;
                        if limit == 0 {
                            limit = !limit;
                        }
                        range_min = n;
                        while (i as usize) < text_bytes.len()
                            && tokens[k].check_mask(text_byte(i, text_bytes))
                            && n + 1 < limit
                        {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = InternalMatcherState {
                                k: k as u32,
                                group_state: 0,
                                prev: 0,
                                i,
                                range_min,
                                range_max,
                            };
                            if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                                let m = tokens[s.k as usize].mask[0] as usize;
                                s.group_state = q_group_state[m];
                                s.prev = q_group_stack[m];
                                q_group_stack[m] = stack_n as u32;
                            }
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        }
                    }
                } else {
                    just_rewinded = 0;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit = range_max;
                        if limit == 0 {
                            limit = !limit;
                        }
                        let cur = text_byte(i, text_bytes);
                        if (i as usize) < text_bytes.len()
                            && tokens[k].check_mask(cur)
                            && cur != 0
                            && range_min < limit
                        {
                            i += 1;
                            range_min += 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = InternalMatcherState {
                                k: k as u32,
                                group_state: 0,
                                prev: 0,
                                i,
                                range_min,
                                range_max,
                            };
                            if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                                let m = tokens[s.k as usize].mask[0] as usize;
                                s.group_state = q_group_state[m];
                                s.prev = q_group_stack[m];
                                q_group_stack[m] = stack_n as u32;
                            }
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        } else {
                            should_rewind = true;
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = InternalMatcherState {
                                k: k as u32,
                                group_state: 0,
                                prev: 0,
                                i,
                                range_min,
                                range_max,
                            };
                            if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                                let m = tokens[s.k as usize].mask[0] as usize;
                                s.group_state = q_group_state[m];
                                s.prev = q_group_stack[m];
                                q_group_stack[m] = stack_n as u32;
                            }
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        } else {
                            should_rewind = true;
                        }
                    }
                }
            } else {
                return None;
            }
        }

        if should_rewind {
            // _REWIND_OR_ABORT
            if stack_n == 0 {
                return None; // -1 no match
            }
            stack_n -= 1;
            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 {
                stack_n -= 1;
            }
            // Edge: if stack_n == 0 and the entry is dummy, we still take it (matches C behavior:
            // C decrements then loops while >0; if we reach 0 and it's dummy, we still use that entry)
            just_rewinded = 1;
            range_min = rewind_stack[stack_n].range_min;
            range_max = rewind_stack[stack_n].range_max;
            i = rewind_stack[stack_n].i;
            k = rewind_stack[stack_n].k as usize;
            if tokens[k].kind == REMIMU_KIND_CLOSE {
                let m = tokens[k].mask[0] as usize;
                q_group_state[m] = rewind_stack[stack_n].group_state;
                q_group_stack[m] = rewind_stack[stack_n].prev;
            }
            // C does k -= 1 to compensate for the for-loop increment.
            // We use a `while` loop with manual increment, so simulate by decrementing then incrementing.
            // i.e., we'll do k = k.wrapping_sub(1) then continue (and the bottom does k+=1)
            if k == 0 {
                // rewind to the very first token; the for loop re-executes k=0 — match by setting k = usize::MAX so k+=1 wraps to 0
                k = usize::MAX;
            } else {
                k -= 1;
            }
            should_abort = false;
        }
        let _ = should_abort;

        if continue_loop {
            // do nothing extra — fall through to k += 1
        }

        k = k.wrapping_add(1);
    }

    if caps != 0 {
        for n in 0..stack_n {
            let s = rewind_stack[n];
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[s.k as usize].mask[0] as usize];
                if cap_index == 0xFFFF {
                    continue;
                }
                let ci = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[ci] = s.i as i64;
                } else if cap_pos[ci] >= 0 {
                    cap_span[ci] = (s.i as i64) - cap_pos[ci];
                }
            }
        }
        for n in 0..caps {
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
        let tk = &tokens[k];
        let kind_idx = tk.kind as usize;
        let mode_idx = tk.mode as usize;
        let kind_str = kind_to_str.get(kind_idx).copied().unwrap_or("UNKNOWN");
        let mode_str = mode_to_str.get(mode_idx).copied().unwrap_or("UNKNOWN");
        print!("{}\t{}\t", kind_str, mode_str);

        let mut c_old: i32 = -1;
        let limit = if tk.kind != 0 { 0 } else { 256 };
        for c in 0..limit {
            let bit_set = tk.check_mask(c as u8);
            if bit_set {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
                let print_c_smart = |x: i32| {
                    if x >= 0x20 && x <= 0x7E {
                        print!("{}", x as u8 as char);
                    } else {
                        print!("\\x{:02x}", x);
                    }
                };
                if (c as i32) - 1 == c_old {
                    print_c_smart(c_old);
                    c_old = -1;
                } else if (c as i32) - 2 == c_old {
                    print_c_smart(c_old);
                    print_c_smart(c_old + 1);
                    c_old = -1;
                } else {
                    print_c_smart(c_old);
                    print!("-");
                    print_c_smart((c as i32) - 1);
                    c_old = -1;
                }
            }
        }

        let count_hi_disp = (tk.count_hi as i32) - 1;
        println!("\t{{{},{}}}\t({})", tk.count_lo, count_hi_disp, tk.pair_offset);

        if tk.kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}
