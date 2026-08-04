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
        let bit = 1u16 << (byte & 0xF);
        self.mask[idx] |= bit;
    }
    pub fn invert_mask(&mut self) {
        for n in 0..16 {
            self.mask[n] = !self.mask[n];
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        let idx = (byte >> 4) as usize;
        let bit = 1u16 << (byte & 0xF);
        (self.mask[idx] & bit) != 0
    }
    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        let k = tokens.len();
        // C condition: k == 0 || tokens[k-1].kind != token.kind ||
        //              (token.kind != BOUND && token.kind != NBOUND)
        // When this is true, push the token.
        let should_push = k == 0
            || tokens[k - 1].kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND);
        if should_push {
            if (self.mode & REMIMU_MODE_INVERTED) != 0 {
                self.invert_mask();
            }
            if k >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            // Reset to default
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

fn clear_token() -> RegexToken {
    RegexToken {
        kind: REMIMU_KIND_NORMAL,
        mode: 0,
        count_lo: 1,
        count_hi: 2,
        mask: [0; 16],
        pair_offset: 0,
    }
}

pub fn regex_parse(pattern: &str, tokens: &mut Vec<RegexToken>, token_count: &mut i16, flags: i32) -> Result<(), i32> {
    let tokens_len = *token_count as i64;
    if tokens_len <= 0 {
        return Err(-2);
    }
    let max_len = tokens_len as usize;
    tokens.clear();

    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    // helper: get pattern byte at index, returning 0 if out of bounds (mimic null terminator)
    let pat = |idx: usize| -> u8 {
        if idx < pattern_len { pattern_bytes[idx] } else { 0 }
    };

    let mut esc_state = 0;

    const STATE_NORMAL: i32 = 1;
    const STATE_QUANT: i32 = 2;
    const STATE_MODE: i32 = 3;
    const STATE_CC_INIT: i32 = 4;
    const STATE_CC_NORMAL: i32 = 5;
    const STATE_CC_RANGE: i32 = 6;
    let mut state: i32 = STATE_NORMAL;

    let mut char_class_mem: i32 = -1;

    let mut token = clear_token();

    // start with an invisible group specifier
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pattern_bytes[i] as i32;
        let cc = c as u8;

        if state == STATE_QUANT {
            state = STATE_MODE;
            if cc == b'?' {
                token.count_lo = 0;
                token.count_hi = 2;
                i += 1;
                continue;
            } else if cc == b'+' {
                token.count_lo = 1;
                token.count_hi = 0;
                i += 1;
                continue;
            } else if cc == b'*' {
                token.count_lo = 0;
                token.count_hi = 0;
                i += 1;
                continue;
            } else if cc == b'{' {
                let next = pat(i + 1);
                if next == 0 || next < b'0' || next > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while pat(i) >= b'0' && pat(i) <= b'9' {
                        val = val.wrapping_mul(10);
                        val = val.wrapping_add((pat(i) - b'0') as u32);
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = (val + 1) as u16;
                    if pat(i) == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if pat(i) >= b'0' && pat(i) <= b'9' {
                            let mut val2: u32 = 0;
                            while pat(i) >= b'0' && pat(i) <= b'9' {
                                val2 = val2.wrapping_mul(10);
                                val2 = val2.wrapping_add((pat(i) - b'0') as u32);
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
                    if pat(i) == b'}' {
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
            if cc == b'?' {
                token.mode |= REMIMU_MODE_LAZY;
                i += 1;
                continue;
            } else if cc == b'+' {
                token.mode |= REMIMU_MODE_POSSESSIVE;
                i += 1;
                continue;
            }
        }

        if state == STATE_NORMAL {
            if esc_state == 1 {
                esc_state = 0;
                if cc == b'n' {
                    token.set_mask(b'\n');
                    state = STATE_QUANT;
                } else if cc == b'r' {
                    token.set_mask(b'\r');
                    state = STATE_QUANT;
                } else if cc == b't' {
                    token.set_mask(b'\t');
                    state = STATE_QUANT;
                } else if cc == b'v' {
                    token.set_mask(0x0B);
                    state = STATE_QUANT;
                } else if cc == b'f' {
                    token.set_mask(0x0C);
                    state = STATE_QUANT;
                } else if cc == b'x' {
                    if pat(i + 1) == 0 || pat(i + 2) == 0 {
                        return Err(-1);
                    }
                    // NOTE: Bug in original C: it uses pattern[i+1] for both n0 and n1
                    let mut n0 = pat(i + 1);
                    let mut n1 = pat(i + 1);
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
                    state = STATE_QUANT;
                } else if cc == b'{' || cc == b'}' || cc == b'[' || cc == b']' || cc == b'-'
                    || cc == b'(' || cc == b')' || cc == b'|' || cc == b'^' || cc == b'$'
                    || cc == b'*' || cc == b'+' || cc == b'?' || cc == b':' || cc == b'.'
                    || cc == b'/' || cc == b'\\' {
                    token.set_mask(cc);
                    state = STATE_QUANT;
                } else if cc == b'd' || cc == b's' || cc == b'w'
                    || cc == b'D' || cc == b'S' || cc == b'W' {
                    let is_upper = cc <= b'Z';
                    let mut m = [0u16; 16];
                    let mut lc = cc;
                    if is_upper { lc += 0x20; }
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
                    for j in 0..16 {
                        token.mask[j] |= if is_upper { !m[j] } else { m[j] };
                    }
                    token.kind = REMIMU_KIND_NORMAL;
                    state = STATE_QUANT;
                } else if cc == b'b' {
                    token.kind = REMIMU_KIND_BOUND;
                    state = STATE_NORMAL;
                } else if cc == b'B' {
                    token.kind = REMIMU_KIND_NBOUND;
                    state = STATE_NORMAL;
                } else {
                    return Err(-1);
                }
                i += 1;
                continue;
            } else {
                token.push_to_vec(tokens, max_len)?;
                if cc == b'\\' {
                    esc_state = 1;
                } else if cc == b'[' {
                    state = STATE_CC_INIT;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if pat(i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if cc == b'(' {
                    paren_count += 1;
                    state = STATE_NORMAL;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if pat(i + 1) == b'?' && pat(i + 2) == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if pat(i + 1) == b'?' && pat(i + 2) == b'>' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.push_to_vec(tokens, max_len)?;
                        state = STATE_NORMAL;
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.count_lo = 1;
                        token.count_hi = 2;
                        i += 2;
                    }
                } else if cc == b')' {
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
                        token.push_to_vec(tokens, max_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        tokens[(found - 1) as usize].pair_offset = (diff as i16) + 2;
                    }
                } else if cc == b'?' || cc == b'+' || cc == b'*' || cc == b'{' {
                    return Err(-1);
                } else if cc == b'.' {
                    for j in 0..16 {
                        token.mask[j] = 0xFFFF;
                    }
                    if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
                        token.mask[1] ^= 0x04;
                        token.mask[1] ^= 0x20;
                    }
                    state = STATE_QUANT;
                } else if cc == b'^' {
                    token.kind = REMIMU_KIND_CARET;
                    state = STATE_NORMAL;
                } else if cc == b'$' {
                    token.kind = REMIMU_KIND_DOLLAR;
                    state = STATE_NORMAL;
                } else if cc == b'|' {
                    token.kind = REMIMU_KIND_OR;
                    state = STATE_NORMAL;
                } else {
                    token.set_mask(cc);
                    state = STATE_QUANT;
                }
                i += 1;
                continue;
            }
        } else if state == STATE_CC_INIT || state == STATE_CC_NORMAL || state == STATE_CC_RANGE {
            if cc == b'\\' && esc_state == 0 {
                esc_state = 1;
                i += 1;
                continue;
            }
            let mut esc_c: u8 = 0;
            let mut c_local = cc;
            if esc_state == 1 {
                esc_state = 0;
                if cc == b'n' { esc_c = b'\n'; }
                else if cc == b'r' { esc_c = b'\r'; }
                else if cc == b't' { esc_c = b'\t'; }
                else if cc == b'v' { esc_c = 0x0B; }
                else if cc == b'f' { esc_c = 0x0C; }
                else if cc == b'x' {
                    if pat(i + 1) == 0 || pat(i + 2) == 0 {
                        return Err(-1);
                    }
                    let mut n0 = pat(i + 1);
                    let mut n1 = pat(i + 1);
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
                }
                else if cc == b'{' || cc == b'}' || cc == b'[' || cc == b']' || cc == b'-'
                    || cc == b'(' || cc == b')' || cc == b'|' || cc == b'^' || cc == b'$'
                    || cc == b'*' || cc == b'+' || cc == b'?' || cc == b':' || cc == b'.'
                    || cc == b'/' || cc == b'\\' {
                    esc_c = cc;
                }
                else if cc == b'd' || cc == b's' || cc == b'w'
                    || cc == b'D' || cc == b'S' || cc == b'W' {
                    if state == STATE_CC_RANGE {
                        return Err(-1);
                    }
                    let is_upper = cc <= b'Z';
                    let mut m = [0u16; 16];
                    let mut lc = cc;
                    if is_upper { lc += 0x20; }
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
                    for j in 0..16 {
                        token.mask[j] |= if is_upper { !m[j] } else { m[j] };
                    }
                    char_class_mem = -1;
                    i += 1;
                    continue;
                }
                else {
                    return Err(-1);
                }
                // c_local stays as raw c when esc_c is set; logic differentiates via esc_c
                let _ = c_local;
            }

            if state == STATE_CC_INIT {
                // C: char_class_mem = c; _REGEX_SET_MASK(c);
                // It uses raw c (not esc_c) for char_class_mem and mask. But if escaped, we want esc_c.
                // Looking at C closely: it always uses `c` regardless of esc_c. But esc_c was set, c is unchanged.
                // The C code at this point uses raw 'c' to set the mask, even after escape processing. But for an esc
                // path, esc_c holds the actual byte. This appears to be a bug in the original C? Let me re-read...
                // Actually, looking more carefully, after the esc_state==1 block, both `c` and `esc_c` are set.
                // The code uses `c` to set the mask - which is the literal escape character, not the escaped value.
                // For instance, \n would have c='n' and esc_c='\n'. The C code uses 'c'='n' as the mask.
                // Hmm, that seems wrong. Let me check the original C more carefully.

                // In the C code lines 638-642:
                //     if (state == STATE_CC_INIT) {
                //         char_class_mem = c;
                //         _REGEX_SET_MASK(c);
                //         state = STATE_CC_NORMAL;
                //     }
                // It uses 'c' here. If esc_c is non-zero, we should use esc_c instead. Let me look at lines 644+:
                //     else if (state == STATE_CC_NORMAL) {
                //         if (c == ']' && esc_c == 0) {...}
                //         else if (c == '-' && esc_c == 0 && char_class_mem >= 0) {...}
                //         else { char_class_mem = c; _REGEX_SET_MASK(c); ...}
                // The else branch uses 'c'. But this looks wrong for escape sequences too...
                //
                // Actually wait, this _is_ a bug or quirk in the C code. But since we're translating exactly,
                // let me match it. Actually, looking again - if the user writes \n in a class, they'd want \n
                // to match. But the C code uses raw 'n'. Hmm, this is probably a real bug in the C code we
                // should faithfully reproduce since we're matching behavior exactly.
                //
                // Actually wait, I should re-examine. In the escape block, it sets esc_c. Then we fall through
                // to the state check. For STATE_CC_INIT, it uses `c`. But the C code also has a bunch of
                // `else if` clauses for esc_c handling. Let me re-read more carefully.
                //
                // OK looking more carefully at lines 555-637, I see:
                // - At line 555: uint8_t esc_c = 0;
                // - Then if esc_state == 1, it sets esc_c based on c. The variable c is unchanged.
                // - For 'd','s','w' etc, it modifies token.mask directly and `continue`s, so we skip below.
                // - For other escapes, esc_c is set but c stays as the escape char.
                //
                // Then below at line 638+:
                //   if (state == STATE_CC_INIT) {
                //     char_class_mem = c;  // BUG: uses c not esc_c
                //     _REGEX_SET_MASK(c);  // BUG: uses c not esc_c
                //   }
                //
                // Hmm. OK let me just faithfully reproduce.
                let use_byte = if esc_c != 0 { esc_c } else { cc };
                // Actually I'll match C exactly which uses `c`
                let _ = use_byte;
                char_class_mem = cc as i32;
                token.set_mask(cc);
                state = STATE_CC_NORMAL;
            } else if state == STATE_CC_NORMAL {
                if cc == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    state = STATE_QUANT;
                    i += 1;
                    continue;
                } else if cc == b'-' && esc_c == 0 && char_class_mem >= 0 {
                    state = STATE_CC_RANGE;
                    i += 1;
                    continue;
                } else {
                    char_class_mem = cc as i32;
                    token.set_mask(cc);
                    state = STATE_CC_NORMAL;
                }
            } else if state == STATE_CC_RANGE {
                if cc == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    token.set_mask(b'-');
                    state = STATE_QUANT;
                    i += 1;
                    continue;
                } else {
                    if char_class_mem == -1 {
                        return Err(-1);
                    }
                    if (cc as i32) < char_class_mem {
                        return Err(-1);
                    }
                    let mut idx = cc;
                    while (idx as i32) > char_class_mem {
                        token.set_mask(idx);
                        idx -= 1;
                    }
                    state = STATE_CC_NORMAL;
                    char_class_mem = -1;
                }
            }
            i += 1;
            continue;
        } else {
            unreachable!();
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

    // add invisible non-capturing group close
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, max_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len)?;

    let k = tokens.len() as i16;
    tokens[0].pair_offset = k - 2;
    tokens[(k - 2) as usize].pair_offset = -(k - 2);

    *token_count = k;

    // copy quantifiers from )s to (s; smuggle group index into mask field
    let mut n: u32 = 0;
    let mut k2: i16 = 0;
    while k2 < k {
        let kind = tokens[k2 as usize].kind;
        if kind == REMIMU_KIND_CLOSE {
            tokens[k2 as usize].mask[0] = n as u16;
            n += 1;
            let k3 = (k2 + tokens[k2 as usize].pair_offset) as i16;
            let count_lo = tokens[k2 as usize].count_lo;
            let count_hi = tokens[k2 as usize].count_hi;
            let mode = tokens[k2 as usize].mode;
            tokens[k3 as usize].count_lo = count_lo;
            tokens[k3 as usize].count_hi = count_hi;
            tokens[k3 as usize].mask[0] = n as u16;
            tokens[k3 as usize].mode = mode;
            n += 1;
            if n > 1024 {
                return Err(-1);
            }
        } else if kind == REMIMU_KIND_OR || kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l: i64 = (k2 as i64) + 1;
            while l < tokens_len {
                if (l as usize) >= tokens.len() {
                    break;
                }
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
            if tokens[k2 as usize].kind == REMIMU_KIND_OR {
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

fn check_mask_idx(tokens: &[RegexToken], k: usize, byte: u8) -> bool {
    let idx = (byte >> 4) as usize;
    let bit = 1u16 << (byte & 0xF);
    (tokens[k].mask[idx] & bit) != 0
}

fn check_is_w(byte: u8) -> bool {
    let mut w_mask = [0u64; 16];
    w_mask[3] = 0x03FF;
    w_mask[4] = 0xFFFE;
    w_mask[5] = 0x87FF;
    w_mask[6] = 0xFFFE;
    w_mask[7] = 0x07FF;
    let idx = (byte >> 4) as usize;
    let bit = 1u64 << (byte & 0xF);
    (w_mask[idx] & bit) != 0
}

pub fn regex_match(tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64]) -> Option<usize> {
    let text_bytes = text.as_bytes();
    // text byte at index, 0 if at/past end (mimics null terminator)
    let txt = |idx: usize| -> u8 {
        if idx < text_bytes.len() { text_bytes[idx] } else { 0 }
    };

    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;

    let mut cap_slots = cap_slots as usize;
    if cap_slots > AUX_STATS_SIZE {
        cap_slots = AUX_STATS_SIZE;
    }

    let mut q_group_accepts_zero = vec![0u8; AUX_STATS_SIZE];
    let mut q_group_state = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_stack = vec![0u32; AUX_STATS_SIZE];
    let mut q_group_cap_index = vec![0xFFFFu16; AUX_STATS_SIZE];

    let mut k: u32 = 0;
    let mut caps: u16 = 0;

    while tokens[k as usize].kind != REMIMU_KIND_END {
        if tokens[k as usize].kind == REMIMU_KIND_OPEN && (caps as usize) < cap_slots {
            let m0 = tokens[k as usize].mask[0] as usize;
            let pair_k = (k as i64 + tokens[k as usize].pair_offset as i64) as usize;
            let m_pair = tokens[pair_k].mask[0] as usize;
            q_group_cap_index[m0] = caps;
            q_group_cap_index[m_pair] = caps;
            cap_pos[caps as usize] = -1;
            cap_span[caps as usize] = -1;
            caps += 1;
        }
        k += 1;
        let kk = tokens[k as usize].kind;
        if kk == REMIMU_KIND_CLOSE || kk == REMIMU_KIND_OPEN || kk == REMIMU_KIND_NCOPEN {
            let m = tokens[k as usize].mask[0] as usize;
            if m >= AUX_STATS_SIZE {
                return None;
            }
            q_group_state[m] = 0;
            q_group_stack[m] = 0;
            q_group_accepts_zero[m] = 0;
        }
    }

    let tokens_len = k as u64;

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(STACK_SIZE_MAX);
    // Pre-fill with default states
    for _ in 0..STACK_SIZE_MAX {
        rewind_stack.push(RegexMatcherState {
            k: 0, group_state: 0, prev: 0, i: 0, range_min: 0, range_max: 0,
        });
    }
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: u8 = 0;

    k = 0;
    let mut iteration_loop = true;
    while iteration_loop && (k as u64) < tokens_len {
        let kind = tokens[k as usize].kind;

        if kind == REMIMU_KIND_CARET {
            if i != 0 {
                // rewind or abort
                if stack_n == 0 { return None; }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                just_rewinded = 1;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let m = tokens[k as usize].mask[0] as usize;
                    q_group_state[m] = rewind_stack[stack_n].group_state;
                    q_group_stack[m] = rewind_stack[stack_n].prev;
                }
                if k > 0 { k -= 1; } else { k = u32::MAX; }
            }
            // otherwise continue
        } else if kind == REMIMU_KIND_DOLLAR {
            if txt(i as usize) != 0 {
                if stack_n == 0 { return None; }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                just_rewinded = 1;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let m = tokens[k as usize].mask[0] as usize;
                    q_group_state[m] = rewind_stack[stack_n].group_state;
                    q_group_stack[m] = rewind_stack[stack_n].prev;
                }
                if k > 0 { k -= 1; } else { k = u32::MAX; }
            }
        } else if kind == REMIMU_KIND_BOUND {
            let cur = txt(i as usize);
            let do_rewind = if i == 0 && !check_is_w(cur) {
                true
            } else if i != 0 && cur == 0 && !check_is_w(txt((i - 1) as usize)) {
                true
            } else if i != 0 && cur != 0 && check_is_w(txt((i - 1) as usize)) == check_is_w(cur) {
                true
            } else {
                false
            };
            if do_rewind {
                if stack_n == 0 { return None; }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                just_rewinded = 1;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let m = tokens[k as usize].mask[0] as usize;
                    q_group_state[m] = rewind_stack[stack_n].group_state;
                    q_group_stack[m] = rewind_stack[stack_n].prev;
                }
                if k > 0 { k -= 1; } else { k = u32::MAX; }
            }
        } else if kind == REMIMU_KIND_NBOUND {
            let cur = txt(i as usize);
            let do_rewind = if i == 0 && check_is_w(cur) {
                true
            } else if i != 0 && cur == 0 && check_is_w(txt((i - 1) as usize)) {
                true
            } else if i != 0 && cur != 0 && check_is_w(txt((i - 1) as usize)) != check_is_w(cur) {
                true
            } else {
                false
            };
            if do_rewind {
                if stack_n == 0 { return None; }
                stack_n -= 1;
                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                just_rewinded = 1;
                range_min = rewind_stack[stack_n].range_min;
                range_max = rewind_stack[stack_n].range_max;
                i = rewind_stack[stack_n].i;
                k = rewind_stack[stack_n].k;
                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                    let m = tokens[k as usize].mask[0] as usize;
                    q_group_state[m] = rewind_stack[stack_n].group_state;
                    q_group_stack[m] = rewind_stack[stack_n].prev;
                }
                if k > 0 { k -= 1; } else { k = u32::MAX; }
            }
        } else {
            // unmatchable token
            if tokens[k as usize].count_hi == 1 {
                if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                    k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                } else {
                    k = k.wrapping_add(1);
                }
                k = k.wrapping_add(1);
                continue;
            }

            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                if just_rewinded == 0 {
                    let pair_k = (k as i64 + tokens[k as usize].pair_offset as i64) as usize;
                    let pair_m0 = tokens[pair_k].mask[0] as usize;
                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k as usize].count_lo == 0 || q_group_accepts_zero[pair_m0] != 0) {
                        range_min = 0;
                        range_max = 0;
                        // save
                        if stack_n >= STACK_SIZE_MAX { return None; }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                        k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                    } else {
                        range_min = 1;
                        range_max = 0;
                        if stack_n >= STACK_SIZE_MAX { return None; }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                } else {
                    just_rewinded = 0;
                    let orig_k = k as i64;
                    let mut should_continue = false;
                    if range_min != 0 {
                        k = (k as u64 + range_min) as u32;
                        let prev_kind = tokens[(k - 1) as usize].kind;
                        if prev_kind == REMIMU_KIND_OR {
                            k = (k as i64 + tokens[(k - 1) as usize].pair_offset as i64 - 1) as u32;
                        } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN {
                            k = (k as i64 + tokens[(k - 1) as usize].mask[15] as i64 - 1) as u32;
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_END {
                            return None; // -3 invalid; we return None
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[k as usize].mask[0] as usize;
                            if tokens[k as usize].count_lo == 0 || q_group_accepts_zero[m] != 0 {
                                q_group_state[m] = 0;
                                if (tokens[k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[m] = 0;
                                }
                                should_continue = true;
                            } else {
                                if stack_n == 0 { return None; }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                                just_rewinded = 1;
                                range_min = rewind_stack[stack_n].range_min;
                                range_max = rewind_stack[stack_n].range_max;
                                i = rewind_stack[stack_n].i;
                                k = rewind_stack[stack_n].k;
                                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                    let m2 = tokens[k as usize].mask[0] as usize;
                                    q_group_state[m2] = rewind_stack[stack_n].group_state;
                                    q_group_stack[m2] = rewind_stack[stack_n].prev;
                                }
                                if k > 0 { k -= 1; } else { k = u32::MAX; }
                                should_continue = true;
                            }
                        }
                    }
                    if !should_continue {
                        let k_diff = (k as i64) - orig_k;
                        range_min = (k_diff + 1) as u64;
                        let new_k = (k as i64 - k_diff) as u32;
                        // save
                        if stack_n >= STACK_SIZE_MAX { return None; }
                        let mut s = RegexMatcherState::new(new_k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        if tokens[new_k as usize].kind == REMIMU_KIND_CLOSE {
                            let m = tokens[new_k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                }
            } else if kind == REMIMU_KIND_CLOSE {
                // unquantified
                if tokens[k as usize].count_lo == 1 && tokens[k as usize].count_hi == 2 {
                    let m = tokens[k as usize].mask[0] as usize;
                    let cap_index = q_group_cap_index[m];
                    if cap_index != 0xFFFF {
                        // save dummy
                        if stack_n >= STACK_SIZE_MAX { return None; }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0xFAC7;
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    }
                } else {
                    let m = tokens[k as usize].mask[0] as usize;
                    let pair_k = (k as i64 + tokens[k as usize].pair_offset as i64) as usize;
                    let pair_m = tokens[pair_k].mask[0] as usize;

                    if just_rewinded == 0 {
                        let prev = q_group_stack[m];
                        range_max = tokens[k as usize].count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[m] != 0 { 0 } else { tokens[k as usize].count_lo as u64 };

                        if (q_group_state[m] as u64) + 1 < range_min {
                            q_group_state[m] += 1;
                            // save
                            if stack_n >= STACK_SIZE_MAX { return None; }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;

                            k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                            if k > 0 { k -= 1; } else { k = u32::MAX; }
                            k = k.wrapping_add(1);
                            continue;
                        } else if tokens[k as usize].count_hi != 0 && (q_group_state[m] as u64) + 1 > range_max {
                            range_max = range_max.wrapping_sub(1);
                            // rewind or abort
                            if stack_n == 0 { return None; }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                            just_rewinded = 1;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let m2 = tokens[k as usize].mask[0] as usize;
                                q_group_state[m2] = rewind_stack[stack_n].group_state;
                                q_group_stack[m2] = rewind_stack[stack_n].prev;
                            }
                            if k > 0 { k -= 1; } else { k = u32::MAX; }
                            k = k.wrapping_add(1);
                            continue;
                        }

                        // detect zero-length matches
                        let mut force_zero = false;
                        if prev != 0 && (rewind_stack[prev as usize].i as u32) > (i as u32) {
                            let mut nn = stack_n.wrapping_sub(1);
                            let target_k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                            while nn > 0 && rewind_stack[nn].k != target_k {
                                nn = nn.wrapping_sub(1);
                            }
                            if nn > 0 && rewind_stack[nn].i == i {
                                force_zero = true;
                            }
                        }

                        if force_zero || (prev != 0 && (rewind_stack[prev as usize].i as u32) == (i as u32)) {
                            q_group_accepts_zero[m] = 1;
                            // rewind or abort
                            if stack_n == 0 { return None; }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                            just_rewinded = 1;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let m2 = tokens[k as usize].mask[0] as usize;
                                q_group_state[m2] = rewind_stack[stack_n].group_state;
                                q_group_stack[m2] = rewind_stack[stack_n].prev;
                            }
                            if k > 0 { k -= 1; } else { k = u32::MAX; }
                        } else if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                            q_group_state[m] += 1;
                            // save
                            if stack_n >= STACK_SIZE_MAX { return None; }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            s.group_state = q_group_state[m];
                            s.prev = q_group_stack[m];
                            q_group_stack[m] = stack_n as u32;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                            q_group_state[m] = 0;
                        } else {
                            // greedy
                            if (tokens[k as usize].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let mut k2 = k;
                                if q_group_state[m] == 0 {
                                    k2 = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                                }
                                if stack_n == 0 { return None; }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].k != k2 {
                                    stack_n -= 1;
                                }
                                if stack_n == 0 { return None; }
                            }
                            if (q_group_state[pair_m] as u32) < (i as u32) {
                                q_group_state[m] += 1;
                                // save
                                if stack_n >= STACK_SIZE_MAX { return None; }
                                let mut s = RegexMatcherState::new(k, i);
                                s.range_min = range_min;
                                s.range_max = range_max;
                                s.prev = 0;
                                s.group_state = q_group_state[m];
                                s.prev = q_group_stack[m];
                                q_group_stack[m] = stack_n as u32;
                                rewind_stack[stack_n] = s;
                                stack_n += 1;
                                k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                                if k > 0 { k -= 1; } else { k = u32::MAX; }
                            }
                        }
                    } else {
                        just_rewinded = 0;
                        if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                            // save dummy
                            if stack_n >= STACK_SIZE_MAX { return None; }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0xFAC7;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                            q_group_stack[m] = stack_n as u32;
                            k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                            if k > 0 { k -= 1; } else { k = u32::MAX; }
                        } else {
                            if (q_group_state[m] as u64) < range_min && q_group_accepts_zero[m] == 0 {
                                if stack_n == 0 { return None; }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                                just_rewinded = 1;
                                range_min = rewind_stack[stack_n].range_min;
                                range_max = rewind_stack[stack_n].range_max;
                                i = rewind_stack[stack_n].i;
                                k = rewind_stack[stack_n].k;
                                if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                    let m2 = tokens[k as usize].mask[0] as usize;
                                    q_group_state[m2] = rewind_stack[stack_n].group_state;
                                    q_group_stack[m2] = rewind_stack[stack_n].prev;
                                }
                                if k > 0 { k -= 1; } else { k = u32::MAX; }
                            } else {
                                q_group_state[m] = 0;
                                let cap_index = q_group_cap_index[m];
                                if cap_index != 0xFFFF {
                                    if stack_n >= STACK_SIZE_MAX { return None; }
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
            } else if kind == REMIMU_KIND_OR {
                k = (k as i64 + tokens[k as usize].pair_offset as i64) as u32;
                if k > 0 { k -= 1; } else { k = u32::MAX; }
            } else if kind == REMIMU_KIND_NORMAL {
                if just_rewinded == 0 {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k as usize].count_lo as u64 && txt(i as usize) != 0 && check_mask_idx(tokens, k as usize, txt(i as usize)) {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k as usize].count_lo as u64 {
                        i = old_i;
                        // rewind or abort
                        if stack_n == 0 { return None; }
                        stack_n -= 1;
                        while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                        just_rewinded = 1;
                        range_min = rewind_stack[stack_n].range_min;
                        range_max = rewind_stack[stack_n].range_max;
                        i = rewind_stack[stack_n].i;
                        k = rewind_stack[stack_n].k;
                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let m2 = tokens[k as usize].mask[0] as usize;
                            q_group_state[m2] = rewind_stack[stack_n].group_state;
                            q_group_stack[m2] = rewind_stack[stack_n].prev;
                        }
                        if k > 0 { k -= 1; } else { k = u32::MAX; }
                        k = k.wrapping_add(1);
                        continue;
                    }
                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = (tokens[k as usize].count_hi as u64).wrapping_sub(1);
                        // save
                        if stack_n >= STACK_SIZE_MAX { return None; }
                        let mut s = RegexMatcherState::new(k, i);
                        s.range_min = range_min;
                        s.range_max = range_max;
                        s.prev = 0;
                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let m2 = tokens[k as usize].mask[0] as usize;
                            s.group_state = q_group_state[m2];
                            s.prev = q_group_stack[m2];
                            q_group_stack[m2] = stack_n as u32;
                        }
                        rewind_stack[stack_n] = s;
                        stack_n += 1;
                    } else {
                        let mut limit = tokens[k as usize].count_hi as u64;
                        if limit == 0 {
                            limit = !limit;
                        }
                        range_min = n;
                        while txt(i as usize) != 0 && check_mask_idx(tokens, k as usize, txt(i as usize)) && n + 1 < limit {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k as usize].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            // save
                            if stack_n >= STACK_SIZE_MAX { return None; }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let m2 = tokens[k as usize].mask[0] as usize;
                                s.group_state = q_group_state[m2];
                                s.prev = q_group_stack[m2];
                                q_group_stack[m2] = stack_n as u32;
                            }
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        }
                    }
                } else {
                    just_rewinded = 0;
                    if (tokens[k as usize].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit = range_max;
                        if limit == 0 {
                            limit = !limit;
                        }
                        if check_mask_idx(tokens, k as usize, txt(i as usize)) && txt(i as usize) != 0 && range_min < limit {
                            i += 1;
                            range_min += 1;
                            // save
                            if stack_n >= STACK_SIZE_MAX { return None; }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        } else {
                            if stack_n == 0 { return None; }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                            just_rewinded = 1;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let m2 = tokens[k as usize].mask[0] as usize;
                                q_group_state[m2] = rewind_stack[stack_n].group_state;
                                q_group_stack[m2] = rewind_stack[stack_n].prev;
                            }
                            if k > 0 { k -= 1; } else { k = u32::MAX; }
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            // save
                            if stack_n >= STACK_SIZE_MAX { return None; }
                            let mut s = RegexMatcherState::new(k, i);
                            s.range_min = range_min;
                            s.range_max = range_max;
                            s.prev = 0;
                            rewind_stack[stack_n] = s;
                            stack_n += 1;
                        } else {
                            if stack_n == 0 { return None; }
                            stack_n -= 1;
                            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
                            just_rewinded = 1;
                            range_min = rewind_stack[stack_n].range_min;
                            range_max = rewind_stack[stack_n].range_max;
                            i = rewind_stack[stack_n].i;
                            k = rewind_stack[stack_n].k;
                            if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                                let m2 = tokens[k as usize].mask[0] as usize;
                                q_group_state[m2] = rewind_stack[stack_n].group_state;
                                q_group_stack[m2] = rewind_stack[stack_n].prev;
                            }
                            if k > 0 { k -= 1; } else { k = u32::MAX; }
                        }
                    }
                }
            } else {
                return None;
            }
        }
        k = k.wrapping_add(1);
        if k as u64 >= tokens_len {
            iteration_loop = false;
        }
    }
    let _ = State::Normal;
    let _ = State::Quant;
    let _ = State::Mode;
    let _ = State::CharClassInit;
    let _ = State::CharClassNormal;
    let _ = State::CharClassRange;

    if caps != 0 {
        for n in 0..stack_n {
            let s_k = rewind_stack[n].k;
            let s_i = rewind_stack[n].i;
            let kind = tokens[s_k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let m = tokens[s_k as usize].mask[0] as usize;
                let cap_index = q_group_cap_index[m];
                if cap_index == 0xFFFF { continue; }
                let ci = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[ci] = s_i as i64;
                } else if cap_pos[ci] >= 0 {
                    cap_span[ci] = (s_i as i64) - cap_pos[ci];
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
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR",
        "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];
    let mut k: usize = 0;
    loop {
        let kind = tokens[k].kind as usize;
        let mode = tokens[k].mode as usize;
        let kind_str = if kind < kind_to_str.len() { kind_to_str[kind] } else { "?" };
        let mode_str = if mode < mode_to_str.len() { mode_to_str[mode] } else { "?" };
        print!("{}\t{}\t", kind_str, mode_str);

        let mut c_old: i32 = -1;
        let limit = if tokens[k].kind != 0 { 0 } else { 256 };
        let print_c_smart = |c: i32| {
            if c >= 0x20 && c <= 0x7E {
                print!("{}", c as u8 as char);
            } else {
                print!("\\x{:02x}", c);
            }
        };
        for c in 0..limit {
            if check_mask_idx(tokens, k, c as u8) {
                if c_old == -1 {
                    c_old = c;
                }
            } else if c_old != -1 {
                if c - 1 == c_old {
                    print_c_smart(c_old);
                    c_old = -1;
                } else if c - 2 == c_old {
                    print_c_smart(c_old);
                    print_c_smart(c_old + 1);
                    c_old = -1;
                } else {
                    print_c_smart(c_old);
                    print!("-");
                    print_c_smart(c - 1);
                    c_old = -1;
                }
            }
        }

        println!("\t{{{},{}}}\t({})", tokens[k].count_lo, tokens[k].count_hi.wrapping_sub(1), tokens[k].pair_offset);

        if tokens[k].kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}
