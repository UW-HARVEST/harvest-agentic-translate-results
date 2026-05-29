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
        self.mask[(byte >> 4) as usize] |= 1u16 << (byte & 0x0F);
    }
    pub fn invert_mask(&mut self) {
        for n in 0..16 {
            self.mask[n] = !self.mask[n];
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1u16 << (byte & 0x0F))) != 0
    }
    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        let k = tokens.len();
        let skip = k != 0
            && tokens[k - 1].kind == self.kind
            && (self.kind == REMIMU_KIND_BOUND || self.kind == REMIMU_KIND_NBOUND);
        if !skip {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
            }
            if k >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            // reset to a cleared NORMAL token (count_lo=1, count_hi=2)
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

#[inline]
fn pat_byte(bytes: &[u8], i: usize) -> u8 {
    if i >= bytes.len() {
        0
    } else {
        bytes[i]
    }
}

#[inline]
fn is_word_char(c: u8) -> bool {
    (c >= b'0' && c <= b'9') || (c >= b'A' && c <= b'Z') || (c >= b'a' && c <= b'z') || c == b'_'
}

pub fn regex_parse(
    pattern: &str,
    tokens: &mut Vec<RegexToken>,
    token_count: &mut i16,
    flags: i32,
) -> Result<(), i32> {
    let max_len = *token_count as i64;
    if max_len <= 0 {
        return Err(-2);
    }
    let max_len = max_len as usize;

    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

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

    // start with an invisible group specifier
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
        let mut handled_this_char = false;

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
                let next = pat_byte(pattern_bytes, i + 1);
                if next == 0 || next < b'0' || next > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while {
                        let b = pat_byte(pattern_bytes, i);
                        b >= b'0' && b <= b'9'
                    } {
                        val = val * 10 + (pat_byte(pattern_bytes, i) - b'0') as u32;
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = (val + 1) as u16;
                    if pat_byte(pattern_bytes, i) == b',' {
                        token.count_hi = 0;
                        i += 1;
                        let b = pat_byte(pattern_bytes, i);
                        if b >= b'0' && b <= b'9' {
                            let mut val2: u32 = 0;
                            while {
                                let b = pat_byte(pattern_bytes, i);
                                b >= b'0' && b <= b'9'
                            } {
                                val2 =
                                    val2 * 10 + (pat_byte(pattern_bytes, i) - b'0') as u32;
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
                    if pat_byte(pattern_bytes, i) == b'}' {
                        i += 1;
                        continue;
                    } else {
                        return Err(-1);
                    }
                }
            }
            // fall through
            handled_this_char = false;
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

        let _ = handled_this_char;

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
                    token.set_mask(0x0B);
                } else if c == b'f' {
                    token.set_mask(0x0C);
                } else if c == b'x' {
                    let p1 = pat_byte(pattern_bytes, i + 1);
                    let p2 = pat_byte(pattern_bytes, i + 2);
                    if p1 == 0 || p2 == 0 {
                        return Err(-1);
                    }
                    let mut n0 = p1;
                    // Match the C bug exactly: n1 also reads pattern[i+1]
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
                    let mut m = [0u16; 16];
                    let cl = if is_upper { c + 0x20 } else { c };
                    if cl == b'd' || cl == b'w' {
                        m[3] |= 0x03FF;
                    }
                    if cl == b's' {
                        m[0] |= 0x3E00;
                        m[2] |= 1;
                    }
                    if cl == b'w' {
                        m[4] |= 0xFFFE;
                        m[5] |= 0x87FF;
                        m[6] |= 0xFFFE;
                        m[7] |= 0x07FF;
                    }
                    for x in 0..16 {
                        token.mask[x] |= if is_upper { !m[x] } else { m[x] };
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
                token.push_to_vec(tokens, max_len)?;
                if c == b'\\' {
                    esc_state = 1;
                } else if c == b'[' {
                    state = STATE_CC_INIT;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if pat_byte(pattern_bytes, i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if c == b'(' {
                    paren_count += 1;
                    state = STATE_NORMAL;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if pat_byte(pattern_bytes, i + 1) == b'?'
                        && pat_byte(pattern_bytes, i + 2) == b':'
                    {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if pat_byte(pattern_bytes, i + 1) == b'?'
                        && pat_byte(pattern_bytes, i + 2) == b'>'
                    {
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
                    let diff = (k as i64) - found;
                    if diff > 32767 {
                        return Err(-1);
                    }
                    token.pair_offset = -(diff as i16);
                    tokens[found as usize].pair_offset = diff as i16;
                    if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                        // phantom group for atomic group emulation
                        token.push_to_vec(tokens, max_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        if (found as usize) >= 1 {
                            tokens[(found as usize) - 1].pair_offset = (diff as i16) + 2;
                        }
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
                i += 1;
                continue;
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
                    esc_c = 0x0B;
                } else if c == b'f' {
                    esc_c = 0x0C;
                } else if c == b'x' {
                    let p1 = pat_byte(pattern_bytes, i + 1);
                    let p2 = pat_byte(pattern_bytes, i + 2);
                    if p1 == 0 || p2 == 0 {
                        return Err(-1);
                    }
                    let mut n0 = p1;
                    let mut n1 = p1; // C bug: same as p1
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
                    let mut m = [0u16; 16];
                    let cl = if is_upper { c + 0x20 } else { c };
                    if cl == b'd' || cl == b'w' {
                        m[3] |= 0x03FF;
                    }
                    if cl == b's' {
                        m[0] |= 0x3E00;
                        m[2] |= 1;
                    }
                    if cl == b'w' {
                        m[4] |= 0xFFFE;
                        m[5] |= 0x87FF;
                        m[6] |= 0xFFFE;
                        m[7] |= 0x07FF;
                    }
                    for x in 0..16 {
                        token.mask[x] |= if is_upper { !m[x] } else { m[x] };
                    }
                    char_class_mem = -1;
                    // If we were CC_INIT, we still need to have been "initialised"; but a
                    // char-class starting with \d/etc is fine. The C code does `continue;`
                    // here. State doesn't change so we stay in init/normal.
                    if state == STATE_CC_INIT {
                        // The C code does not transition state, but the next iter's
                        // STATE_CC_INIT branch would re-evaluate. To match the C code
                        // exactly, leave state as is. But practically, the C code's
                        // STATE_CC_INIT needs at least one literal char before exiting
                        // via ']'. Looking again: actually the C `continue`s with state
                        // unchanged, which effectively means the next iter still treats
                        // the class as init (no first literal seen yet). We mirror that.
                        state = STATE_CC_NORMAL;
                    }
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

    token.push_to_vec(tokens, max_len)?;

    // closing invisible group
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, max_len)?;

    // end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len)?;

    let k = tokens.len();
    if k < 3 {
        return Err(-1);
    }
    let kk = k as i16;
    tokens[0].pair_offset = kk - 2;
    tokens[(k - 2) as usize].pair_offset = -(kk - 2);

    *token_count = kk;

    // Second pass: assign group indices and alternation offsets
    let mut n: u64 = 0;
    let mut k2: i16 = 0;
    while (k2 as usize) < k {
        let kind = tokens[k2 as usize].kind;
        if kind == REMIMU_KIND_CLOSE {
            tokens[k2 as usize].mask[0] = (n & 0xFFFF) as u16;
            n += 1;
            let k3 = k2 + tokens[k2 as usize].pair_offset;
            if k3 < 0 || (k3 as usize) >= k {
                return Err(-1);
            }
            tokens[k3 as usize].count_lo = tokens[k2 as usize].count_lo;
            tokens[k3 as usize].count_hi = tokens[k2 as usize].count_hi;
            tokens[k3 as usize].mask[0] = (n & 0xFFFF) as u16;
            n += 1;
            tokens[k3 as usize].mode = tokens[k2 as usize].mode;

            if n > 1024 {
                return Err(-1);
            }
        } else if kind == REMIMU_KIND_OR
            || kind == REMIMU_KIND_OPEN
            || kind == REMIMU_KIND_NCOPEN
        {
            // find next | or ) (at the same paren depth)
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l: i64 = (k2 as i64) + 1;
            while (l as usize) < (max_len as usize) && (l as usize) < k {
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

#[inline]
fn text_byte(text: &[u8], i: u64) -> u8 {
    let i = i as usize;
    if i >= text.len() {
        0
    } else {
        text[i]
    }
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
    let stack_size_max: usize = 1024;
    let aux_stats_size: usize = 1024;

    let mut cap_slots = cap_slots as usize;
    if cap_slots > aux_stats_size {
        cap_slots = aux_stats_size;
    }

    let mut q_group_accepts_zero = vec![0u8; aux_stats_size];
    let mut q_group_state = vec![0u32; aux_stats_size];
    let mut q_group_stack = vec![0u32; aux_stats_size];
    let mut q_group_cap_index = vec![0xFFFFu16; aux_stats_size];

    let mut k: usize = 0;
    let mut caps: usize = 0;

    while k < tokens.len() && tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let m0 = tokens[k].mask[0] as usize;
            if m0 < aux_stats_size {
                q_group_cap_index[m0] = caps as u16;
            }
            let pair_idx = (k as i64) + tokens[k].pair_offset as i64;
            if pair_idx >= 0 && (pair_idx as usize) < tokens.len() {
                let pair_m0 = tokens[pair_idx as usize].mask[0] as usize;
                if pair_m0 < aux_stats_size {
                    q_group_cap_index[pair_m0] = caps as u16;
                }
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
            if m0 >= aux_stats_size {
                return None;
            }
            q_group_state[m0] = 0;
            q_group_stack[m0] = 0;
            q_group_accepts_zero[m0] = 0;
        }
    }

    let tokens_len: usize = k;
    if tokens_len == 0 {
        return None;
    }

    let mut rewind_stack: Vec<RegexMatcherState> =
        (0..stack_size_max).map(|_| RegexMatcherState::new(0, 0)).collect();
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    k = 0;
    'main_loop: while k < tokens_len {
        let kind = tokens[k].kind;

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
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue 'main_loop;
            }
            k += 1;
            continue;
        } else if kind == REMIMU_KIND_DOLLAR {
            if (i as usize) < text_bytes.len() {
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
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue 'main_loop;
            }
            k += 1;
            continue;
        } else if kind == REMIMU_KIND_BOUND || kind == REMIMU_KIND_NBOUND {
            let cur = text_byte(text_bytes, i);
            let cur_w = is_word_char(cur);
            let need_rewind;
            if i == 0 {
                if kind == REMIMU_KIND_BOUND {
                    need_rewind = !cur_w;
                } else {
                    need_rewind = cur_w;
                }
            } else if (i as usize) >= text_bytes.len() {
                let prev = text_byte(text_bytes, i - 1);
                let prev_w = is_word_char(prev);
                if kind == REMIMU_KIND_BOUND {
                    need_rewind = !prev_w;
                } else {
                    need_rewind = prev_w;
                }
            } else {
                let prev = text_byte(text_bytes, i - 1);
                let prev_w = is_word_char(prev);
                if kind == REMIMU_KIND_BOUND {
                    need_rewind = prev_w == cur_w;
                } else {
                    need_rewind = prev_w != cur_w;
                }
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
                k = rewind_stack[stack_n].k as usize;
                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let m0 = tokens[k].mask[0] as usize;
                    q_group_state[m0] = rewind_stack[stack_n].group_state;
                    q_group_stack[m0] = rewind_stack[stack_n].prev;
                }
                continue 'main_loop;
            }
            k += 1;
            continue;
        }

        // Handle a{0} edge case: deliberately unmatchable
        if tokens[k].count_hi == 1 {
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                let new_k = (k as i64) + tokens[k].pair_offset as i64;
                if new_k < 0 {
                    return None;
                }
                k = new_k as usize;
                k += 1;
            } else {
                k += 2;
            }
            continue;
        }

        if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
            if !just_rewinded {
                let pair_k = ((k as i64) + tokens[k].pair_offset as i64) as usize;
                let pair_m0 = tokens[pair_k].mask[0] as usize;
                let lazy = (tokens[k].mode & REMIMU_MODE_LAZY) != 0;
                if lazy && (tokens[k].count_lo == 0 || q_group_accepts_zero[pair_m0] != 0) {
                    range_min = 0;
                    range_max = 0;
                    // save
                    if stack_n >= stack_size_max {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(k as u32, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.prev = 0;
                    rewind_stack[stack_n] = s;
                    stack_n += 1;
                    let new_k = (k as i64) + tokens[k].pair_offset as i64;
                    if new_k < 0 {
                        return None;
                    }
                    k = new_k as usize;
                    k += 1;
                } else {
                    range_min = 1;
                    range_max = 0;
                    if stack_n >= stack_size_max {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(k as u32, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.prev = 0;
                    rewind_stack[stack_n] = s;
                    stack_n += 1;
                    k += 1;
                }
                continue;
            } else {
                just_rewinded = false;
                let orig_k = k;
                if range_min != 0 {
                    k += range_min as usize;
                    if k == 0 {
                        return None;
                    }
                    let prev_kind = tokens[k - 1].kind;
                    if prev_kind == REMIMU_KIND_OR {
                        let off = tokens[k - 1].pair_offset as i64;
                        let nk = (k as i64) + off - 1;
                        if nk < 0 {
                            return None;
                        }
                        k = nk as usize;
                    } else if prev_kind == REMIMU_KIND_OPEN
                        || prev_kind == REMIMU_KIND_NCOPEN
                    {
                        let off = tokens[k - 1].mask[15] as i64;
                        let nk = (k as i64) + off - 1;
                        if nk < 0 {
                            return None;
                        }
                        k = nk as usize;
                    }
                    if k >= tokens.len() {
                        return None;
                    }
                    if tokens[k].kind == REMIMU_KIND_END {
                        return None;
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
                            // rewind
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
                            k = rewind_stack[stack_n].k as usize;
                            if tokens[k].kind == REMIMU_KIND_CLOSE {
                                let m02 = tokens[k].mask[0] as usize;
                                q_group_state[m02] = rewind_stack[stack_n].group_state;
                                q_group_stack[m02] = rewind_stack[stack_n].prev;
                            }
                            continue;
                        }
                    }
                    // assert kind == OR
                }
                let k_diff = (k as i64) - (orig_k as i64);
                range_min = (k_diff + 1) as u64;
                let save_k = ((k as i64) - k_diff) as usize;
                if stack_n >= stack_size_max {
                    return None;
                }
                let mut s = RegexMatcherState::new(save_k as u32, i);
                s.range_min = range_min;
                s.range_max = range_max;
                s.prev = 0;
                rewind_stack[stack_n] = s;
                stack_n += 1;
                k += 1;
                continue;
            }
        } else if kind == REMIMU_KIND_CLOSE {
            // unquantified
            if tokens[k].count_lo == 1 && tokens[k].count_hi == 2 {
                let m0 = tokens[k].mask[0] as usize;
                if m0 < aux_stats_size {
                    let cap_index = q_group_cap_index[m0];
                    if cap_index != 0xFFFF {
                        // dummy save
                        if stack_n >= stack_size_max {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k as u32, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0xFAC7;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                }
                k += 1;
                continue;
            }
            // quantified
            if !just_rewinded {
                let m0 = tokens[k].mask[0] as usize;
                let prev_idx = q_group_stack[m0] as usize;
                range_max = tokens[k].count_hi as u64;
                range_max = range_max.wrapping_sub(1);
                range_min = if q_group_accepts_zero[m0] != 0 {
                    0
                } else {
                    tokens[k].count_lo as u64
                };

                if (q_group_state[m0] as u64) + 1 < range_min {
                    q_group_state[m0] += 1;
                    if stack_n >= stack_size_max {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(k as u32, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.prev = 0;
                    s.group_state = q_group_state[m0];
                    s.prev = q_group_stack[m0];
                    q_group_stack[m0] = stack_n as u32;
                    rewind_stack[stack_n] = s;
                    stack_n += 1;

                    let new_k = (k as i64) + tokens[k].pair_offset as i64;
                    if new_k < 0 {
                        return None;
                    }
                    k = new_k as usize;
                    continue;
                } else if tokens[k].count_hi != 0 && (q_group_state[m0] as u64) + 1 > range_max
                {
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
                    k = rewind_stack[stack_n].k as usize;
                    if tokens[k].kind == REMIMU_KIND_CLOSE {
                        let m02 = tokens[k].mask[0] as usize;
                        q_group_state[m02] = rewind_stack[stack_n].group_state;
                        q_group_stack[m02] = rewind_stack[stack_n].prev;
                    }
                    continue;
                }

                let mut force_zero = false;
                if prev_idx != 0 && (rewind_stack[prev_idx].i as u32) > (i as u32) {
                    let pair_k = ((k as i64) + tokens[k].pair_offset as i64) as usize;
                    let mut n = stack_n.saturating_sub(1);
                    while n > 0 && rewind_stack[n].k as usize != pair_k {
                        n -= 1;
                    }
                    if n > 0 && rewind_stack[n].i == i {
                        force_zero = true;
                    }
                }

                if force_zero
                    || (prev_idx != 0 && (rewind_stack[prev_idx].i as u32) == (i as u32))
                {
                    q_group_accepts_zero[m0] = 1;
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
                    k = rewind_stack[stack_n].k as usize;
                    if tokens[k].kind == REMIMU_KIND_CLOSE {
                        let m02 = tokens[k].mask[0] as usize;
                        q_group_state[m02] = rewind_stack[stack_n].group_state;
                        q_group_stack[m02] = rewind_stack[stack_n].prev;
                    }
                    continue;
                } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                    q_group_state[m0] += 1;
                    if stack_n >= stack_size_max {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(k as u32, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.group_state = q_group_state[m0];
                    s.prev = q_group_stack[m0];
                    q_group_stack[m0] = stack_n as u32;
                    rewind_stack[stack_n] = s;
                    stack_n += 1;
                    q_group_state[m0] = 0;
                    k += 1;
                    continue;
                } else {
                    // greedy
                    if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                        let pair_k =
                            ((k as i64) + tokens[k].pair_offset as i64) as usize;
                        let k2_target = if q_group_state[m0] == 0 { pair_k } else { k };
                        if stack_n == 0 {
                            return None;
                        }
                        stack_n -= 1;
                        while stack_n > 0 && rewind_stack[stack_n].k as usize != k2_target {
                            stack_n -= 1;
                        }
                        if stack_n == 0 && rewind_stack[0].k as usize != k2_target {
                            return None;
                        }
                    }
                    let pair_k = ((k as i64) + tokens[k].pair_offset as i64) as usize;
                    let pair_m0 = tokens[pair_k].mask[0] as usize;
                    if (q_group_state[pair_m0] as u32) < (i as u32) {
                        q_group_state[m0] += 1;
                        if stack_n >= stack_size_max {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k as u32, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.group_state = q_group_state[m0];
                        s.prev = q_group_stack[m0];
                        q_group_stack[m0] = stack_n as u32;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                        let new_k = (k as i64) + tokens[k].pair_offset as i64;
                        if new_k < 0 {
                            return None;
                        }
                        k = new_k as usize;
                        continue;
                    } else {
                        k += 1;
                        continue;
                    }
                }
            } else {
                just_rewinded = false;
                let m0 = tokens[k].mask[0] as usize;
                if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                    // dummy save
                    if stack_n >= stack_size_max {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(k as u32, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.prev = 0xFAC7;
                    rewind_stack[stack_n] = s;
                    stack_n += 1;
                    q_group_stack[m0] = stack_n as u32;
                    let new_k = (k as i64) + tokens[k].pair_offset as i64;
                    if new_k < 0 {
                        return None;
                    }
                    k = new_k as usize;
                    continue;
                } else {
                    if (q_group_state[m0] as u64) < range_min && q_group_accepts_zero[m0] == 0
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
                        k = rewind_stack[stack_n].k as usize;
                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            let m02 = tokens[k].mask[0] as usize;
                            q_group_state[m02] = rewind_stack[stack_n].group_state;
                            q_group_stack[m02] = rewind_stack[stack_n].prev;
                        }
                        continue;
                    } else {
                        q_group_state[m0] = 0;
                        let cap_index = q_group_cap_index[m0];
                        if cap_index != 0xFFFF {
                            if stack_n >= stack_size_max {
                                return None;
                            }
                            let mut s = RegexMatcherState::new(k as u32, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0xFAC7;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        }
                        k += 1;
                        continue;
                    }
                }
            }
        } else if kind == REMIMU_KIND_OR {
            let off = tokens[k].pair_offset as i64;
            let nk = (k as i64) + off;
            if nk < 0 {
                return None;
            }
            k = nk as usize;
            continue;
        } else if kind == REMIMU_KIND_NORMAL {
            if !just_rewinded {
                let mut n: u64 = 0;
                let old_i = i;
                while n < tokens[k].count_lo as u64
                    && (i as usize) < text_bytes.len()
                    && tokens[k].check_mask(text_byte(text_bytes, i))
                {
                    i += 1;
                    n += 1;
                }
                if n < tokens[k].count_lo as u64 {
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
                    k = rewind_stack[stack_n].k as usize;
                    if tokens[k].kind == REMIMU_KIND_CLOSE {
                        let m02 = tokens[k].mask[0] as usize;
                        q_group_state[m02] = rewind_stack[stack_n].group_state;
                        q_group_stack[m02] = rewind_stack[stack_n].prev;
                    }
                    continue;
                }

                if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                    range_min = n;
                    range_max = (tokens[k].count_hi as u64).wrapping_sub(1);
                    if stack_n >= stack_size_max {
                        return None;
                    }
                    let mut s = RegexMatcherState::new(k as u32, i);
                    s.range_min = range_min;
                    s.range_max = range_max;
                    s.prev = 0;
                    rewind_stack[stack_n] = s;
                    stack_n += 1;
                } else {
                    let mut limit = tokens[k].count_hi as u64;
                    if limit == 0 {
                        limit = !limit;
                    }
                    range_min = n;
                    while (i as usize) < text_bytes.len()
                        && tokens[k].check_mask(text_byte(text_bytes, i))
                        && n + 1 < limit
                    {
                        i += 1;
                        n += 1;
                    }
                    range_max = n;
                    if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                        if stack_n >= stack_size_max {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k as u32, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                }
                k += 1;
                continue;
            } else {
                just_rewinded = false;
                if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                    let mut limit = range_max;
                    if limit == 0 {
                        limit = !limit;
                    }
                    if (i as usize) < text_bytes.len()
                        && tokens[k].check_mask(text_byte(text_bytes, i))
                        && range_min < limit
                    {
                        i += 1;
                        range_min += 1;
                        if stack_n >= stack_size_max {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k as u32, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                        k += 1;
                        continue;
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
                        k = rewind_stack[stack_n].k as usize;
                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            let m02 = tokens[k].mask[0] as usize;
                            q_group_state[m02] = rewind_stack[stack_n].group_state;
                            q_group_stack[m02] = rewind_stack[stack_n].prev;
                        }
                        continue;
                    }
                } else {
                    if range_max > range_min {
                        i -= 1;
                        range_max -= 1;
                        if stack_n >= stack_size_max {
                            return None;
                        }
                        let mut s = RegexMatcherState::new(k as u32, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                        k += 1;
                        continue;
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
                        k = rewind_stack[stack_n].k as usize;
                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            let m02 = tokens[k].mask[0] as usize;
                            q_group_state[m02] = rewind_stack[stack_n].group_state;
                            q_group_stack[m02] = rewind_stack[stack_n].prev;
                        }
                        continue;
                    }
                }
            }
        } else {
            return None;
        }
    }

    if caps != 0 {
        for n in 0..stack_n {
            let s_k = rewind_stack[n].k as usize;
            if s_k >= tokens.len() {
                continue;
            }
            let k_kind = tokens[s_k].kind;
            if k_kind == REMIMU_KIND_OPEN || k_kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[s_k].mask[0] as usize;
                if m0 >= aux_stats_size {
                    continue;
                }
                let cap_index = q_group_cap_index[m0];
                if cap_index == 0xFFFF {
                    continue;
                }
                let ci = cap_index as usize;
                if k_kind == REMIMU_KIND_OPEN {
                    if ci < cap_pos.len() {
                        cap_pos[ci] = rewind_stack[n].i as i64;
                    }
                } else if ci < cap_pos.len() && cap_pos[ci] >= 0 {
                    if ci < cap_span.len() {
                        cap_span[ci] = (rewind_stack[n].i as i64) - cap_pos[ci];
                    }
                }
            }
        }
        for n in 0..caps {
            if n < cap_span.len() && cap_span[n] == -1 {
                if n < cap_pos.len() {
                    cap_pos[n] = -1;
                }
            }
        }
    }

    Some(i as usize)
}

fn print_c_smart(c: u8) {
    if c >= 0x20 && c <= 0x7E {
        print!("{}", c as char);
    } else {
        print!("\\x{:02x}", c);
    }
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];

    let mut k = 0;
    while k < tokens.len() {
        let kind_idx = tokens[k].kind as usize;
        let mode_idx = tokens[k].mode as usize;
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
        let max_c = if tokens[k].kind != 0 { 0 } else { 256 };
        let mut c = 0i32;
        while c < max_c {
            let in_mask = tokens[k].check_mask(c as u8);
            if in_mask {
                if c_old == -1 {
                    c_old = c;
                }
            } else if c_old != -1 {
                if c - 1 == c_old {
                    print_c_smart(c_old as u8);
                    c_old = -1;
                } else if c - 2 == c_old {
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
            c += 1;
        }

        let count_hi_minus_one = tokens[k].count_hi.wrapping_sub(1);
        println!(
            "\t{{{},{}}}\t({})",
            tokens[k].count_lo, count_hi_minus_one, tokens[k].pair_offset
        );

        if tokens[k].kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}
