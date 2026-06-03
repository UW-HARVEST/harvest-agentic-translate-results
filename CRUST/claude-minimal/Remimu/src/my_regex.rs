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
        // Skip duplicate consecutive bound/nbound tokens
        if k > 0
            && tokens[k - 1].kind == self.kind
            && (self.kind == REMIMU_KIND_BOUND || self.kind == REMIMU_KIND_NBOUND)
        {
            // reset token without pushing
            *self = RegexToken::default();
            self.count_lo = 1;
            self.count_hi = 2;
            return Ok(());
        }
        if (self.mode & REMIMU_MODE_INVERTED) != 0 {
            self.invert_mask();
        }
        if k >= max_len {
            return Err(-2);
        }
        tokens.push(*self);
        *self = RegexToken::default();
        self.count_lo = 1;
        self.count_hi = 2;
        Ok(())
    }
}

impl Default for RegexToken {
    fn default() -> Self {
        let mut t = Self::new(REMIMU_KIND_NORMAL, 0);
        t.count_lo = 0;
        t.count_hi = 0;
        t
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Normal,
    Quant,
    Mode,
    CharClassInit,
    CharClassNormal,
    CharClassRange,
}

fn is_w(byte: u8) -> bool {
    let mut w_mask = [0u16; 16];
    w_mask[3] = 0x03FF;
    w_mask[4] = 0xFFFE;
    w_mask[5] = 0x87FF;
    w_mask[6] = 0xFFFE;
    w_mask[7] = 0x07FF;
    (w_mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
}

fn make_token() -> RegexToken {
    RegexToken {
        kind: 0,
        mode: 0,
        count_lo: 1,
        count_hi: 2,
        mask: [0; 16],
        pair_offset: 0,
    }
}

fn set_mask(token: &mut RegexToken, byte: u8) {
    token.mask[(byte >> 4) as usize] |= 1u16 << (byte & 0xF);
}

fn set_mask_all(token: &mut RegexToken) {
    for n in 0..16 {
        token.mask[n] = 0xFFFF;
    }
}

fn do_invert(token: &mut RegexToken) {
    for n in 0..16 {
        token.mask[n] = !token.mask[n];
    }
    token.mode &= !REMIMU_MODE_INVERTED;
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

    if tokens_len <= 0 {
        return Err(-2);
    }

    // tokens is intended to be a buffer; use it as such
    // Convert to in-place buffer behavior:
    // We expect tokens to be pre-allocated with at least tokens_len elements (per the test).
    // We'll write to tokens[0..k] and finally truncate/leave size as-is, but report k via token_count.

    let mut esc_state = 0;
    let mut state = State::Normal;
    let mut char_class_mem: i32 = -1;

    let mut token = make_token();

    // helper closures aren't super easy due to borrowing; do everything inline.

    let mut k: i16 = 0;

    // start with an invisible group specifier
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;

    // ensure tokens has space; if user passed a Vec, we'll grow as needed but bound by tokens_len
    // Since the test passes vec![Default; 1024], we treat it as a buffer.
    let buffer_capacity = tokens.len();
    if buffer_capacity < tokens_len as usize {
        tokens.resize(tokens_len as usize, RegexToken::default());
    }

    // Macro-like helper: push token
    macro_rules! push_token {
        ($tokens:expr, $token:expr, $k:expr) => {{
            let do_push = $k == 0
                || $tokens[($k - 1) as usize].kind != $token.kind
                || ($token.kind != REMIMU_KIND_BOUND && $token.kind != REMIMU_KIND_NBOUND);
            if do_push {
                if ($token.mode & REMIMU_MODE_INVERTED) != 0 {
                    do_invert(&mut $token);
                }
                if $k as i64 >= tokens_len {
                    return Err(-2);
                }
                $tokens[$k as usize] = $token;
                $k += 1;
                $token = make_token();
                $token.count_lo = 1;
                $token.count_hi = 2;
            }
        }};
    }

    let mut i: usize = 0;
    while i < pattern_len {
        let c = pattern_bytes[i];

        if state == State::Quant {
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
                let next = if i + 1 < pattern_len {
                    pattern_bytes[i + 1]
                } else {
                    0
                };
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
                            while i < pattern_len
                                && pattern_bytes[i] >= b'0'
                                && pattern_bytes[i] <= b'9'
                            {
                                val2 = val2
                                    .wrapping_mul(10)
                                    .wrapping_add((pattern_bytes[i] - b'0') as u32);
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

        if state == State::Mode {
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

        if state == State::Normal {
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' {
                    set_mask(&mut token, b'\n');
                    state = State::Quant;
                } else if c == b'r' {
                    set_mask(&mut token, b'\r');
                    state = State::Quant;
                } else if c == b't' {
                    set_mask(&mut token, b'\t');
                    state = State::Quant;
                } else if c == 0x0B {
                    set_mask(&mut token, 0x0B);
                    state = State::Quant;
                } else if c == b'v' {
                    set_mask(&mut token, 0x0B);
                    state = State::Quant;
                } else if c == b'f' {
                    set_mask(&mut token, 0x0C);
                    state = State::Quant;
                } else if c == b'x' {
                    if i + 2 >= pattern_len {
                        return Err(-1);
                    }
                    let mut n0 = pattern_bytes[i + 1];
                    let mut n1 = pattern_bytes[i + 1];
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
                    n0 = n0.wrapping_sub(b'0');
                    n1 = n1.wrapping_sub(b'0');
                    set_mask(&mut token, (n1 << 4) | n0);
                    i += 2;
                    state = State::Quant;
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
                    set_mask(&mut token, c);
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
                    for n in 0..16 {
                        token.mask[n] |= if is_upper { !m[n] } else { m[n] };
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
                i += 1;
                continue;
            } else {
                push_token!(tokens, token, k);
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
                    if i + 2 < pattern_len
                        && pattern_bytes[i + 1] == b'?'
                        && pattern_bytes[i + 2] == b':'
                    {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if i + 2 < pattern_len
                        && pattern_bytes[i + 1] == b'?'
                        && pattern_bytes[i + 2] == b'>'
                    {
                        token.kind = REMIMU_KIND_NCOPEN;
                        push_token!(tokens, token, k);

                        state = State::Normal;
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.count_lo = 1;
                        token.count_hi = 2;

                        i += 2;
                    }
                } else if c == b')' {
                    paren_count -= 1;
                    if paren_count < 0 || k == 0 {
                        return Err(-1);
                    }
                    token.kind = REMIMU_KIND_CLOSE;
                    state = State::Quant;

                    let mut balance = 0i32;
                    let mut found: i64 = -1;
                    let mut l = (k as i64) - 1;
                    while l >= 0 {
                        if tokens[l as usize].kind == REMIMU_KIND_NCOPEN
                            || tokens[l as usize].kind == REMIMU_KIND_OPEN
                        {
                            if balance == 0 {
                                found = l;
                                break;
                            } else {
                                balance -= 1;
                            }
                        } else if tokens[l as usize].kind == REMIMU_KIND_CLOSE {
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
                        push_token!(tokens, token, k);
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        tokens[(found - 1) as usize].pair_offset = (diff + 2) as i16;
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
                    set_mask(&mut token, c);
                    state = State::Quant;
                }
                i += 1;
                continue;
            }
        } else if state == State::CharClassInit
            || state == State::CharClassNormal
            || state == State::CharClassRange
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
                    if i + 2 >= pattern_len {
                        return Err(-1);
                    }
                    let mut n0 = pattern_bytes[i + 1];
                    let mut n1 = pattern_bytes[i + 1];
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
                    n0 = n0.wrapping_sub(b'0');
                    n1 = n1.wrapping_sub(b'0');
                    esc_c = (n1 << 4) | n0;
                    i += 2;
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
                    if state == State::CharClassRange {
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
            if state == State::CharClassInit {
                char_class_mem = c as i32;
                set_mask(&mut token, c);
                state = State::CharClassNormal;
            } else if state == State::CharClassNormal {
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
                    set_mask(&mut token, c);
                    state = State::CharClassNormal;
                }
            } else if state == State::CharClassRange {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    set_mask(&mut token, b'-');
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
                    let mut z = c;
                    while (z as i32) > char_class_mem {
                        set_mask(&mut token, z);
                        z = z.wrapping_sub(1);
                    }
                    state = State::CharClassNormal;
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
    if state == State::CharClassInit
        || state == State::CharClassNormal
        || state == State::CharClassRange
    {
        return Err(-1);
    }

    push_token!(tokens, token, k);

    // close invisible group
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    push_token!(tokens, token, k);

    // end token
    token.kind = REMIMU_KIND_END;
    push_token!(tokens, token, k);

    tokens[0].pair_offset = (k - 2) as i16;
    tokens[(k - 2) as usize].pair_offset = -((k - 2) as i16);

    *token_count = k;

    let mut n: u64 = 0;
    let mut k2: i16 = 0;
    while k2 < k {
        if tokens[k2 as usize].kind == REMIMU_KIND_CLOSE {
            tokens[k2 as usize].mask[0] = n as u16;
            n += 1;
            let k3 = (k2 as i32) + (tokens[k2 as usize].pair_offset as i32);
            tokens[k3 as usize].count_lo = tokens[k2 as usize].count_lo;
            tokens[k3 as usize].count_hi = tokens[k2 as usize].count_hi;
            tokens[k3 as usize].mask[0] = n as u16;
            tokens[k3 as usize].mode = tokens[k2 as usize].mode;
            n += 1;
            if n > 1024 {
                return Err(-1);
            }
        } else if tokens[k2 as usize].kind == REMIMU_KIND_OR
            || tokens[k2 as usize].kind == REMIMU_KIND_OPEN
            || tokens[k2 as usize].kind == REMIMU_KIND_NCOPEN
        {
            let mut balance = 0i32;
            let mut found: i64 = -1;
            let mut l = (k2 as i64) + 1;
            while l < tokens_len {
                let kind = tokens[l as usize].kind;
                if kind == REMIMU_KIND_OR && balance == 0 {
                    found = l;
                    break;
                } else if kind == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = l;
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

fn check_mask(tokens: &[RegexToken], k: usize, byte: u8) -> bool {
    (tokens[k].mask[(byte >> 4) as usize] & (1u16 << (byte & 0xF))) != 0
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
    // helper for safely indexing text including a "null terminator"
    let text_byte = |i: usize| -> u8 {
        if i < text_bytes.len() {
            text_bytes[i]
        } else {
            0
        }
    };

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

    let mut tokens_len: usize = 0;
    let mut k: usize = 0;
    let mut caps: usize = 0;

    while tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            q_group_cap_index[tokens[k].mask[0] as usize] = caps as u16;
            let pair_idx = (k as isize + tokens[k].pair_offset as isize) as usize;
            q_group_cap_index[tokens[pair_idx].mask[0] as usize] = caps as u16;
            cap_pos[caps] = -1;
            cap_span[caps] = -1;
            caps += 1;
        }
        k += 1;
        if tokens[k].kind == REMIMU_KIND_CLOSE
            || tokens[k].kind == REMIMU_KIND_OPEN
            || tokens[k].kind == REMIMU_KIND_NCOPEN
        {
            if (tokens[k].mask[0] as usize) >= aux_stats_size {
                return None;
            }
            q_group_state[tokens[k].mask[0] as usize] = 0;
            q_group_stack[tokens[k].mask[0] as usize] = 0;
            q_group_accepts_zero[tokens[k].mask[0] as usize] = 0;
        }
    }

    tokens_len = k;

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(stack_size_max);
    for _ in 0..stack_size_max {
        rewind_stack.push(RegexMatcherState::new(0, 0));
    }
    let mut stack_n: usize = 0;

    let mut i: usize = start_i;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    macro_rules! rewind_save_raw {
        ($K:expr, $ISDUMMY:expr) => {{
            if stack_n >= stack_size_max {
                return None;
            }
            let mut s = RegexMatcherState::new($K as u32, i as u64);
            s.range_min = range_min;
            s.range_max = range_max;
            s.prev = 0;
            if $ISDUMMY {
                s.prev = 0xFAC7;
            } else if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                s.group_state = q_group_state[tokens[s.k as usize].mask[0] as usize];
                s.prev = q_group_stack[tokens[s.k as usize].mask[0] as usize];
                q_group_stack[tokens[s.k as usize].mask[0] as usize] = stack_n as u32;
            }
            rewind_stack[stack_n] = s;
            stack_n += 1;
        }};
    }

    macro_rules! rewind_save {
        ($K:expr) => {
            rewind_save_raw!($K, false)
        };
    }

    macro_rules! rewind_save_dummy {
        ($K:expr) => {
            rewind_save_raw!($K, true)
        };
    }

    macro_rules! rewind_or_abort {
        () => {{
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
            i = rewind_stack[stack_n].i as usize;
            k = rewind_stack[stack_n].k as usize;
            if tokens[k].kind == REMIMU_KIND_CLOSE {
                q_group_state[tokens[k].mask[0] as usize] = rewind_stack[stack_n].group_state;
                q_group_stack[tokens[k].mask[0] as usize] = rewind_stack[stack_n].prev;
            }
            // The for loop will do k+=1; we want the same k, so subtract 1.
            // But we use a while loop, so we need to set a flag or structure.
            // We'll use a label/continue to skip the k+=1.
        }};
    }

    // Convert the for loop into a manual loop where we control k advancement
    k = 0;
    let mut rewinded_from_macro: bool;
    'outer: loop {
        if k >= tokens_len {
            break;
        }
        rewinded_from_macro = false;

        if tokens[k].kind == REMIMU_KIND_CARET {
            if i != 0 {
                rewind_or_abort!();
                rewinded_from_macro = true;
            }
            if !rewinded_from_macro {
                k += 1;
                continue 'outer;
            }
        } else if tokens[k].kind == REMIMU_KIND_DOLLAR {
            if text_byte(i) != 0 {
                rewind_or_abort!();
                rewinded_from_macro = true;
            }
            if !rewinded_from_macro {
                k += 1;
                continue 'outer;
            }
        } else if tokens[k].kind == REMIMU_KIND_BOUND {
            if i == 0 && !is_w(text_byte(i)) {
                rewind_or_abort!();
                rewinded_from_macro = true;
            } else if i != 0 && text_byte(i) == 0 && !is_w(text_byte(i - 1)) {
                rewind_or_abort!();
                rewinded_from_macro = true;
            } else if i != 0
                && text_byte(i) != 0
                && is_w(text_byte(i - 1)) == is_w(text_byte(i))
            {
                rewind_or_abort!();
                rewinded_from_macro = true;
            }
        } else if tokens[k].kind == REMIMU_KIND_NBOUND {
            if i == 0 && is_w(text_byte(i)) {
                rewind_or_abort!();
                rewinded_from_macro = true;
            } else if i != 0 && text_byte(i) == 0 && is_w(text_byte(i - 1)) {
                rewind_or_abort!();
                rewinded_from_macro = true;
            } else if i != 0
                && text_byte(i) != 0
                && is_w(text_byte(i - 1)) != is_w(text_byte(i))
            {
                rewind_or_abort!();
                rewinded_from_macro = true;
            }
        } else {
            // deliberately unmatchable token
            if tokens[k].count_hi == 1 {
                if tokens[k].kind == REMIMU_KIND_OPEN || tokens[k].kind == REMIMU_KIND_NCOPEN {
                    k = (k as isize + tokens[k].pair_offset as isize) as usize;
                } else {
                    k += 1;
                }
                k += 1;
                continue 'outer;
            }

            if tokens[k].kind == REMIMU_KIND_OPEN || tokens[k].kind == REMIMU_KIND_NCOPEN {
                if !just_rewinded {
                    let pair_mask0 = tokens[(k as isize + tokens[k].pair_offset as isize) as usize]
                        .mask[0] as usize;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k].count_lo == 0 || q_group_accepts_zero[pair_mask0] != 0)
                    {
                        range_min = 0;
                        range_max = 0;
                        rewind_save!(k);
                        k = (k as isize + tokens[k].pair_offset as isize) as usize;
                    // outer increment will move past CLOSE
                    } else {
                        range_min = 1;
                        range_max = 0;
                        rewind_save!(k);
                    }
                } else {
                    just_rewinded = false;

                    let orig_k = k;

                    if range_min != 0 {
                        k += range_min as usize;
                        if tokens[k - 1].kind == REMIMU_KIND_OR {
                            k = (k as isize + tokens[k - 1].pair_offset as isize - 1) as usize;
                        } else if tokens[k - 1].kind == REMIMU_KIND_OPEN
                            || tokens[k - 1].kind == REMIMU_KIND_NCOPEN
                        {
                            k = (k as isize + tokens[k - 1].mask[15] as isize - 1) as usize;
                        }

                        if tokens[k].kind == REMIMU_KIND_END {
                            return None;
                        }

                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            if tokens[k].count_lo == 0
                                || q_group_accepts_zero[tokens[k].mask[0] as usize] != 0
                            {
                                q_group_state[tokens[k].mask[0] as usize] = 0;
                                if (tokens[k].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[tokens[k].mask[0] as usize] = 0;
                                }
                                k += 1;
                                continue 'outer;
                            } else {
                                rewind_or_abort!();
                                continue 'outer;
                            }
                        }

                        // assert OR
                    }

                    let k_diff = k as isize - orig_k as isize;
                    range_min = (k_diff + 1) as u64;
                    rewind_save!((k as isize - k_diff) as usize);
                }
            } else if tokens[k].kind == REMIMU_KIND_CLOSE {
                if tokens[k].count_lo == 1 && tokens[k].count_hi == 2 {
                    let cap_index = q_group_cap_index[tokens[k].mask[0] as usize];
                    if cap_index != 0xFFFF {
                        rewind_save_dummy!(k);
                    }
                } else {
                    if !just_rewinded {
                        let prev = q_group_stack[tokens[k].mask[0] as usize];

                        range_max = tokens[k].count_hi as u64;
                        range_max = range_max.wrapping_sub(1);
                        range_min = if q_group_accepts_zero[tokens[k].mask[0] as usize] != 0 {
                            0
                        } else {
                            tokens[k].count_lo as u64
                        };

                        if (q_group_state[tokens[k].mask[0] as usize] as u64 + 1) < range_min {
                            q_group_state[tokens[k].mask[0] as usize] += 1;
                            rewind_save!(k);
                            k = (k as isize + tokens[k].pair_offset as isize) as usize;
                            // need to ensure outer loop hits k next without skipping
                            // by subtracting 1 then letting the +1 happen
                            continue 'outer;
                        } else if tokens[k].count_hi != 0
                            && (q_group_state[tokens[k].mask[0] as usize] as u64 + 1) > range_max
                        {
                            range_max = range_max.wrapping_sub(1);
                            rewind_or_abort!();
                            continue 'outer;
                        }

                        let mut force_zero = false;
                        if prev != 0 && (rewind_stack[prev as usize].i as u32) > (i as u32) {
                            let mut n = stack_n - 1;
                            let target_k = (k as isize + tokens[k].pair_offset as isize) as u32;
                            while n > 0 && rewind_stack[n].k != target_k {
                                n -= 1;
                            }
                            if rewind_stack[n].i as usize == i {
                                force_zero = true;
                            }
                        }

                        if force_zero
                            || (prev != 0 && (rewind_stack[prev as usize].i as u32) == (i as u32))
                        {
                            q_group_accepts_zero[tokens[k].mask[0] as usize] = 1;
                            rewind_or_abort!();
                            continue 'outer;
                        } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            q_group_state[tokens[k].mask[0] as usize] += 1;
                            rewind_save!(k);
                            q_group_state[tokens[k].mask[0] as usize] = 0;
                        } else {
                            if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let mut k2 = k;
                                if q_group_state[tokens[k].mask[0] as usize] == 0 {
                                    k2 = (k as isize + tokens[k].pair_offset as isize) as usize;
                                }
                                if stack_n == 0 {
                                    return None;
                                }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].k as usize != k2 {
                                    stack_n -= 1;
                                }
                                if stack_n == 0 {
                                    return None;
                                }
                            }
                            let pair_idx =
                                (k as isize + tokens[k].pair_offset as isize) as usize;
                            if (q_group_state[tokens[pair_idx].mask[0] as usize] as u32)
                                < (i as u32)
                            {
                                q_group_state[tokens[k].mask[0] as usize] += 1;
                                rewind_save!(k);
                                k = (k as isize + tokens[k].pair_offset as isize) as usize;
                                continue 'outer;
                            }
                        }
                    } else {
                        just_rewinded = false;

                        if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            rewind_save_dummy!(k);
                            q_group_stack[tokens[k].mask[0] as usize] = stack_n as u32;
                            k = (k as isize + tokens[k].pair_offset as isize) as usize;
                            continue 'outer;
                        } else {
                            if (q_group_state[tokens[k].mask[0] as usize] as u64) < range_min
                                && q_group_accepts_zero[tokens[k].mask[0] as usize] == 0
                            {
                                rewind_or_abort!();
                            } else {
                                q_group_state[tokens[k].mask[0] as usize] = 0;
                                let cap_index = q_group_cap_index[tokens[k].mask[0] as usize];
                                if cap_index != 0xFFFF {
                                    rewind_save_dummy!(k);
                                }
                            }
                        }
                    }
                }
            } else if tokens[k].kind == REMIMU_KIND_OR {
                k = (k as isize + tokens[k].pair_offset as isize) as usize;
                k = k.wrapping_sub(1);
            } else if tokens[k].kind == REMIMU_KIND_NORMAL {
                if !just_rewinded {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k].count_lo as u64
                        && text_byte(i) != 0
                        && check_mask(tokens, k, text_byte(i))
                    {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k].count_lo as u64 {
                        i = old_i;
                        rewind_or_abort!();
                        continue 'outer;
                    }

                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = (tokens[k].count_hi as u64).wrapping_sub(1);
                        rewind_save!(k);
                    } else {
                        let mut limit = tokens[k].count_hi as u64;
                        if limit == 0 {
                            limit = !limit;
                        }
                        range_min = n;
                        while text_byte(i) != 0
                            && check_mask(tokens, k, text_byte(i))
                            && n + 1 < limit
                        {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            rewind_save!(k);
                        }
                    }
                } else {
                    just_rewinded = false;

                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit = range_max;
                        if limit == 0 {
                            limit = !limit;
                        }
                        if check_mask(tokens, k, text_byte(i))
                            && text_byte(i) != 0
                            && range_min < limit
                        {
                            i += 1;
                            range_min += 1;
                            rewind_save!(k);
                        } else {
                            rewind_or_abort!();
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            rewind_save!(k);
                        } else {
                            rewind_or_abort!();
                        }
                    }
                }
            } else {
                return None;
            }
        }
        k = k.wrapping_add(1);
    }

    if caps != 0 {
        for n in 0..stack_n {
            let s_k = rewind_stack[n].k as usize;
            let s_i = rewind_stack[n].i as i64;
            let kind = tokens[s_k].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[s_k].mask[0] as usize];
                if cap_index == 0xFFFF {
                    continue;
                }
                let cap_index = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[cap_index] = s_i;
                } else if cap_pos[cap_index] >= 0 {
                    cap_span[cap_index] = s_i - cap_pos[cap_index];
                }
            }
        }
        for n in 0..caps {
            if cap_span[n] == -1 {
                cap_pos[n] = -1;
            }
        }
    }

    Some(i)
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];
    let mut k: usize = 0;
    loop {
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

        let mut c_old: i32 = -1;
        let upper = if tokens[k].kind != 0 { 0 } else { 256 };
        for c in 0..upper {
            let cb = c as u8;
            let in_mask = (tokens[k].mask[(cb >> 4) as usize] & (1u16 << (cb & 0xF))) != 0;
            let print_c_smart = |c: i32| {
                if c >= 0x20 && c <= 0x7E {
                    print!("{}", c as u8 as char);
                } else {
                    print!("\\x{:02x}", c);
                }
            };
            if in_mask {
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

        println!(
            "\t{{{},{}}}\t({})",
            tokens[k].count_lo,
            (tokens[k].count_hi as i32) - 1,
            tokens[k].pair_offset
        );

        if tokens[k].kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}
