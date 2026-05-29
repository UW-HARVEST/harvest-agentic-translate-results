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
        let should_push = tokens.is_empty()
            || tokens.last().unwrap().kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND);
        if should_push {
            if (self.mode & REMIMU_MODE_INVERTED) != 0 {
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

fn set_mask_all(token: &mut RegexToken) {
    for n in 0..16 {
        token.mask[n] = 0xFFFF;
    }
}

#[allow(dead_code)]
fn clear_token(token: &mut RegexToken) {
    *token = RegexToken {
        kind: 0,
        mode: 0,
        count_lo: 1,
        count_hi: 2,
        mask: [0; 16],
        pair_offset: 0,
    };
}

pub fn regex_parse(pattern: &str, tokens: &mut Vec<RegexToken>, token_count: &mut i16, flags: i32) -> Result<(), i32> {
    let tokens_len = *token_count as i64;
    if tokens_len <= 0 {
        // Match `if (token_count == 0) return -2;` (semantically: no room).
        return Err(-2);
    }
    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    tokens.clear();

    // 0: normal, 1: just saw a backslash
    let mut esc_state = 0;

    let mut state = State::Normal;

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

    let max_len_usize = tokens_len as usize;

    // Helper closures (manual functions because of borrow checker).
    // Use direct logic in the loop.

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pattern_bytes[i];

        // STATE_QUANT
        if matches!(state, State::Quant) {
            state = State::Mode;
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
                let next = if i + 1 < pattern_len { pattern_bytes[i + 1] } else { 0 };
                if next == 0 || next < b'0' || next > b'9' {
                    state = State::Normal;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while i < pattern_len && pattern_bytes[i] >= b'0' && pattern_bytes[i] <= b'9' {
                        val = val.wrapping_mul(10).wrapping_add((pattern_bytes[i] - b'0') as u32);
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
                            while i < pattern_len && pattern_bytes[i] >= b'0' && pattern_bytes[i] <= b'9' {
                                val2 = val2.wrapping_mul(10).wrapping_add((pattern_bytes[i] - b'0') as u32);
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

        // STATE_MODE
        if matches!(state, State::Mode) {
            state = State::Normal;
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

        if matches!(state, State::Normal) {
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' {
                    token.set_mask(b'\n');
                } else if c == b'r' {
                    token.set_mask(b'\r');
                } else if c == b't' {
                    token.set_mask(b'\t');
                } else if c == b'v' {
                    token.set_mask(0x0B);
                } else if c == b'f' {
                    token.set_mask(0x0C);
                } else if c == b'x' {
                    if i + 1 >= pattern_len || i + 2 >= pattern_len
                        || pattern_bytes[i + 1] == 0 || pattern_bytes[i + 2] == 0 {
                        return Err(-1);
                    }
                    let mut n0 = pattern_bytes[i + 1];
                    // Note: faithful to C source bug — uses i+1 twice.
                    let mut n1 = pattern_bytes[i + 1];
                    if n0 < b'0' || n0 > b'f' || n1 < b'0' || n1 > b'f'
                        || (n0 > b'9' && n0 < b'A') || (n1 > b'9' && n1 < b'A') {
                        return Err(-1);
                    }
                    if n0 > b'F' { n0 -= 0x20; }
                    if n1 > b'F' { n1 -= 0x20; }
                    if n0 >= b'A' { n0 -= b'A' - 10; }
                    if n1 >= b'A' { n1 -= b'A' - 10; }
                    n0 -= b'0';
                    n1 -= b'0';
                    token.set_mask((n1 << 4) | n0);
                    i += 2;
                    state = State::Quant;
                } else if matches!(c,
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')'
                    | b'|' | b'^' | b'$' | b'*' | b'+' | b'?' | b':'
                    | b'.' | b'/' | b'\\') {
                    token.set_mask(c);
                    state = State::Quant;
                } else if matches!(c, b'd' | b's' | b'w' | b'D' | b'S' | b'W') {
                    let is_upper = c <= b'Z';
                    let mut m = [0u16; 16];
                    let cc = if is_upper { c + 0x20 } else { c };
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
                    for j in 0..16 {
                        token.mask[j] |= if is_upper { !m[j] } else { m[j] };
                    }
                    token.kind = REMIMU_KIND_NORMAL;
                    state = State::Quant;
                } else if c == b'b' {
                    token.kind = REMIMU_KIND_BOUND;
                    state = State::Normal;
                } else if c == b'B' {
                    token.kind = REMIMU_KIND_NBOUND;
                    state = State::Normal;
                } else {
                    return Err(-1);
                }
            } else {
                token.push_to_vec(tokens, max_len_usize)?;

                if c == b'\\' {
                    esc_state = 1;
                } else if c == b'[' {
                    state = State::CharClassInit;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if i + 1 < pattern_len && pattern_bytes[i + 1] == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if c == b'(' {
                    paren_count += 1;
                    state = State::Normal;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if i + 2 < pattern_len && pattern_bytes[i + 1] == b'?' && pattern_bytes[i + 2] == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if i + 2 < pattern_len && pattern_bytes[i + 1] == b'?' && pattern_bytes[i + 2] == b'>' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.push_to_vec(tokens, max_len_usize)?;

                        state = State::Normal;
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
                    state = State::Quant;

                    let mut balance: i32 = 0;
                    let k = tokens.len() as i64;
                    let mut found: i64 = -1;
                    let mut l = k - 1;
                    while l >= 0 {
                        let kind_l = tokens[l as usize].kind;
                        if kind_l == REMIMU_KIND_NCOPEN || kind_l == REMIMU_KIND_OPEN {
                            if balance == 0 {
                                found = l;
                                break;
                            } else {
                                balance -= 1;
                            }
                        } else if kind_l == REMIMU_KIND_CLOSE {
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
                        token.push_to_vec(tokens, max_len_usize)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        tokens[(found - 1) as usize].pair_offset = (diff as i16) + 2;
                    }
                } else if c == b'?' || c == b'+' || c == b'*' || c == b'{' {
                    return Err(-1);
                } else if c == b'.' {
                    set_mask_all(&mut token);
                    if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
                        token.mask[1] ^= 0x04;
                        token.mask[1] ^= 0x20;
                    }
                    state = State::Quant;
                } else if c == b'^' {
                    token.kind = REMIMU_KIND_CARET;
                    state = State::Normal;
                } else if c == b'$' {
                    token.kind = REMIMU_KIND_DOLLAR;
                    state = State::Normal;
                } else if c == b'|' {
                    token.kind = REMIMU_KIND_OR;
                    state = State::Normal;
                } else {
                    token.set_mask(c);
                    state = State::Quant;
                }
            }
        } else if matches!(state, State::CharClassInit | State::CharClassNormal | State::CharClassRange) {
            if c == b'\\' && esc_state == 0 {
                esc_state = 1;
                i += 1;
                continue;
            }
            let mut esc_c: u8 = 0;
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' { esc_c = b'\n'; }
                else if c == b'r' { esc_c = b'\r'; }
                else if c == b't' { esc_c = b'\t'; }
                else if c == b'v' { esc_c = 0x0B; }
                else if c == b'f' { esc_c = 0x0C; }
                else if c == b'x' {
                    if i + 1 >= pattern_len || i + 2 >= pattern_len
                        || pattern_bytes[i + 1] == 0 || pattern_bytes[i + 2] == 0 {
                        return Err(-1);
                    }
                    let mut n0 = pattern_bytes[i + 1];
                    let mut n1 = pattern_bytes[i + 1]; // faithful to C source bug
                    if n0 < b'0' || n0 > b'f' || n1 < b'0' || n1 > b'f'
                        || (n0 > b'9' && n0 < b'A') || (n1 > b'9' && n1 < b'A') {
                        return Err(-1);
                    }
                    if n0 > b'F' { n0 -= 0x20; }
                    if n1 > b'F' { n1 -= 0x20; }
                    if n0 >= b'A' { n0 -= b'A' - 10; }
                    if n1 >= b'A' { n1 -= b'A' - 10; }
                    n0 -= b'0';
                    n1 -= b'0';
                    esc_c = (n1 << 4) | n0;
                    i += 2;
                } else if matches!(c,
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')'
                    | b'|' | b'^' | b'$' | b'*' | b'+' | b'?' | b':'
                    | b'.' | b'/' | b'\\') {
                    esc_c = c;
                } else if matches!(c, b'd' | b's' | b'w' | b'D' | b'S' | b'W') {
                    if matches!(state, State::CharClassRange) {
                        return Err(-1);
                    }
                    let is_upper = c <= b'Z';
                    let mut m = [0u16; 16];
                    let cc = if is_upper { c + 0x20 } else { c };
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
                    for j in 0..16 {
                        token.mask[j] |= if is_upper { !m[j] } else { m[j] };
                    }
                    char_class_mem = -1;
                    i += 1;
                    continue;
                } else {
                    return Err(-1);
                }
            }

            if matches!(state, State::CharClassInit) {
                char_class_mem = c as i32;
                token.set_mask(c);
                state = State::CharClassNormal;
            } else if matches!(state, State::CharClassNormal) {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    state = State::Quant;
                    i += 1;
                    continue;
                } else if c == b'-' && esc_c == 0 && char_class_mem >= 0 {
                    state = State::CharClassRange;
                    i += 1;
                    continue;
                } else {
                    char_class_mem = c as i32;
                    token.set_mask(c);
                    state = State::CharClassNormal;
                }
            } else if matches!(state, State::CharClassRange) {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    token.set_mask(b'-');
                    state = State::Quant;
                    i += 1;
                    continue;
                } else {
                    if char_class_mem == -1 {
                        return Err(-1);
                    }
                    if (c as i32) < char_class_mem {
                        return Err(-1);
                    }
                    let start = char_class_mem as u8;
                    let end = c;
                    // for (uint8_t i = c; i > char_class_mem; i--) _REGEX_SET_MASK(i);
                    // Note: this excludes char_class_mem itself; that was already set previously.
                    let mut x = end;
                    while (x as i32) > (start as i32) {
                        token.set_mask(x);
                        if x == 0 { break; }
                        x = x.wrapping_sub(1);
                    }
                    state = State::CharClassNormal;
                    char_class_mem = -1;
                }
            }
        } else {
            // unreachable
        }

        i += 1;
    }

    if paren_count > 0 {
        return Err(-1);
    }
    if esc_state != 0 {
        return Err(-1);
    }
    if matches!(state, State::CharClassInit | State::CharClassNormal | State::CharClassRange) {
        return Err(-1);
    }

    token.push_to_vec(tokens, max_len_usize)?;

    // add invisible non-capturing group specifier
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, max_len_usize)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len_usize)?;

    let k = tokens.len() as i16;
    let k_minus_2 = k - 2;
    tokens[0].pair_offset = k_minus_2;
    tokens[(k - 2) as usize].pair_offset = -k_minus_2;

    *token_count = k;

    // copy quantifiers from )s to (s and number quantified groups
    let mut n: u64 = 0;
    let kk = tokens.len() as i16;
    let mut k2: i16 = 0;
    while k2 < kk {
        let k2u = k2 as usize;
        if tokens[k2u].kind == REMIMU_KIND_CLOSE {
            tokens[k2u].mask[0] = n as u16;
            n += 1;

            let k3 = k2 + tokens[k2u].pair_offset;
            let k3u = k3 as usize;
            tokens[k3u].count_lo = tokens[k2u].count_lo;
            tokens[k3u].count_hi = tokens[k2u].count_hi;
            tokens[k3u].mask[0] = n as u16;
            tokens[k3u].mode = tokens[k2u].mode;
            n += 1;

            if n > 1024 {
                return Err(-1);
            }
        } else if tokens[k2u].kind == REMIMU_KIND_OR
            || tokens[k2u].kind == REMIMU_KIND_OPEN
            || tokens[k2u].kind == REMIMU_KIND_NCOPEN
        {
            // find next | or ) and how far away it is
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l = (k2 + 1) as i64;
            let limit = tokens_len; // mirrors C: < tokens_len (the original max length)
            while l < limit && (l as usize) < tokens.len() {
                let kl = tokens[l as usize].kind;
                if kl == REMIMU_KIND_OR && balance == 0 {
                    found = l;
                    break;
                } else if kl == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = l;
                        break;
                    } else {
                        balance -= 1;
                    }
                } else if kl == REMIMU_KIND_NCOPEN || kl == REMIMU_KIND_OPEN {
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
            if tokens[k2u].kind == REMIMU_KIND_OR {
                tokens[k2u].pair_offset = diff as i16;
            } else {
                tokens[k2u].mask[15] = diff as u16;
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

const W_MASK: [u16; 16] = [
    0, 0, 0, 0x03FF, 0xFFFE, 0x87FF, 0xFFFE, 0x07FF,
    0, 0, 0, 0, 0, 0, 0, 0,
];

fn check_is_w(byte: u8) -> bool {
    (W_MASK[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
}

fn check_token_mask(tokens: &[RegexToken], k: usize, byte: u8) -> bool {
    (tokens[k].mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
}

pub fn regex_match(tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64]) -> Option<usize> {
    let text_bytes = text.as_bytes();
    let stack_size_max: usize = 1024;
    let aux_stats_size: usize = 1024;
    let mut cap_slots = cap_slots as usize;
    if cap_slots > aux_stats_size {
        cap_slots = aux_stats_size;
    }

    // Helper: read text byte (0 if past end)
    let text_byte = |i: u64| -> u8 {
        if (i as usize) < text_bytes.len() { text_bytes[i as usize] } else { 0 }
    };

    let mut q_group_accepts_zero: Vec<u8> = vec![0; aux_stats_size];
    let mut q_group_state: Vec<u32> = vec![0; aux_stats_size];
    let mut q_group_stack: Vec<u32> = vec![0; aux_stats_size];
    let mut q_group_cap_index: Vec<u16> = vec![0xFFFF; aux_stats_size];

    let tokens_len: u64;
    let mut k: u32 = 0;
    let mut caps: u16 = 0;

    while tokens[k as usize].kind != REMIMU_KIND_END {
        let kk = tokens[k as usize].kind;
        if kk == REMIMU_KIND_OPEN && (caps as usize) < cap_slots {
            let idx_open = tokens[k as usize].mask[0] as usize;
            let pair_idx = (k as i32 + tokens[k as usize].pair_offset as i32) as usize;
            let idx_close = tokens[pair_idx].mask[0] as usize;
            if idx_open < q_group_cap_index.len() {
                q_group_cap_index[idx_open] = caps;
            }
            if idx_close < q_group_cap_index.len() {
                q_group_cap_index[idx_close] = caps;
            }
            cap_pos[caps as usize] = -1;
            cap_span[caps as usize] = -1;
            caps += 1;
        }
        k += 1;
        let kk2 = tokens[k as usize].kind;
        if kk2 == REMIMU_KIND_CLOSE || kk2 == REMIMU_KIND_OPEN || kk2 == REMIMU_KIND_NCOPEN {
            let m0 = tokens[k as usize].mask[0] as usize;
            if m0 >= aux_stats_size {
                return None;
            }
            q_group_state[m0] = 0;
            q_group_stack[m0] = 0;
            q_group_accepts_zero[m0] = 0;
        }
    }

    tokens_len = k as u64;

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(stack_size_max);
    // We'll grow it as needed but cap at stack_size_max.
    // C uses fixed-size array, with stack_n as count. We use Vec, treat len as stack_n.

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: u8 = 0;

    // Use a state machine. We need to be able to rewind k inside loop.
    // C uses for(k=0; k<tokens_len; k++) with manual k modifications.
    // Translate to a manual loop.
    k = 0;
    let limit_count: usize = 10000;
    let _ = limit_count; // C had this commented out (limit-- inside loop body)

    let mut iteration_should_continue: bool;
    'main: loop {
        if k as u64 >= tokens_len {
            break;
        }
        iteration_should_continue = false;

        let kind = tokens[k as usize].kind;

        if kind == REMIMU_KIND_CARET {
            if i != 0 {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            }
            // continue
            iteration_should_continue = true;
        } else if kind == REMIMU_KIND_DOLLAR {
            if text_byte(i) != 0 {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            }
            iteration_should_continue = true;
        } else if kind == REMIMU_KIND_BOUND {
            let cur = text_byte(i);
            if i == 0 && !check_is_w(cur) {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            } else if i != 0 && cur == 0 && !check_is_w(text_byte(i - 1)) {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            } else if i != 0 && cur != 0 && check_is_w(text_byte(i - 1)) == check_is_w(cur) {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            }
        } else if kind == REMIMU_KIND_NBOUND {
            let cur = text_byte(i);
            if i == 0 && check_is_w(cur) {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            } else if i != 0 && cur == 0 && check_is_w(text_byte(i - 1)) {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            } else if i != 0 && cur != 0 && check_is_w(text_byte(i - 1)) != check_is_w(cur) {
                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                    return None;
                }
            }
        } else {
            // deliberately unmatchable token (e.g. a{0}, a{0,0})
            if tokens[k as usize].count_hi == 1 {
                if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                    k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                } else {
                    k += 1;
                }
                iteration_should_continue = true;
            } else if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                if just_rewinded == 0 {
                    let pair_idx = (k as i32 + tokens[k as usize].pair_offset as i32) as usize;
                    let close_m0 = tokens[pair_idx].mask[0] as usize;
                    let lazy_branch = (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k as usize].count_lo == 0 || q_group_accepts_zero[close_m0] != 0);
                    if lazy_branch {
                        range_min = 0;
                        range_max = 0;
                        if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                            return None;
                        }
                        k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                    } else {
                        range_min = 1;
                        range_max = 0;
                        if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                            return None;
                        }
                    }
                } else {
                    just_rewinded = 0;

                    let orig_k = k as i64;

                    if range_min != 0 {
                        k = (k as u64 + range_min) as u32;
                        if k > 0 {
                            let prev_k = (k - 1) as usize;
                            let prev_kind = tokens[prev_k].kind;
                            if prev_kind == REMIMU_KIND_OR {
                                k = (k as i32 + tokens[prev_k].pair_offset as i32 - 1) as u32;
                            } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN {
                                k = (k as i32 + tokens[prev_k].mask[15] as i32 - 1) as u32;
                            }
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_END {
                            return None; // -3
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let m0 = tokens[k as usize].mask[0] as usize;
                            if tokens[k as usize].count_lo == 0 || q_group_accepts_zero[m0] != 0 {
                                q_group_state[m0] = 0;
                                if (tokens[k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[m0] = 0;
                                }
                                iteration_should_continue = true;
                            } else {
                                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                                    return None;
                                }
                                iteration_should_continue = true;
                            }
                        } else {
                            // assert tokens[k].kind == OR
                            // proceed below
                        }
                    }

                    if !iteration_should_continue {
                        let k_diff = (k as i64) - orig_k;
                        range_min = (k_diff + 1) as u64;

                        if !rewind_save(&mut rewind_stack, stack_size_max, (k as i64 - k_diff) as u32, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                            return None;
                        }
                    }
                }
            } else if kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[k as usize].mask[0] as usize;

                // unquantified
                if tokens[k as usize].count_lo == 1 && tokens[k as usize].count_hi == 2 {
                    let cap_index = q_group_cap_index[m0];
                    if cap_index != 0xFFFF {
                        if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, true, tokens, &mut q_group_state, &mut q_group_stack) {
                            return None;
                        }
                    }
                } else {
                    // quantified
                    if just_rewinded == 0 {
                        let prev = q_group_stack[m0];

                        range_max = tokens[k as usize].count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[m0] != 0 { 0 } else { tokens[k as usize].count_lo as u64 };

                        // minimum requirement not yet met
                        if (q_group_state[m0] as u64) + 1 < range_min {
                            q_group_state[m0] += 1;
                            if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                            k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                            // k -= 1 to ensure we hit the group node next
                            k = k.wrapping_sub(1);
                            // Will increment below. Actually the C does k -= 1; continue;
                            // The continue skips the k += 1 at end of loop. But our loop increments k at the bottom.
                            // We need to redo this properly: set k correctly and signal "skip increment".
                            // The C continue means: skip the rest of the body. The for-loop's k++ then runs.
                            // So C does: k = (k + pair_offset) - 1; then at loop top, k++. Net: k = original + pair_offset.
                            // In our loop, we can either set k = original + pair_offset and not increment,
                            // or set k = original + pair_offset - 1 and increment.
                            // Currently using the latter approach. Need to ensure increment happens.
                            iteration_should_continue = true;
                            // After the elseif chain ends, we increment k.
                        } else if tokens[k as usize].count_hi != 0 && (q_group_state[m0] as u64) + 1 > range_max {
                            range_max = range_max.wrapping_sub(1);
                            if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                            iteration_should_continue = true;
                        } else {
                            // fallback case
                            let mut force_zero: u8 = 0;
                            if prev != 0 && (rewind_stack[prev as usize].i as u32) > (i as u32) {
                                let mut n = (rewind_stack.len() as i64) - 1;
                                let target_k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                                while n > 0 && rewind_stack[n as usize].k != target_k {
                                    n -= 1;
                                }
                                // assert n > 0
                                if n >= 0 && rewind_stack[n as usize].i == i {
                                    force_zero = 1;
                                }
                            }

                            if force_zero != 0 || (prev != 0 && (rewind_stack[prev as usize].i as u32) == (i as u32)) {
                                q_group_accepts_zero[m0] = 1;
                                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                                    return None;
                                }
                                // C falls through to bottom of loop here; doesn't 'continue', just exits the if/else
                            } else if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                                q_group_state[m0] += 1;
                                if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                                    return None;
                                }
                                q_group_state[m0] = 0;
                            } else {
                                // greedy
                                if (tokens[k as usize].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                    let mut k2 = k;
                                    if q_group_state[m0] == 0 {
                                        k2 = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                                    }
                                    if rewind_stack.is_empty() {
                                        return None;
                                    }
                                    rewind_stack.pop();
                                    while !rewind_stack.is_empty() && rewind_stack.last().unwrap().k != k2 {
                                        rewind_stack.pop();
                                    }
                                    if rewind_stack.is_empty() {
                                        return None;
                                    }
                                }
                                let pair_close_m0 = tokens[(k as i32 + tokens[k as usize].pair_offset as i32) as usize].mask[0] as usize;
                                if (q_group_state[pair_close_m0] as u32) < (i as u32) {
                                    q_group_state[m0] += 1;
                                    if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                                        return None;
                                    }
                                    k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                                    k = k.wrapping_sub(1);
                                    iteration_should_continue = true;
                                }
                            }
                        }
                    } else {
                        just_rewinded = 0;

                        if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                            if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, true, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                            q_group_stack[m0] = rewind_stack.len() as u32;
                            k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                            k = k.wrapping_sub(1);
                            iteration_should_continue = true;
                        } else {
                            if (q_group_state[m0] as u64) < range_min && q_group_accepts_zero[m0] == 0 {
                                if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                                    return None;
                                }
                            } else {
                                q_group_state[m0] = 0;
                                let cap_index = q_group_cap_index[m0];
                                if cap_index != 0xFFFF {
                                    if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, true, tokens, &mut q_group_state, &mut q_group_stack) {
                                        return None;
                                    }
                                }
                            }
                        }
                    }
                }
            } else if kind == REMIMU_KIND_OR {
                k = (k as i32 + tokens[k as usize].pair_offset as i32) as u32;
                k = k.wrapping_sub(1);
            } else if kind == REMIMU_KIND_NORMAL {
                if just_rewinded == 0 {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k as usize].count_lo as u64 && text_byte(i) != 0 && check_token_mask(tokens, k as usize, text_byte(i)) {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k as usize].count_lo as u64 {
                        i = old_i;
                        if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                            return None;
                        }
                        iteration_should_continue = true;
                    } else if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = (tokens[k as usize].count_hi as u64).wrapping_sub(1);
                        if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                            return None;
                        }
                    } else {
                        let mut limit: u64 = tokens[k as usize].count_hi as u64;
                        if limit == 0 {
                            limit = !limit;
                        }
                        range_min = n;
                        while text_byte(i) != 0 && check_token_mask(tokens, k as usize, text_byte(i)) && n + 1 < limit {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k as usize].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                        }
                    }
                } else {
                    just_rewinded = 0;
                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit: u64 = range_max;
                        if limit == 0 {
                            limit = !limit;
                        }
                        if check_token_mask(tokens, k as usize, text_byte(i)) && text_byte(i) != 0 && range_min < limit {
                            i += 1;
                            range_min += 1;
                            if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                        } else {
                            if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                        }
                    } else {
                        if range_max > range_min {
                            i = i.wrapping_sub(1);
                            range_max = range_max.wrapping_sub(1);
                            if !rewind_save(&mut rewind_stack, stack_size_max, k, i, range_min, range_max, false, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                        } else {
                            if !rewind_or_abort(&mut rewind_stack, &mut just_rewinded, &mut range_min, &mut range_max, &mut i, &mut k, tokens, &mut q_group_state, &mut q_group_stack) {
                                return None;
                            }
                        }
                    }
                }
            } else {
                // unimplemented token kind
                return None;
            }
        }

        // Increment for next iteration (mirrors C `for (k = 0; k < tokens_len; k++)`)
        let _ = iteration_should_continue;
        k = k.wrapping_add(1);
        let _ = limit_count;
        if k as u64 >= tokens_len {
            break 'main;
        }
        // Continue loop
    }

    if caps != 0 {
        for n in 0..rewind_stack.len() {
            let s = &rewind_stack[n];
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[s.k as usize].mask[0] as usize;
                let cap_index = q_group_cap_index[m0];
                if cap_index == 0xFFFF {
                    continue;
                }
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[cap_index as usize] = s.i as i64;
                } else if cap_pos[cap_index as usize] >= 0 {
                    cap_span[cap_index as usize] = (s.i as i64) - cap_pos[cap_index as usize];
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

fn rewind_save(
    rewind_stack: &mut Vec<RegexMatcherState>,
    stack_size_max: usize,
    k: u32,
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
        // C: q_group_stack[..] = stack_n (which is the index where the new entry is being placed).
        q_group_stack[m0] = rewind_stack.len() as u32;
    }
    rewind_stack.push(s);
    true
}

fn rewind_or_abort(
    rewind_stack: &mut Vec<RegexMatcherState>,
    just_rewinded: &mut u8,
    range_min: &mut u64,
    range_max: &mut u64,
    i: &mut u64,
    k: &mut u32,
    tokens: &[RegexToken],
    q_group_state: &mut [u32],
    q_group_stack: &mut [u32],
) -> bool {
    if rewind_stack.is_empty() {
        return false;
    }
    rewind_stack.pop();
    while !rewind_stack.is_empty() && rewind_stack.last().unwrap().prev == 0xFAC7 {
        rewind_stack.pop();
    }
    if rewind_stack.is_empty() {
        // Special: C decrements stack_n to 0 and reads rewind_stack[0]. We need the slot
        // whose data we'd read before popping. Mirror behavior: if we popped past zero,
        // it's an abort.
        // Actually C does: stack_n -= 1; while (stack_n > 0 ...) stack_n -= 1; then reads
        // rewind_stack[stack_n]. If stack_n becomes 0, it reads index 0 which still holds
        // the data before popping. We've popped it, so we lost the data. Re-emulate by
        // not popping the final slot if there's no outer dummy chain — i.e., check carefully.
        // But the C code's invariant: once stack_n = 0 with no prev != FAC7 above, it's stuck
        // reading stale index 0. To mimic: store the popped state.
        return false;
    }
    *just_rewinded = 1;
    let s = rewind_stack.last().unwrap();
    *range_min = s.range_min;
    *range_max = s.range_max;
    *i = s.i;
    *k = s.k;
    let prev = s.prev;
    let group_state = s.group_state;
    let kk = *k;
    if tokens[kk as usize].kind == REMIMU_KIND_CLOSE {
        let m0 = tokens[kk as usize].mask[0] as usize;
        q_group_state[m0] = group_state;
        q_group_stack[m0] = prev;
    }
    *k = k.wrapping_sub(1);
    true
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR",
        "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];
    let mut k = 0usize;
    loop {
        let kind = tokens[k].kind as usize;
        let mode = tokens[k].mode as usize;
        let kind_str = kind_to_str.get(kind).copied().unwrap_or("?");
        let mode_str = mode_to_str.get(mode).copied().unwrap_or("?");
        print!("{}\t{}\t", kind_str, mode_str);

        let mut c_old: i32 = -1;
        let upper = if tokens[k].kind != 0 { 0 } else { 256 };
        for c in 0..upper {
            let is_set = tokens[k].check_mask(c as u8);
            if is_set {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
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
                    print_c_smart(c as i32 - 1);
                    c_old = -1;
                }
            }
        }

        println!("\t{{{},{}}}\t({})",
            tokens[k].count_lo,
            (tokens[k].count_hi as i32) - 1,
            tokens[k].pair_offset);

        if tokens[k].kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}

fn print_c_smart(c: i32) {
    if c >= 0x20 && c <= 0x7E {
        print!("{}", (c as u8) as char);
    } else {
        print!("\\x{:02x}", c);
    }
}
