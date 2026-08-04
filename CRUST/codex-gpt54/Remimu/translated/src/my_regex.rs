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
        self.mask[(byte >> 4) as usize] |= 1 << (byte & 0x0F);
    }
    pub fn invert_mask(&mut self) {
        for entry in &mut self.mask {
            *entry = !*entry;
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1 << (byte & 0x0F))) != 0
    }
    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        if tokens.is_empty()
            || tokens[tokens.len() - 1].kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND)
        {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
            }
            if tokens.len() >= max_len {
                return Err(-2);
            }
            tokens.push(*self);
            *self = RegexToken::default();
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
const AUX_STATS_SIZE: usize = 1024;
const STACK_SIZE_MAX: usize = 1024;
const DUMMY_PREV: u32 = 0xFAC7;

fn pattern_byte(bytes: &[u8], idx: usize) -> u8 {
    bytes.get(idx).copied().unwrap_or(0)
}

fn text_byte(bytes: &[u8], idx: u64) -> u8 {
    bytes.get(idx as usize).copied().unwrap_or(0)
}

fn is_word(byte: u8) -> bool {
    byte.is_ascii_digit() || byte.is_ascii_uppercase() || byte == b'_' || byte.is_ascii_lowercase()
}

fn parse_hex_quirky(bytes: &[u8], idx: usize) -> Result<u8, i32> {
    if pattern_byte(bytes, idx + 1) == 0 || pattern_byte(bytes, idx + 2) == 0 {
        return Err(-1);
    }

    // The original C code reads the same nibble twice. Preserve that behavior.
    let mut n0 = pattern_byte(bytes, idx + 1);
    let mut n1 = pattern_byte(bytes, idx + 1);
    if !(n0.is_ascii_hexdigit() && n1.is_ascii_hexdigit()) {
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
    Ok((n1 << 4) | n0)
}

fn apply_shorthand_mask(token: &mut RegexToken, c: u8) {
    let is_upper = c.is_ascii_uppercase();
    let c = c.to_ascii_lowercase();
    let mut m = [0u16; 16];

    if c == b'd' || c == b'w' {
        m[3] |= 0x03FF;
    }
    if c == b's' {
        m[0] |= 0x3E00;
        m[2] |= 1;
    }
    if c == b'w' {
        m[4] |= 0xFFFE;
        m[5] |= 0x87FF;
        m[6] |= 0xFFFE;
        m[7] |= 0x07FF;
    }

    for (dst, src) in token.mask.iter_mut().zip(m) {
        *dst |= if is_upper { !src } else { src };
    }
}

struct MatchCtx {
    q_group_accepts_zero: [u8; AUX_STATS_SIZE],
    q_group_state: [u32; AUX_STATS_SIZE],
    q_group_stack: [u32; AUX_STATS_SIZE],
    rewind_stack: Vec<RegexMatcherState>,
    stack_n: usize,
    i: u64,
    range_min: u64,
    range_max: u64,
    just_rewinded: bool,
}

impl MatchCtx {
    fn new(start_i: usize) -> Self {
        Self {
            q_group_accepts_zero: [0; AUX_STATS_SIZE],
            q_group_state: [0; AUX_STATS_SIZE],
            q_group_stack: [0; AUX_STATS_SIZE],
            rewind_stack: Vec::with_capacity(STACK_SIZE_MAX),
            stack_n: 0,
            i: start_i as u64,
            range_min: 0,
            range_max: 0,
            just_rewinded: false,
        }
    }

    fn save_raw(&mut self, tokens: &[RegexToken], k: usize, is_dummy: bool) -> Result<(), i32> {
        if self.stack_n >= STACK_SIZE_MAX {
            return Err(-2);
        }

        let mut s = RegexMatcherState::new(k as u32, self.i);
        s.range_min = self.range_min;
        s.range_max = self.range_max;
        if is_dummy {
            s.prev = DUMMY_PREV;
        } else if tokens[k].kind == REMIMU_KIND_CLOSE {
            let group = tokens[k].mask[0] as usize;
            s.group_state = self.q_group_state[group];
            s.prev = self.q_group_stack[group];
            self.q_group_stack[group] = self.stack_n as u32;
        }

        if self.stack_n == self.rewind_stack.len() {
            self.rewind_stack.push(s);
        } else {
            self.rewind_stack[self.stack_n] = s;
        }
        self.stack_n += 1;
        Ok(())
    }

    fn save(&mut self, tokens: &[RegexToken], k: usize) -> Result<(), i32> {
        self.save_raw(tokens, k, false)
    }

    fn save_dummy(&mut self, tokens: &[RegexToken], k: usize) -> Result<(), i32> {
        self.save_raw(tokens, k, true)
    }

    fn rewind_or_abort(&mut self, tokens: &[RegexToken], k: &mut isize) -> Result<(), i32> {
        if self.stack_n == 0 {
            return Err(-1);
        }

        self.stack_n -= 1;
        while self.stack_n > 0 && self.rewind_stack[self.stack_n].prev == DUMMY_PREV {
            self.stack_n -= 1;
        }

        let s = &self.rewind_stack[self.stack_n];
        self.just_rewinded = true;
        self.range_min = s.range_min;
        self.range_max = s.range_max;
        self.i = s.i;
        *k = s.k as isize;
        if tokens[*k as usize].kind == REMIMU_KIND_CLOSE {
            let group = tokens[*k as usize].mask[0] as usize;
            self.q_group_state[group] = s.group_state;
            self.q_group_stack[group] = s.prev;
        }
        *k -= 1;
        Ok(())
    }
}

pub fn regex_parse(pattern: &str, tokens: &mut Vec<RegexToken>, token_count: &mut i16, flags: i32) -> Result<(), i32> {
    let max_len = (*token_count).max(0) as usize;
    if max_len == 0 {
        return Err(-2);
    }

    let bytes = pattern.as_bytes();
    let pattern_len = bytes.len();
    let mut esc_state = false;
    let mut state = State::Normal;
    let mut char_class_mem: i32 = -1;
    let mut token = RegexToken::default();
    let mut parsed = Vec::with_capacity(max_len.min(pattern_len + 3));

    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;
    let mut i = 0usize;
    while i < pattern_len {
        let c = bytes[i];

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
                if pattern_byte(bytes, i + 1) == 0 || !pattern_byte(bytes, i + 1).is_ascii_digit() {
                    state = State::Normal;
                } else {
                    i += 1;
                    let mut val = 0u32;
                    while pattern_byte(bytes, i).is_ascii_digit() {
                        val = val.saturating_mul(10) + (pattern_byte(bytes, i) - b'0') as u32;
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = token.count_lo.wrapping_add(1);
                    if pattern_byte(bytes, i) == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if pattern_byte(bytes, i).is_ascii_digit() {
                            let mut val2 = 0u32;
                            while pattern_byte(bytes, i).is_ascii_digit() {
                                val2 = val2.saturating_mul(10) + (pattern_byte(bytes, i) - b'0') as u32;
                                if val2 > 0xFFFF {
                                    return Err(-1);
                                }
                                i += 1;
                            }
                            if val2 < val {
                                return Err(-1);
                            }
                            token.count_hi = (val2 as u16).wrapping_add(1);
                        }
                    }

                    if pattern_byte(bytes, i) == b'}' {
                        i += 1;
                        continue;
                    } else {
                        return Err(-1);
                    }
                }
            }
        }

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
            if esc_state {
                esc_state = false;
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
                    token.set_mask(parse_hex_quirky(bytes, i)?);
                    i += 2;
                } else if matches!(
                    c,
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$' | b'*'
                        | b'+' | b'?' | b':' | b'.' | b'/' | b'\\'
                ) {
                    token.set_mask(c);
                    state = State::Quant;
                    i += 1;
                    continue;
                } else if matches!(c, b'd' | b's' | b'w' | b'D' | b'S' | b'W') {
                    apply_shorthand_mask(&mut token, c);
                    token.kind = REMIMU_KIND_NORMAL;
                    state = State::Quant;
                    i += 1;
                    continue;
                } else if c == b'b' {
                    token.kind = REMIMU_KIND_BOUND;
                    i += 1;
                    continue;
                } else if c == b'B' {
                    token.kind = REMIMU_KIND_NBOUND;
                    i += 1;
                    continue;
                } else {
                    return Err(-1);
                }
            } else {
                token.push_to_vec(&mut parsed, max_len)?;
                if c == b'\\' {
                    esc_state = true;
                    i += 1;
                    continue;
                } else if c == b'[' {
                    state = State::CharClassInit;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if pattern_byte(bytes, i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                    i += 1;
                    continue;
                } else if c == b'(' {
                    paren_count += 1;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if pattern_byte(bytes, i + 1) == b'?' && pattern_byte(bytes, i + 2) == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if pattern_byte(bytes, i + 1) == b'?' && pattern_byte(bytes, i + 2) == b'>' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.push_to_vec(&mut parsed, max_len)?;
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.count_lo = 1;
                        token.count_hi = 2;
                        i += 2;
                    }
                    i += 1;
                    continue;
                } else if c == b')' {
                    paren_count -= 1;
                    if paren_count < 0 || parsed.is_empty() {
                        return Err(-1);
                    }
                    token.kind = REMIMU_KIND_CLOSE;
                    state = State::Quant;

                    let mut balance = 0i32;
                    let mut found = None;
                    for l in (0..parsed.len()).rev() {
                        if parsed[l].kind == REMIMU_KIND_NCOPEN || parsed[l].kind == REMIMU_KIND_OPEN {
                            if balance == 0 {
                                found = Some(l);
                                break;
                            } else {
                                balance -= 1;
                            }
                        } else if parsed[l].kind == REMIMU_KIND_CLOSE {
                            balance += 1;
                        }
                    }

                    let found = found.ok_or(-1)?;
                    let diff = parsed.len() - found;
                    if diff > i16::MAX as usize {
                        return Err(-1);
                    }
                    token.pair_offset = -(diff as i16);
                    parsed[found].pair_offset = diff as i16;
                    if parsed[found].mode == REMIMU_MODE_POSSESSIVE {
                        token.push_to_vec(&mut parsed, max_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        parsed[found - 1].pair_offset = diff as i16 + 2;
                    }
                    i += 1;
                    continue;
                } else if matches!(c, b'?' | b'+' | b'*' | b'{') {
                    return Err(-1);
                } else if c == b'.' {
                    token.mask.fill(0xFFFF);
                    if flags & REMIMU_FLAG_DOT_NO_NEWLINES != 0 {
                        token.mask[1] ^= 0x04;
                        token.mask[1] ^= 0x20;
                    }
                    state = State::Quant;
                    i += 1;
                    continue;
                } else if c == b'^' {
                    token.kind = REMIMU_KIND_CARET;
                    i += 1;
                    continue;
                } else if c == b'$' {
                    token.kind = REMIMU_KIND_DOLLAR;
                    i += 1;
                    continue;
                } else if c == b'|' {
                    token.kind = REMIMU_KIND_OR;
                    i += 1;
                    continue;
                } else {
                    token.set_mask(c);
                    state = State::Quant;
                    i += 1;
                    continue;
                }
            }

            i += 1;
            continue;
        }

        if matches!(state, State::CharClassInit | State::CharClassNormal | State::CharClassRange) {
            if c == b'\\' && !esc_state {
                esc_state = true;
                i += 1;
                continue;
            }

            let mut esc_c = 0u8;
            if esc_state {
                esc_state = false;
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
                    esc_c = parse_hex_quirky(bytes, i)?;
                    i += 2;
                } else if matches!(
                    c,
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$' | b'*'
                        | b'+' | b'?' | b':' | b'.' | b'/' | b'\\'
                ) {
                    esc_c = c;
                } else if matches!(c, b'd' | b's' | b'w' | b'D' | b'S' | b'W') {
                    if matches!(state, State::CharClassRange) {
                        return Err(-1);
                    }
                    apply_shorthand_mask(&mut token, c);
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
            } else {
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
                    if c < char_class_mem as u8 {
                        return Err(-1);
                    }
                    for b in ((char_class_mem as u8) + 1..=c).rev() {
                        token.set_mask(b);
                    }
                    state = State::CharClassNormal;
                    char_class_mem = -1;
                }
            }

            i += 1;
            continue;
        }
    }

    if paren_count > 0 || esc_state || matches!(state, State::CharClassInit | State::CharClassNormal | State::CharClassRange) {
        return Err(-1);
    }

    token.push_to_vec(&mut parsed, max_len)?;

    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(&mut parsed, max_len)?;

    token.kind = REMIMU_KIND_END;
    token.push_to_vec(&mut parsed, max_len)?;

    let k = parsed.len();
    parsed[0].pair_offset = (k - 2) as i16;
    parsed[k - 2].pair_offset = -((k - 2) as i16);
    *token_count = k as i16;

    let mut n = 0u64;
    for k2 in 0..k {
        if parsed[k2].kind == REMIMU_KIND_CLOSE {
            parsed[k2].mask[0] = n as u16;
            n += 1;
            let k3 = (k2 as isize + parsed[k2].pair_offset as isize) as usize;
            parsed[k3].count_lo = parsed[k2].count_lo;
            parsed[k3].count_hi = parsed[k2].count_hi;
            parsed[k3].mask[0] = n as u16;
            n += 1;
            parsed[k3].mode = parsed[k2].mode;
            if n > 1024 {
                return Err(-1);
            }
        } else if matches!(parsed[k2].kind, REMIMU_KIND_OR | REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN) {
            let mut balance = 0i32;
            let mut found = None;
            for l in k2 + 1..k {
                if parsed[l].kind == REMIMU_KIND_OR && balance == 0 {
                    found = Some(l);
                    break;
                } else if parsed[l].kind == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = Some(l);
                        break;
                    } else {
                        balance -= 1;
                    }
                } else if parsed[l].kind == REMIMU_KIND_NCOPEN || parsed[l].kind == REMIMU_KIND_OPEN {
                    balance += 1;
                }
            }

            let found = found.ok_or(-1)?;
            let diff = found - k2;
            if diff > i16::MAX as usize {
                return Err(-1);
            }

            if parsed[k2].kind == REMIMU_KIND_OR {
                parsed[k2].pair_offset = diff as i16;
            } else {
                parsed[k2].mask[15] = diff as u16;
            }
        }
    }

    if tokens.len() < k {
        tokens.resize(k, RegexToken::default());
    }
    for (dst, src) in tokens.iter_mut().zip(parsed.iter()) {
        *dst = *src;
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
pub fn regex_match(tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64]) -> Option<usize> {
    let cap_slots = usize::from(cap_slots)
        .min(AUX_STATS_SIZE)
        .min(cap_pos.len())
        .min(cap_span.len());
    let text_bytes = text.as_bytes();
    let mut q_group_cap_index = [u16::MAX; AUX_STATS_SIZE];
    let mut caps = 0usize;
    let mut k = 0usize;

    while k < tokens.len() && tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let close_k = (k as isize + tokens[k].pair_offset as isize) as usize;
            let open_group = tokens[k].mask[0] as usize;
            let close_group = tokens[close_k].mask[0] as usize;
            q_group_cap_index[open_group] = caps as u16;
            q_group_cap_index[close_group] = caps as u16;
            cap_pos[caps] = -1;
            cap_span[caps] = -1;
            caps += 1;
        }
        k += 1;
        if k < tokens.len() && matches!(tokens[k].kind, REMIMU_KIND_CLOSE | REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN) {
            let group = tokens[k].mask[0] as usize;
            if group >= AUX_STATS_SIZE {
                return None;
            }
        }
    }

    if k >= tokens.len() || tokens[k].kind != REMIMU_KIND_END {
        return None;
    }
    let tokens_len = k;
    let mut ctx = MatchCtx::new(start_i);

    let mut k: isize = 0;
    while (k as usize) < tokens_len {
        let tk = tokens[k as usize];
        if tk.kind == REMIMU_KIND_CARET {
            if ctx.i != 0 && ctx.rewind_or_abort(tokens, &mut k).is_err() {
                return None;
            }
        } else if tk.kind == REMIMU_KIND_DOLLAR {
            if text_byte(text_bytes, ctx.i) != 0 && ctx.rewind_or_abort(tokens, &mut k).is_err() {
                return None;
            }
        } else if tk.kind == REMIMU_KIND_BOUND {
            let curr = text_byte(text_bytes, ctx.i);
            if (ctx.i == 0 && !is_word(curr))
                || (ctx.i != 0 && curr == 0 && !is_word(text_byte(text_bytes, ctx.i - 1)))
                || (ctx.i != 0 && curr != 0 && (is_word(text_byte(text_bytes, ctx.i - 1)) == is_word(curr)))
            {
                if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                    return None;
                }
            }
        } else if tk.kind == REMIMU_KIND_NBOUND {
            let curr = text_byte(text_bytes, ctx.i);
            if (ctx.i == 0 && is_word(curr))
                || (ctx.i != 0 && curr == 0 && is_word(text_byte(text_bytes, ctx.i - 1)))
                || (ctx.i != 0 && curr != 0 && (is_word(text_byte(text_bytes, ctx.i - 1)) != is_word(curr)))
            {
                if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                    return None;
                }
            }
        } else {
            if tk.count_hi == 1 {
                if tk.kind == REMIMU_KIND_OPEN || tk.kind == REMIMU_KIND_NCOPEN {
                    k += tk.pair_offset as isize;
                } else {
                    k += 1;
                }
                k += 1;
                continue;
            }

            if tk.kind == REMIMU_KIND_OPEN || tk.kind == REMIMU_KIND_NCOPEN {
                let close_group = tokens[(k + tk.pair_offset as isize) as usize].mask[0] as usize;
                if !ctx.just_rewinded {
                    if (tk.mode & REMIMU_MODE_LAZY) != 0
                        && (tk.count_lo == 0 || ctx.q_group_accepts_zero[close_group] != 0)
                    {
                        ctx.range_min = 0;
                        ctx.range_max = 0;
                        if ctx.save(tokens, k as usize).is_err() {
                            return None;
                        }
                        k += tk.pair_offset as isize;
                    } else {
                        ctx.range_min = 1;
                        ctx.range_max = 0;
                        if ctx.save(tokens, k as usize).is_err() {
                            return None;
                        }
                    }
                } else {
                    ctx.just_rewinded = false;
                    let orig_k = k;

                    if ctx.range_min != 0 {
                        k += ctx.range_min as isize;
                        let prev = tokens[(k - 1) as usize];
                        if prev.kind == REMIMU_KIND_OR {
                            k += prev.pair_offset as isize - 1;
                        } else if prev.kind == REMIMU_KIND_OPEN || prev.kind == REMIMU_KIND_NCOPEN {
                            k += prev.mask[15] as isize - 1;
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_END {
                            return None;
                        }

                        if tokens[k as usize].kind == REMIMU_KIND_CLOSE {
                            let group = tokens[k as usize].mask[0] as usize;
                            if tokens[k as usize].count_lo == 0 || ctx.q_group_accepts_zero[group] != 0 {
                                ctx.q_group_state[group] = 0;
                                if (tokens[k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                    ctx.q_group_stack[group] = 0;
                                }
                                k += 1;
                                continue;
                            } else {
                                if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                                    return None;
                                }
                                k += 1;
                                continue;
                            }
                        }
                    }

                    let k_diff = k - orig_k;
                    ctx.range_min = (k_diff + 1) as u64;
                    if ctx.save(tokens, (k - k_diff) as usize).is_err() {
                        return None;
                    }
                }
            } else if tk.kind == REMIMU_KIND_CLOSE {
                if tk.count_lo == 1 && tk.count_hi == 2 {
                    let cap_index = q_group_cap_index[tk.mask[0] as usize];
                    if cap_index != u16::MAX && ctx.save_dummy(tokens, k as usize).is_err() {
                        return None;
                    }
                } else {
                    let group = tk.mask[0] as usize;
                    if !ctx.just_rewinded {
                        let prev = ctx.q_group_stack[group];
                        ctx.range_max = u64::from(tk.count_hi).wrapping_sub(1);
                        ctx.range_min = if ctx.q_group_accepts_zero[group] != 0 {
                            0
                        } else {
                            u64::from(tk.count_lo)
                        };

                        if ctx.q_group_state[group] + 1 < ctx.range_min as u32 {
                            ctx.q_group_state[group] += 1;
                            if ctx.save(tokens, k as usize).is_err() {
                                return None;
                            }
                            k += tk.pair_offset as isize;
                            k -= 1;
                            k += 1;
                            continue;
                        } else if tk.count_hi != 0 && ctx.q_group_state[group] + 1 > ctx.range_max as u32 {
                            ctx.range_max = ctx.range_max.wrapping_sub(1);
                            if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                                return None;
                            }
                            k += 1;
                            continue;
                        }

                        let mut force_zero = false;
                        if prev != 0 && ctx.rewind_stack[prev as usize].i > ctx.i {
                            let mut n = ctx.stack_n.saturating_sub(1);
                            while n > 0 && ctx.rewind_stack[n].k != (k + tk.pair_offset as isize) as u32 {
                                n -= 1;
                            }
                            if n == 0 {
                                return None;
                            }
                            if ctx.rewind_stack[n].i == ctx.i {
                                force_zero = true;
                            }
                        }

                        if force_zero || (prev != 0 && ctx.rewind_stack[prev as usize].i == ctx.i) {
                            ctx.q_group_accepts_zero[group] = 1;
                            if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                                return None;
                            }
                            k += 1;
                            continue;
                        } else if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                            ctx.q_group_state[group] += 1;
                            if ctx.save(tokens, k as usize).is_err() {
                                return None;
                            }
                            ctx.q_group_state[group] = 0;
                        } else {
                            if (tk.mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let mut k2 = k;
                                if ctx.q_group_state[group] == 0 {
                                    k2 = k + tk.pair_offset as isize;
                                }
                                if ctx.stack_n == 0 {
                                    return None;
                                }
                                ctx.stack_n -= 1;
                                while ctx.stack_n > 0 && ctx.rewind_stack[ctx.stack_n].k != k2 as u32 {
                                    ctx.stack_n -= 1;
                                }
                                if ctx.stack_n == 0 {
                                    return None;
                                }
                            }

                            let open_group = tokens[(k + tk.pair_offset as isize) as usize].mask[0] as usize;
                            if ctx.q_group_state[open_group] < ctx.i as u32 {
                                ctx.q_group_state[group] += 1;
                                if ctx.save(tokens, k as usize).is_err() {
                                    return None;
                                }
                                k += tk.pair_offset as isize;
                                k -= 1;
                            }
                        }
                    } else {
                        ctx.just_rewinded = false;
                        if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                            if ctx.save_dummy(tokens, k as usize).is_err() {
                                return None;
                            }
                            ctx.q_group_stack[group] = ctx.stack_n as u32;
                            k += tk.pair_offset as isize;
                            k -= 1;
                        } else if ctx.q_group_state[group] < ctx.range_min as u32
                            && ctx.q_group_accepts_zero[group] == 0
                        {
                            if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                                return None;
                            }
                        } else {
                            ctx.q_group_state[group] = 0;
                            let cap_index = q_group_cap_index[group];
                            if cap_index != u16::MAX && ctx.save_dummy(tokens, k as usize).is_err() {
                                return None;
                            }
                        }
                    }
                }
            } else if tk.kind == REMIMU_KIND_OR {
                k += tk.pair_offset as isize;
                k -= 1;
            } else if tk.kind == REMIMU_KIND_NORMAL {
                if !ctx.just_rewinded {
                    let mut n = 0u64;
                    let old_i = ctx.i;
                    while n < u64::from(tk.count_lo)
                        && text_byte(text_bytes, ctx.i) != 0
                        && tk.check_mask(text_byte(text_bytes, ctx.i))
                    {
                        ctx.i += 1;
                        n += 1;
                    }
                    if n < u64::from(tk.count_lo) {
                        ctx.i = old_i;
                        if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                            return None;
                        }
                        k += 1;
                        continue;
                    }

                    if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                        ctx.range_min = n;
                        ctx.range_max = u64::from(tk.count_hi).wrapping_sub(1);
                        if ctx.save(tokens, k as usize).is_err() {
                            return None;
                        }
                    } else {
                        let mut limit = u64::from(tk.count_hi);
                        if limit == 0 {
                            limit = !0;
                        }
                        ctx.range_min = n;
                        while text_byte(text_bytes, ctx.i) != 0
                            && tk.check_mask(text_byte(text_bytes, ctx.i))
                            && n + 1 < limit
                        {
                            ctx.i += 1;
                            n += 1;
                        }
                        ctx.range_max = n;
                        if (tk.mode & REMIMU_MODE_POSSESSIVE) == 0
                            && ctx.save(tokens, k as usize).is_err()
                        {
                            return None;
                        }
                    }
                } else {
                    ctx.just_rewinded = false;
                    if (tk.mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit = ctx.range_max;
                        if limit == 0 {
                            limit = !0;
                        }
                        if text_byte(text_bytes, ctx.i) != 0
                            && tk.check_mask(text_byte(text_bytes, ctx.i))
                            && ctx.range_min < limit
                        {
                            ctx.i += 1;
                            ctx.range_min += 1;
                            if ctx.save(tokens, k as usize).is_err() {
                                return None;
                            }
                        } else {
                            if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                                return None;
                            }
                        }
                    } else if ctx.range_max > ctx.range_min {
                        ctx.i = ctx.i.saturating_sub(1);
                        ctx.range_max -= 1;
                        if ctx.save(tokens, k as usize).is_err() {
                            return None;
                        }
                    } else if ctx.rewind_or_abort(tokens, &mut k).is_err() {
                        return None;
                    }
                }
            } else {
                return None;
            }
        }

        k += 1;
    }

    if caps != 0 {
        for n in 0..ctx.stack_n {
            let s = &ctx.rewind_stack[n];
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[s.k as usize].mask[0] as usize];
                if cap_index == u16::MAX {
                    continue;
                }
                let cap_index = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[cap_index] = s.i as i64;
                } else if cap_pos[cap_index] >= 0 {
                    cap_span[cap_index] = s.i as i64 - cap_pos[cap_index];
                }
            }
        }
        for n in 0..caps {
            if cap_span[n] == -1 {
                cap_pos[n] = -1;
            }
        }
    }

    Some(ctx.i as usize)
}
pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];

    for (k, token) in tokens.iter().enumerate() {
        let mode_to_str = match token.mode {
            0 => "GREEDY",
            1 => "POSSESS",
            2 => "LAZY",
            _ => "UNKNOWN",
        };
        print!("{}\t{}\t", kind_to_str[token.kind as usize], mode_to_str);

        let mut c_old: i32 = -1;
        if token.kind == REMIMU_KIND_NORMAL {
            for c in 0u16..256 {
                let c_u8 = c as u8;
                if token.check_mask(c_u8) {
                    if c_old == -1 {
                        c_old = c as i32;
                    }
                } else if c_old != -1 {
                    let print_c = |c: i32| {
                        if (0x20..=0x7E).contains(&c) {
                            print!("{}", c as u8 as char);
                        } else {
                            print!("\\x{:02x}", c);
                        }
                    };
                    if c as i32 - 1 == c_old {
                        print_c(c_old);
                    } else if c as i32 - 2 == c_old {
                        print_c(c_old);
                        print_c(c_old + 1);
                    } else {
                        print_c(c_old);
                        print!("-");
                        print_c(c as i32 - 1);
                    }
                    c_old = -1;
                }
            }
        }

        println!("\t{{{},{}}}\t({})", token.count_lo, token.count_hi.wrapping_sub(1), token.pair_offset);
        if token.kind == REMIMU_KIND_END || k + 1 >= tokens.len() {
            break;
        }
    }
}
