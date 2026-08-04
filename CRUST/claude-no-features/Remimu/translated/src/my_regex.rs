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
        let same_bound = !tokens.is_empty()
            && tokens.last().unwrap().kind == self.kind
            && (self.kind == REMIMU_KIND_BOUND || self.kind == REMIMU_KIND_NBOUND);
        if !same_bound {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
            }
            if tokens.len() >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
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

#[derive(Clone, Copy)]
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

fn pat_at(bytes: &[u8], i: usize) -> u8 {
    if i < bytes.len() { bytes[i] } else { 0 }
}

// Helper: applies a class shorthand (\d \s \w \D \S \W) to the current token's mask.
fn apply_shorthand(token: &mut RegexToken, c: u8) {
    let is_upper = c <= b'Z';
    let mut m = [0u16; 16];
    let lc = if is_upper { c + 0x20 } else { c };
    if lc == b'd' || lc == b'w' {
        m[3] |= 0x03FF; // 0~9
    }
    if lc == b's' {
        m[0] |= 0x3E00; // \t-\r
        m[2] |= 1; // ' '
    }
    if lc == b'w' {
        m[4] |= 0xFFFE; // A-O
        m[5] |= 0x87FF; // P-Z_
        m[6] |= 0xFFFE; // a-o
        m[7] |= 0x07FF; // p-z
    }
    for n in 0..16 {
        if is_upper {
            token.mask[n] |= !m[n];
        } else {
            token.mask[n] |= m[n];
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
    if tokens_len == 0 {
        return Err(-2);
    }
    tokens.clear();

    let pat = pattern.as_bytes();
    let pattern_len = pat.len();

    // Escape state: 0 normal, 1 just saw backslash
    let mut esc_state: i32 = 0;

    const STATE_NORMAL: i32 = 1;
    const STATE_QUANT: i32 = 2;
    const STATE_MODE: i32 = 3;
    const STATE_CC_INIT: i32 = 4;
    const STATE_CC_NORMAL: i32 = 5;
    const STATE_CC_RANGE: i32 = 6;
    let mut state: i32 = STATE_NORMAL;

    let mut char_class_mem: i32 = -1;

    // initial token (invisible group)
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
        let c = pat[i];

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
                let n1 = pat_at(pat, i + 1);
                if n1 == 0 || n1 < b'0' || n1 > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while pat_at(pat, i) >= b'0' && pat_at(pat, i) <= b'9' {
                        val *= 10;
                        val += (pat[i] - b'0') as u32;
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
                                val2 *= 10;
                                val2 += (pat[i] - b'0') as u32;
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
                        // success - skip past }
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
                    let mut n0 = pat[i + 1];
                    // Note: original C has bug - both n0 and n1 use i+1
                    let mut n1 = pat[i + 1];
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
                    apply_shorthand(&mut token, c);
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
                    if paren_count < 0 || tokens.is_empty() {
                        return Err(-1);
                    }
                    token.kind = REMIMU_KIND_CLOSE;
                    state = STATE_QUANT;

                    let k_now = tokens.len() as isize;
                    let mut balance: i32 = 0;
                    let mut found: isize = -1;
                    let mut l: isize = k_now - 1;
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
                    let diff = k_now - found;
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
                        // Note: tokens[found - 1] is the phantom NCOPEN
                        tokens[(found - 1) as usize].pair_offset = (diff as i16) + 2;
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
                    let mut n0 = pat[i + 1];
                    let mut n1 = pat[i + 1]; // bug-for-bug
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
                    apply_shorthand(&mut token, c);
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
                    let actual = if esc_c != 0 { esc_c } else { c };
                    char_class_mem = actual as i32;
                    token.set_mask(actual);
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
                    let actual = if esc_c != 0 { esc_c } else { c };
                    if (actual as i32) < char_class_mem {
                        return Err(-1);
                    }
                    let mut x: u8 = actual;
                    while x as i32 > char_class_mem {
                        token.set_mask(x);
                        if x == 0 {
                            break;
                        }
                        x -= 1;
                    }
                    state = STATE_CC_NORMAL;
                    char_class_mem = -1;
                }
            }
            i += 1;
            continue;
        } else {
            // unreachable
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

    // add invisible non-capturing group close specifier
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, tokens_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, tokens_len)?;

    let k = tokens.len();
    if k < 3 {
        return Err(-1);
    }
    tokens[0].pair_offset = (k as i16) - 2;
    tokens[k - 2].pair_offset = -((k as i16) - 2);

    *token_count = k as i16;

    // copy quantifiers from )s to (s and assign group indices
    let mut n: u64 = 0;
    let total_k = k as i16;
    let mut k2: i16 = 0;
    while k2 < total_k {
        let kind = tokens[k2 as usize].kind;
        if kind == REMIMU_KIND_CLOSE {
            tokens[k2 as usize].mask[0] = n as u16;
            n += 1;

            let k3 = k2 + tokens[k2 as usize].pair_offset;
            tokens[k3 as usize].count_lo = tokens[k2 as usize].count_lo;
            tokens[k3 as usize].count_hi = tokens[k2 as usize].count_hi;
            tokens[k3 as usize].mask[0] = n as u16;
            tokens[k3 as usize].mode = tokens[k2 as usize].mode;
            n += 1;

            if n > 1024 {
                return Err(-1);
            }
        } else if kind == REMIMU_KIND_OR
            || kind == REMIMU_KIND_OPEN
            || kind == REMIMU_KIND_NCOPEN
        {
            // find next | or ) and how far away it is
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l: i64 = (k2 + 1) as i64;
            let len = tokens.len() as i64;
            while l < len {
                let lk = tokens[l as usize].kind;
                if lk == REMIMU_KIND_OR && balance == 0 {
                    found = l;
                    break;
                } else if lk == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = l;
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
    let text_bytes = text.as_bytes();
    let text_at = |idx: usize| -> u8 {
        if idx < text_bytes.len() {
            text_bytes[idx]
        } else {
            0
        }
    };

    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;
    let mut cap_slots: usize = cap_slots as usize;
    if cap_slots > AUX_STATS_SIZE {
        cap_slots = AUX_STATS_SIZE;
    }

    let mut q_group_accepts_zero = vec![0u8; AUX_STATS_SIZE];
    let mut q_group_state = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_stack = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_cap_index = vec![0xFFFFu16; AUX_STATS_SIZE];

    let mut k: u32 = 0;
    let mut caps: u16 = 0;

    while (k as usize) < tokens.len() && tokens[k as usize].kind != REMIMU_KIND_END {
        if tokens[k as usize].kind == REMIMU_KIND_OPEN && (caps as usize) < cap_slots {
            let idx0 = tokens[k as usize].mask[0] as usize;
            let pair_idx = ((k as i32) + (tokens[k as usize].pair_offset as i32)) as usize;
            let idx1 = tokens[pair_idx].mask[0] as usize;
            if idx0 < AUX_STATS_SIZE {
                q_group_cap_index[idx0] = caps;
            }
            if idx1 < AUX_STATS_SIZE {
                q_group_cap_index[idx1] = caps;
            }
            cap_pos[caps as usize] = -1;
            cap_span[caps as usize] = -1;
            caps += 1;
        }
        k += 1;
        if (k as usize) < tokens.len() {
            let kind = tokens[k as usize].kind;
            if kind == REMIMU_KIND_CLOSE
                || kind == REMIMU_KIND_OPEN
                || kind == REMIMU_KIND_NCOPEN
            {
                let idx = tokens[k as usize].mask[0] as usize;
                if idx >= AUX_STATS_SIZE {
                    return None;
                }
                q_group_state[idx] = 0;
                q_group_stack[idx] = 0;
                q_group_accepts_zero[idx] = 0;
            }
        }
    }

    let tokens_len = k as u64;

    let mut rewind_stack: Vec<RegexMatcherState> =
        vec![RegexMatcherState::new(0, 0); STACK_SIZE_MAX];
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    k = 0;
    'main_loop: loop {
        if (k as u64) >= tokens_len {
            break;
        }

        let cur_kind = tokens[k as usize].kind;
        if cur_kind == REMIMU_KIND_CARET {
            if i != 0 {
                // _REWIND_OR_ABORT
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
                k = k.wrapping_sub(1);
                k = k.wrapping_add(1);
                continue 'main_loop;
            }
            k = k.wrapping_add(1);
            continue 'main_loop;
        } else if cur_kind == REMIMU_KIND_DOLLAR {
            if text_at(i as usize) != 0 {
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
                continue 'main_loop;
            }
            k = k.wrapping_add(1);
            continue 'main_loop;
        } else if cur_kind == REMIMU_KIND_BOUND {
            let idx = i as usize;
            let ti = text_at(idx);
            let need_rewind: bool;
            if i == 0 && !check_is_w(ti) {
                need_rewind = true;
            } else if i != 0 && ti == 0 && !check_is_w(text_at(idx - 1)) {
                need_rewind = true;
            } else if i != 0 && ti != 0 && check_is_w(text_at(idx - 1)) == check_is_w(ti) {
                need_rewind = true;
            } else {
                need_rewind = false;
            }
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
                    let idx2 = tokens[k as usize].mask[0] as usize;
                    q_group_state[idx2] = rewind_stack[stack_n].group_state;
                    q_group_stack[idx2] = rewind_stack[stack_n].prev;
                }
                continue 'main_loop;
            }
            k = k.wrapping_add(1);
            continue 'main_loop;
        } else if cur_kind == REMIMU_KIND_NBOUND {
            let idx = i as usize;
            let ti = text_at(idx);
            let need_rewind: bool;
            if i == 0 && check_is_w(ti) {
                need_rewind = true;
            } else if i != 0 && ti == 0 && check_is_w(text_at(idx - 1)) {
                need_rewind = true;
            } else if i != 0 && ti != 0 && check_is_w(text_at(idx - 1)) != check_is_w(ti) {
                need_rewind = true;
            } else {
                need_rewind = false;
            }
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
                    let idx2 = tokens[k as usize].mask[0] as usize;
                    q_group_state[idx2] = rewind_stack[stack_n].group_state;
                    q_group_stack[idx2] = rewind_stack[stack_n].prev;
                }
                continue 'main_loop;
            }
            k = k.wrapping_add(1);
            continue 'main_loop;
        } else {
            // deliberately unmatchable token
            if tokens[k as usize].count_hi == 1 {
                if cur_kind == REMIMU_KIND_OPEN || cur_kind == REMIMU_KIND_NCOPEN {
                    let off = tokens[k as usize].pair_offset as i32;
                    k = ((k as i32) + off) as u32;
                } else {
                    k = k.wrapping_add(1);
                }
                k = k.wrapping_add(1);
                continue 'main_loop;
            }

            if cur_kind == REMIMU_KIND_OPEN || cur_kind == REMIMU_KIND_NCOPEN {
                if !just_rewinded {
                    let mode = tokens[k as usize].mode;
                    let count_lo = tokens[k as usize].count_lo;
                    let pair_off = tokens[k as usize].pair_offset as i32;
                    let pair_idx = ((k as i32) + pair_off) as u32;
                    let pair_mask0 = tokens[pair_idx as usize].mask[0] as usize;
                    let lazy = mode & REMIMU_MODE_LAZY != 0;
                    if lazy && (count_lo == 0 || q_group_accepts_zero[pair_mask0] != 0) {
                        range_min = 0;
                        range_max = 0;
                        // _REWIND_DO_SAVE(k)
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                            let idx = tokens[s.k as usize].mask[0] as usize;
                            s.group_state = q_group_state[idx];
                            s.prev = q_group_stack[idx];
                            q_group_stack[idx] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                        k = ((k as i32) + pair_off) as u32;
                    } else {
                        range_min = 1;
                        range_max = 0;
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                            let idx = tokens[s.k as usize].mask[0] as usize;
                            s.group_state = q_group_state[idx];
                            s.prev = q_group_stack[idx];
                            q_group_stack[idx] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                } else {
                    just_rewinded = false;

                    let orig_k = k;

                    if range_min != 0 {
                        k = (k as u64 + range_min) as u32;

                        let prev_kind = tokens[(k as i64 - 1) as usize].kind;
                        if prev_kind == REMIMU_KIND_OR {
                            let off = tokens[(k as i64 - 1) as usize].pair_offset as i32;
                            k = ((k as i32) + off - 1) as u32;
                        } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN
                        {
                            let off = tokens[(k as i64 - 1) as usize].mask[15] as i32;
                            k = ((k as i32) + off - 1) as u32;
                        }

                        if (k as u64) >= tokens_len
                            || tokens[k as usize].kind == REMIMU_KIND_END
                        {
                            return None;
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let count_lo = tokens[k as usize].count_lo;
                            let mask0 = tokens[k as usize].mask[0] as usize;
                            if count_lo == 0 || q_group_accepts_zero[mask0] != 0 {
                                q_group_state[mask0] = 0;
                                if tokens[k as usize].mode & REMIMU_MODE_LAZY == 0 {
                                    q_group_stack[mask0] = 0;
                                }
                                k = k.wrapping_add(1);
                                continue 'main_loop;
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
                                continue 'main_loop;
                            }
                        }
                        // assert kind == OR
                    }

                    let k_diff = (k as i64) - (orig_k as i64);
                    range_min = (k_diff + 1) as u64;

                    let save_k = ((orig_k as i64)) as u32; // k - k_diff = orig_k
                    if stack_n >= STACK_SIZE_MAX {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(save_k, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.prev = 0;
                    if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                        let idx = tokens[s.k as usize].mask[0] as usize;
                        s.group_state = q_group_state[idx];
                        s.prev = q_group_stack[idx];
                        q_group_stack[idx] = stack_n as u32;
                    }
                    rewind_stack[stack_n] = s;
                    stack_n += 1;
                }
            } else if cur_kind == REMIMU_KIND_CLOSE {
                let count_lo = tokens[k as usize].count_lo;
                let count_hi = tokens[k as usize].count_hi;
                if count_lo == 1 && count_hi == 2 {
                    // unquantified - for captures
                    let mask0 = tokens[k as usize].mask[0] as usize;
                    let cap_index = q_group_cap_index[mask0];
                    if cap_index != 0xFFFF {
                        // _REWIND_DO_SAVE_DUMMY(k)
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0xFAC7;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                } else {
                    let mask0 = tokens[k as usize].mask[0] as usize;
                    let mode = tokens[k as usize].mode;
                    if !just_rewinded {
                        let prev = q_group_stack[mask0];

                        range_max = count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[mask0] != 0 {
                            0
                        } else {
                            count_lo as u64
                        };

                        // minimum requirement not yet met
                        if (q_group_state[mask0] as u64) + 1 < range_min {
                            q_group_state[mask0] += 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            s.group_state = q_group_state[mask0];
                            s.prev = q_group_stack[mask0];
                            q_group_stack[mask0] = stack_n as u32;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;

                            let pair_off = tokens[k as usize].pair_offset as i32;
                            k = ((k as i32) + pair_off) as u32;
                            k = k.wrapping_sub(1);
                            k = k.wrapping_add(1);
                            continue 'main_loop;
                        } else if count_hi != 0 && (q_group_state[mask0] as u64) + 1 > range_max {
                            range_max = range_max.wrapping_sub(1);
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
                            continue 'main_loop;
                        }

                        // detect zero-length matches when backtracking
                        let mut force_zero: bool = false;
                        if prev != 0 && (rewind_stack[prev as usize].i as u32) > (i as u32) {
                            // find matching open paren
                            let pair_off = tokens[k as usize].pair_offset as i32;
                            let target_k = ((k as i32) + pair_off) as u32;
                            let mut n = stack_n.wrapping_sub(1);
                            while n > 0 && rewind_stack[n].k != target_k {
                                n = n.wrapping_sub(1);
                            }
                            if n > 0 && rewind_stack[n].i == i {
                                force_zero = true;
                            }
                        }

                        if force_zero || (prev != 0 && (rewind_stack[prev as usize].i as u32) == (i as u32)) {
                            q_group_accepts_zero[mask0] = 1;
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
                            continue 'main_loop;
                        } else if mode & REMIMU_MODE_LAZY != 0 {
                            q_group_state[mask0] += 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            s.group_state = q_group_state[mask0];
                            s.prev = q_group_stack[mask0];
                            q_group_stack[mask0] = stack_n as u32;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                            q_group_state[mask0] = 0;
                        } else {
                            // greedy
                            if mode & REMIMU_MODE_POSSESSIVE != 0 {
                                let mut k2 = k;
                                if q_group_state[mask0] == 0 {
                                    let pair_off = tokens[k as usize].pair_offset as i32;
                                    k2 = ((k as i32) + pair_off) as u32;
                                }
                                if stack_n == 0 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].k != k2 {
                                    stack_n -= 1;
                                }
                                if stack_n == 0 {
                                    // Only a problem if rewind_stack[0].k != k2
                                    if rewind_stack[0].k != k2 {
                                        return None;
                                    }
                                }
                            }
                            // continue to next match if sane
                            let pair_off = tokens[k as usize].pair_offset as i32;
                            let pair_idx = ((k as i32) + pair_off) as u32;
                            let pair_mask0 = tokens[pair_idx as usize].mask[0] as usize;
                            if (q_group_state[pair_mask0] as u64) < i {
                                q_group_state[mask0] += 1;
                                if stack_n >= STACK_SIZE_MAX {
                                    return None;
                                }
                                let mut s = RegexMatcherState::new(k, i);
                                s.range_min = range_min;
                                s.range_max = range_max;
                                s.prev = 0;
                                s.group_state = q_group_state[mask0];
                                s.prev = q_group_stack[mask0];
                                q_group_stack[mask0] = stack_n as u32;
                                rewind_stack[stack_n] = s;
                                stack_n += 1;
                                let pair_off2 = tokens[k as usize].pair_offset as i32;
                                k = ((k as i32) + pair_off2) as u32;
                                k = k.wrapping_sub(1);
                            }
                        }
                    } else {
                        just_rewinded = false;

                        if mode & REMIMU_MODE_LAZY != 0 {
                            // _REWIND_DO_SAVE_DUMMY(k)
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0xFAC7;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;

                            q_group_stack[mask0] = stack_n as u32;
                            let pair_off = tokens[k as usize].pair_offset as i32;
                            k = ((k as i32) + pair_off) as u32;
                            k = k.wrapping_sub(1);
                        } else {
                            if (q_group_state[mask0] as u64) < range_min
                                && q_group_accepts_zero[mask0] == 0
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
                                continue 'main_loop;
                            } else {
                                q_group_state[mask0] = 0;
                                let cap_index = q_group_cap_index[mask0];
                                if cap_index != 0xFFFF {
                                    if stack_n >= STACK_SIZE_MAX {
                                        return None;
                                    }
                                    let mut s = RegexMatcherState::new(k, i);
                                    s.range_min = range_min;
                                    s.range_max = range_max;
                                    s.prev = 0xFAC7;
                                    rewind_stack[stack_n] = s;
                                    stack_n += 1;
                                }
                            }
                        }
                    }
                }
            } else if cur_kind == REMIMU_KIND_OR {
                let off = tokens[k as usize].pair_offset as i32;
                k = ((k as i32) + off) as u32;
                k = k.wrapping_sub(1);
            } else if cur_kind == REMIMU_KIND_NORMAL {
                if !just_rewinded {
                    let mut n: u64 = 0;
                    let old_i = i;
                    let count_lo = tokens[k as usize].count_lo as u64;
                    let count_hi = tokens[k as usize].count_hi as u64;
                    let mode = tokens[k as usize].mode;
                    while n < count_lo
                        && text_at(i as usize) != 0
                        && tokens[k as usize].check_mask(text_at(i as usize))
                    {
                        i += 1;
                        n += 1;
                    }
                    if n < count_lo {
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
                        continue 'main_loop;
                    }

                    if mode & REMIMU_MODE_LAZY != 0 {
                        range_min = n;
                        range_max = count_hi.wrapping_sub(1);
                        if stack_n >= STACK_SIZE_MAX {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    } else {
                        let mut limit = count_hi;
                        if limit == 0 {
                            limit = !limit;
                        }
                        range_min = n;
                        while text_at(i as usize) != 0
                            && tokens[k as usize].check_mask(text_at(i as usize))
                            && n + 1 < limit
                        {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if mode & REMIMU_MODE_POSSESSIVE == 0 {
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        }
                    }
                } else {
                    just_rewinded = false;
                    let mode = tokens[k as usize].mode;

                    if mode & REMIMU_MODE_LAZY != 0 {
                        let mut limit = range_max;
                        if limit == 0 {
                            limit = !limit;
                        }
                        let ti = text_at(i as usize);
                        if tokens[k as usize].check_mask(ti) && ti != 0 && range_min < limit {
                            i += 1;
                            range_min += 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
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
                            continue 'main_loop;
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            if stack_n >= STACK_SIZE_MAX {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
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
                            continue 'main_loop;
                        }
                    }
                }
            } else {
                return None;
            }
        }

        k = k.wrapping_add(1);
    }

    // collect captures
    if caps != 0 {
        for n in 0..stack_n {
            let s = rewind_stack[n];
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
                if ci >= cap_pos.len() || ci >= cap_span.len() {
                    continue;
                }
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[ci] = s.i as i64;
                } else if cap_pos[ci] >= 0 {
                    cap_span[ci] = s.i as i64 - cap_pos[ci];
                }
            }
        }
        for n in 0..(caps as usize) {
            if n >= cap_span.len() || n >= cap_pos.len() {
                break;
            }
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

    for k in 0..tokens.len() {
        let kind = tokens[k].kind as usize;
        let mode = tokens[k].mode as usize;
        let kind_str = if kind < kind_to_str.len() {
            kind_to_str[kind]
        } else {
            "?"
        };
        let mode_str = if mode < mode_to_str.len() {
            mode_to_str[mode]
        } else {
            "?"
        };

        print!("{}\t{}\t", kind_str, mode_str);

        let mut c_old: i32 = -1;
        let max_c = if tokens[k].kind != 0 { 0 } else { 256 };
        for c in 0..max_c {
            let in_mask = tokens[k].check_mask(c as u8);
            if in_mask {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
                let print_c = |c: i32| {
                    if c >= 0x20 && c <= 0x7E {
                        print!("{}", c as u8 as char);
                    } else {
                        print!("\\x{:02x}", c);
                    }
                };
                if (c as i32) - 1 == c_old {
                    print_c(c_old);
                    c_old = -1;
                } else if (c as i32) - 2 == c_old {
                    print_c(c_old);
                    print_c(c_old + 1);
                    c_old = -1;
                } else {
                    print_c(c_old);
                    print!("-");
                    print_c((c as i32) - 1);
                    c_old = -1;
                }
            }
        }

        println!(
            "\t{{{},{}}}\t({})",
            tokens[k].count_lo,
            (tokens[k].count_hi as i32) - 1,
            tokens[k].pair_offset
        );

        if tokens[k].kind == REMIMU_KIND_END {
            break;
        }
    }
}
