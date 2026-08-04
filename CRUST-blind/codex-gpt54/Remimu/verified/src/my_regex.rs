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
        self.mask[(byte >> 4) as usize] |= 1 << (byte & 0xF);
    }

    pub fn invert_mask(&mut self) {
        for mask in &mut self.mask {
            *mask = !*mask;
        }
        self.mode &= !REMIMU_MODE_INVERTED;
    }

    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
    }

    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        if tokens.is_empty()
            || tokens[tokens.len() - 1].kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND)
        {
            if (self.mode & REMIMU_MODE_INVERTED) != 0 {
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

impl Copy for RegexMatcherState {}

impl Clone for RegexMatcherState {
    fn clone(&self) -> Self {
        *self
    }
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

#[derive(Debug, Clone, Copy)]
enum State {
    Normal,
    Quant,
    Mode,
    CharClassInit,
    CharClassNormal,
    CharClassRange,
}

fn pattern_byte(pattern: &[u8], index: usize) -> u8 {
    pattern.get(index).copied().unwrap_or(0)
}

fn text_byte(text: &[u8], index: u64) -> u8 {
    text.get(index as usize).copied().unwrap_or(0)
}

fn add_offset(index: usize, offset: i16) -> Result<usize, i32> {
    let value = index as isize + offset as isize;
    if value < 0 {
        return Err(-3);
    }
    Ok(value as usize)
}

fn parse_hex_with_c_bug(pattern: &[u8], index: usize) -> Result<(u8, usize), i32> {
    if pattern_byte(pattern, index + 1) == 0 || pattern_byte(pattern, index + 2) == 0 {
        return Err(-1);
    }

    let mut n0 = pattern[index + 1];
    let mut n1 = pattern[index + 1];
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
    Ok((((n1 << 4) | n0), 2))
}

fn apply_shorthand_mask(token: &mut RegexToken, c: u8) {
    let mut ch = c;
    let is_upper = ch <= b'Z';
    let mut mask = [0u16; 16];

    if is_upper {
        ch += 0x20;
    }
    if ch == b'd' || ch == b'w' {
        mask[3] |= 0x03FF;
    }
    if ch == b's' {
        mask[0] |= 0x3E00;
        mask[2] |= 1;
    }
    if ch == b'w' {
        mask[4] |= 0xFFFE;
        mask[5] |= 0x87FF;
        mask[6] |= 0xFFFE;
        mask[7] |= 0x07FF;
    }

    for (dst, src) in token.mask.iter_mut().zip(mask.iter()) {
        *dst |= if is_upper { !*src } else { *src };
    }
}

pub fn regex_parse(
    pattern: &str,
    tokens: &mut Vec<RegexToken>,
    token_count: &mut i16,
    flags: i32,
) -> Result<(), i32> {
    let max_len = usize::try_from(*token_count).unwrap_or(0);
    let pattern = pattern.as_bytes();

    tokens.clear();

    let mut esc_state = false;
    let mut state = State::Normal;
    let mut char_class_mem: i32 = -1;
    let mut token = RegexToken::default();
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count = 0i32;
    let mut i = 0usize;
    while i < pattern.len() {
        let c = pattern[i];

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
                if pattern_byte(pattern, i + 1) == 0
                    || !pattern_byte(pattern, i + 1).is_ascii_digit()
                {
                    state = State::Normal;
                } else {
                    i += 1;
                    let mut val = 0u32;
                    while pattern_byte(pattern, i).is_ascii_digit() {
                        val *= 10;
                        val += (pattern[i] - b'0') as u32;
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = token.count_lo.wrapping_add(1);
                    if pattern_byte(pattern, i) == b',' {
                        token.count_hi = 0;
                        i += 1;

                        if pattern_byte(pattern, i).is_ascii_digit() {
                            let mut val2 = 0u32;
                            while pattern_byte(pattern, i).is_ascii_digit() {
                                val2 *= 10;
                                val2 += (pattern[i] - b'0') as u32;
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

                    if pattern_byte(pattern, i) == b'}' {
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
                match c {
                    b'n' => token.set_mask(b'\n'),
                    b'r' => token.set_mask(b'\r'),
                    b't' => token.set_mask(b'\t'),
                    b'v' => token.set_mask(0x0B),
                    b'f' => token.set_mask(0x0C),
                    b'x' => {
                        let (byte, consumed) = parse_hex_with_c_bug(pattern, i)?;
                        token.set_mask(byte);
                        i += consumed;
                    }
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$'
                    | b'*' | b'+' | b'?' | b':' | b'.' | b'/' | b'\\' => {
                        token.set_mask(c);
                        state = State::Quant;
                    }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        apply_shorthand_mask(&mut token, c);
                        token.kind = REMIMU_KIND_NORMAL;
                        state = State::Quant;
                    }
                    b'b' => {
                        token.kind = REMIMU_KIND_BOUND;
                        state = State::Normal;
                    }
                    b'B' => {
                        token.kind = REMIMU_KIND_NBOUND;
                        state = State::Normal;
                    }
                    _ => return Err(-1),
                }
            } else {
                token.push_to_vec(tokens, max_len)?;
                match c {
                    b'\\' => {
                        esc_state = true;
                    }
                    b'[' => {
                        state = State::CharClassInit;
                        char_class_mem = -1;
                        token.kind = REMIMU_KIND_NORMAL;
                        if pattern_byte(pattern, i + 1) == b'^' {
                            token.mode |= REMIMU_MODE_INVERTED;
                            i += 1;
                        }
                    }
                    b'(' => {
                        paren_count += 1;
                        token.kind = REMIMU_KIND_OPEN;
                        token.count_lo = 0;
                        token.count_hi = 1;
                        if pattern_byte(pattern, i + 1) == b'?' && pattern_byte(pattern, i + 2) == b':' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            i += 2;
                        } else if pattern_byte(pattern, i + 1) == b'?'
                            && pattern_byte(pattern, i + 2) == b'>'
                        {
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.push_to_vec(tokens, max_len)?;
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.count_lo = 1;
                            token.count_hi = 2;
                            i += 2;
                        }
                    }
                    b')' => {
                        paren_count -= 1;
                        if paren_count < 0 || tokens.is_empty() {
                            return Err(-1);
                        }
                        token.kind = REMIMU_KIND_CLOSE;
                        state = State::Quant;

                        let mut balance = 0i32;
                        let mut found = None;
                        for l in (0..tokens.len()).rev() {
                            match tokens[l].kind {
                                REMIMU_KIND_NCOPEN | REMIMU_KIND_OPEN => {
                                    if balance == 0 {
                                        found = Some(l);
                                        break;
                                    }
                                    balance -= 1;
                                }
                                REMIMU_KIND_CLOSE => balance += 1,
                                _ => {}
                            }
                        }
                        let found = found.ok_or(-1)?;
                        let diff = tokens.len() as isize - found as isize;
                        if diff > i16::MAX as isize {
                            return Err(-1);
                        }
                        token.pair_offset = -(diff as i16);
                        tokens[found].pair_offset = diff as i16;
                        if tokens[found].mode == REMIMU_MODE_POSSESSIVE {
                            token.push_to_vec(tokens, max_len)?;
                            token.kind = REMIMU_KIND_CLOSE;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.pair_offset = -(diff as i16) - 2;
                            if found == 0 {
                                return Err(-1);
                            }
                            tokens[found - 1].pair_offset = diff as i16 + 2;
                        }
                    }
                    b'?' | b'+' | b'*' | b'{' => {
                        return Err(-1);
                    }
                    b'.' => {
                        for mask in &mut token.mask {
                            *mask = 0xFFFF;
                        }
                        if (flags & REMIMU_FLAG_DOT_NO_NEWLINES) != 0 {
                            token.mask[1] ^= 0x04;
                            token.mask[1] ^= 0x20;
                        }
                        state = State::Quant;
                    }
                    b'^' => {
                        token.kind = REMIMU_KIND_CARET;
                    }
                    b'$' => {
                        token.kind = REMIMU_KIND_DOLLAR;
                    }
                    b'|' => {
                        token.kind = REMIMU_KIND_OR;
                    }
                    _ => {
                        token.set_mask(c);
                        state = State::Quant;
                    }
                }
            }
        } else {
            if c == b'\\' && !esc_state {
                esc_state = true;
                i += 1;
                continue;
            }
            let mut esc_c = 0u8;
            if esc_state {
                esc_state = false;
                match c {
                    b'n' => esc_c = b'\n',
                    b'r' => esc_c = b'\r',
                    b't' => esc_c = b'\t',
                    b'v' => esc_c = 0x0B,
                    b'f' => esc_c = 0x0C,
                    b'x' => {
                        let (byte, consumed) = parse_hex_with_c_bug(pattern, i)?;
                        esc_c = byte;
                        i += consumed;
                    }
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$'
                    | b'*' | b'+' | b'?' | b':' | b'.' | b'/' | b'\\' => {
                        esc_c = c;
                    }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        if matches!(state, State::CharClassRange) {
                            return Err(-1);
                        }
                        apply_shorthand_mask(&mut token, c);
                        char_class_mem = -1;
                        i += 1;
                        continue;
                    }
                    _ => return Err(-1),
                }
            }

            match state {
                State::CharClassInit => {
                    char_class_mem = c as i32;
                    token.set_mask(c);
                    state = State::CharClassNormal;
                }
                State::CharClassNormal => {
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
                }
                State::CharClassRange => {
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
                        let mut value = c;
                        while value > char_class_mem as u8 {
                            token.set_mask(value);
                            value = value.wrapping_sub(1);
                        }
                        state = State::CharClassNormal;
                        char_class_mem = -1;
                    }
                }
                _ => {}
            }
        }

        i += 1;
    }

    if paren_count > 0 || esc_state || matches!(state, State::CharClassInit | State::CharClassNormal | State::CharClassRange) {
        return Err(-1);
    }

    token.push_to_vec(tokens, max_len)?;

    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, max_len)?;

    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len)?;

    if tokens.len() < 3 {
        return Err(-1);
    }

    let k = tokens.len();
    tokens[0].pair_offset = (k - 2) as i16;
    tokens[k - 2].pair_offset = -((k - 2) as i16);
    *token_count = k as i16;

    let mut n = 0u64;
    for k2 in 0..k {
        if tokens[k2].kind == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16;
            n += 1;

            let k3 = add_offset(k2, tokens[k2].pair_offset)?;
            tokens[k3].count_lo = tokens[k2].count_lo;
            tokens[k3].count_hi = tokens[k2].count_hi;
            tokens[k3].mask[0] = n as u16;
            n += 1;
            tokens[k3].mode = tokens[k2].mode;

            if n > 1024 {
                return Err(-1);
            }
        } else if matches!(
            tokens[k2].kind,
            REMIMU_KIND_OR | REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN
        ) {
            let mut balance = 0i32;
            let mut found = None;
            for l in (k2 + 1)..k {
                if tokens[l].kind == REMIMU_KIND_OR && balance == 0 {
                    found = Some(l);
                    break;
                } else if tokens[l].kind == REMIMU_KIND_CLOSE {
                    if balance == 0 {
                        found = Some(l);
                        break;
                    }
                    balance -= 1;
                } else if matches!(tokens[l].kind, REMIMU_KIND_NCOPEN | REMIMU_KIND_OPEN) {
                    balance += 1;
                }
            }
            let found = found.ok_or(-1)?;
            let diff = found - k2;
            if diff > i16::MAX as usize {
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

fn save_rewind_state(
    rewind_stack: &mut [RegexMatcherState],
    stack_n: &mut usize,
    tokens: &[RegexToken],
    k: usize,
    i: u64,
    range_min: u64,
    range_max: u64,
    q_group_state: &mut [u32],
    q_group_stack: &mut [u32],
    is_dummy: bool,
) -> Result<(), i32> {
    if *stack_n >= rewind_stack.len() {
        return Err(-2);
    }

    let mut state = RegexMatcherState::new(k as u32, i);
    state.range_min = range_min;
    state.range_max = range_max;
    if is_dummy {
        state.prev = 0xFAC7;
    } else if tokens[k].kind == REMIMU_KIND_CLOSE {
        let group = tokens[k].mask[0] as usize;
        state.group_state = q_group_state[group];
        state.prev = q_group_stack[group];
        q_group_stack[group] = *stack_n as u32;
    }
    rewind_stack[*stack_n] = state;
    *stack_n += 1;
    Ok(())
}

fn rewind_or_abort(
    rewind_stack: &[RegexMatcherState],
    stack_n: &mut usize,
    tokens: &[RegexToken],
    q_group_state: &mut [u32],
    q_group_stack: &mut [u32],
    i: &mut u64,
    range_min: &mut u64,
    range_max: &mut u64,
    just_rewinded: &mut bool,
) -> Result<usize, i32> {
    if *stack_n == 0 {
        return Err(-1);
    }
    *stack_n -= 1;
    while *stack_n > 0 && rewind_stack[*stack_n].prev == 0xFAC7 {
        *stack_n -= 1;
    }

    let state = rewind_stack[*stack_n];
    *just_rewinded = true;
    *range_min = state.range_min;
    *range_max = state.range_max;
    *i = state.i;
    let k = state.k as usize;
    if tokens[k].kind == REMIMU_KIND_CLOSE {
        let group = tokens[k].mask[0] as usize;
        q_group_state[group] = state.group_state;
        q_group_stack[group] = state.prev;
    }
    Ok(k)
}

fn regex_match_impl(
    tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Result<usize, i32> {
    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;

    if start_i > text.len() {
        return Err(-3);
    }

    let mut cap_slots = cap_slots as usize;
    cap_slots = cap_slots.min(AUX_STATS_SIZE).min(cap_pos.len()).min(cap_span.len());

    let mut q_group_accepts_zero = [0u8; AUX_STATS_SIZE];
    let mut q_group_state = [0u32; AUX_STATS_SIZE];
    let mut q_group_stack = [0u32; AUX_STATS_SIZE];
    let mut q_group_cap_index = [u16::MAX; AUX_STATS_SIZE];

    let mut k = 0usize;
    let mut caps = 0usize;
    while k < tokens.len() && tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let open_group = tokens[k].mask[0] as usize;
            let close_index = add_offset(k, tokens[k].pair_offset)?;
            let close_group = tokens[close_index].mask[0] as usize;
            q_group_cap_index[open_group] = caps as u16;
            q_group_cap_index[close_group] = caps as u16;
            cap_pos[caps] = -1;
            cap_span[caps] = -1;
            caps += 1;
        }
        k += 1;
        if k >= tokens.len() {
            return Err(-3);
        }
        if matches!(
            tokens[k].kind,
            REMIMU_KIND_CLOSE | REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN
        ) {
            let group = tokens[k].mask[0] as usize;
            if group >= AUX_STATS_SIZE {
                return Err(-2);
            }
            q_group_state[group] = 0;
            q_group_stack[group] = 0;
            q_group_accepts_zero[group] = 0;
        }
    }
    if k >= tokens.len() {
        return Err(-3);
    }
    let tokens_len = k;

    let mut rewind_stack = [RegexMatcherState::new(0, 0); STACK_SIZE_MAX];
    let mut stack_n = 0usize;
    let bytes = text.as_bytes();
    let mut i = start_i as u64;
    let mut range_min = 0u64;
    let mut range_max = 0u64;
    let mut just_rewinded = false;

    let mut w_mask = [0u64; 16];
    w_mask[3] = 0x03FF;
    w_mask[4] = 0xFFFE;
    w_mask[5] = 0x87FF;
    w_mask[6] = 0xFFFE;
    w_mask[7] = 0x07FF;
    let is_word = |byte: u8| -> bool { (w_mask[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0 };

    k = 0;
    while k < tokens_len {
        let token = tokens[k];
        if token.kind == REMIMU_KIND_CARET {
            if i != 0 {
                k = rewind_or_abort(
                    &rewind_stack,
                    &mut stack_n,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                )?;
                continue;
            }
            k += 1;
            continue;
        } else if token.kind == REMIMU_KIND_DOLLAR {
            if text_byte(bytes, i) != 0 {
                k = rewind_or_abort(
                    &rewind_stack,
                    &mut stack_n,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                )?;
                continue;
            }
            k += 1;
            continue;
        } else if token.kind == REMIMU_KIND_BOUND {
            if (i == 0 && !is_word(text_byte(bytes, i)))
                || (i != 0 && text_byte(bytes, i) == 0 && !is_word(text_byte(bytes, i - 1)))
                || (i != 0
                    && text_byte(bytes, i) != 0
                    && is_word(text_byte(bytes, i - 1)) == is_word(text_byte(bytes, i)))
            {
                k = rewind_or_abort(
                    &rewind_stack,
                    &mut stack_n,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                )?;
                continue;
            }
            k += 1;
            continue;
        } else if token.kind == REMIMU_KIND_NBOUND {
            if (i == 0 && is_word(text_byte(bytes, i)))
                || (i != 0 && text_byte(bytes, i) == 0 && is_word(text_byte(bytes, i - 1)))
                || (i != 0
                    && text_byte(bytes, i) != 0
                    && is_word(text_byte(bytes, i - 1)) != is_word(text_byte(bytes, i)))
            {
                k = rewind_or_abort(
                    &rewind_stack,
                    &mut stack_n,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                )?;
                continue;
            }
            k += 1;
            continue;
        }

        if token.count_hi == 1 {
            if matches!(token.kind, REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN) {
                k = add_offset(k, token.pair_offset)? + 1;
            } else {
                k += 2;
            }
            continue;
        }

        if matches!(token.kind, REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN) {
            if !just_rewinded {
                if (token.mode & REMIMU_MODE_LAZY) != 0 {
                    let close_group = tokens[add_offset(k, token.pair_offset)?].mask[0] as usize;
                    if token.count_lo == 0 || q_group_accepts_zero[close_group] != 0 {
                        range_min = 0;
                        range_max = 0;
                        save_rewind_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            tokens,
                            k,
                            i,
                            range_min,
                            range_max,
                            &mut q_group_state,
                            &mut q_group_stack,
                            false,
                        )?;
                        k = add_offset(k, token.pair_offset)? + 1;
                        continue;
                    }
                }

                range_min = 1;
                range_max = 0;
                save_rewind_state(
                    &mut rewind_stack,
                    &mut stack_n,
                    tokens,
                    k,
                    i,
                    range_min,
                    range_max,
                    &mut q_group_state,
                    &mut q_group_stack,
                    false,
                )?;
                k += 1;
                continue;
            }

            just_rewinded = false;
            let orig_k = k;

            if range_min != 0 {
                k += range_min as usize;
                if tokens[k - 1].kind == REMIMU_KIND_OR {
                    k = add_offset(k - 1, tokens[k - 1].pair_offset)?;
                } else if matches!(tokens[k - 1].kind, REMIMU_KIND_OPEN | REMIMU_KIND_NCOPEN) {
                    k = k - 1 + tokens[k - 1].mask[15] as usize;
                }

                if tokens[k].kind == REMIMU_KIND_END {
                    return Err(-3);
                }

                if tokens[k].kind == REMIMU_KIND_CLOSE {
                    let group = tokens[k].mask[0] as usize;
                    if tokens[k].count_lo == 0 || q_group_accepts_zero[group] != 0 {
                        q_group_state[group] = 0;
                        if (tokens[k].mode & REMIMU_MODE_LAZY) == 0 {
                            q_group_stack[group] = 0;
                        }
                        k += 1;
                        continue;
                    }

                    k = rewind_or_abort(
                        &rewind_stack,
                        &mut stack_n,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                    )?;
                    continue;
                }
            }

            let k_diff = k - orig_k;
            range_min = (k_diff + 1) as u64;
            save_rewind_state(
                &mut rewind_stack,
                &mut stack_n,
                tokens,
                orig_k,
                i,
                range_min,
                range_max,
                &mut q_group_state,
                &mut q_group_stack,
                false,
            )?;
            k = orig_k + 1;
            continue;
        } else if token.kind == REMIMU_KIND_CLOSE {
            if token.count_lo == 1 && token.count_hi == 2 {
                let cap_index = q_group_cap_index[token.mask[0] as usize];
                if cap_index != u16::MAX {
                    save_rewind_state(
                        &mut rewind_stack,
                        &mut stack_n,
                        tokens,
                        k,
                        i,
                        range_min,
                        range_max,
                        &mut q_group_state,
                        &mut q_group_stack,
                        true,
                    )?;
                }
                k += 1;
                continue;
            }

            if !just_rewinded {
                let group = token.mask[0] as usize;
                let prev = q_group_stack[group];
                range_max = (token.count_hi as u64).wrapping_sub(1);
                range_min = if q_group_accepts_zero[group] != 0 {
                    0
                } else {
                    token.count_lo as u64
                };

                if q_group_state[group] as u64 + 1 < range_min {
                    q_group_state[group] += 1;
                    save_rewind_state(
                        &mut rewind_stack,
                        &mut stack_n,
                        tokens,
                        k,
                        i,
                        range_min,
                        range_max,
                        &mut q_group_state,
                        &mut q_group_stack,
                        false,
                    )?;
                    k = add_offset(k, token.pair_offset)?;
                    continue;
                } else if token.count_hi != 0 && q_group_state[group] as u64 + 1 > range_max {
                    range_max = range_max.wrapping_sub(1);
                    k = rewind_or_abort(
                        &rewind_stack,
                        &mut stack_n,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                    )?;
                    continue;
                }

                let mut force_zero = false;
                if prev != 0 && rewind_stack[prev as usize].i > i {
                    let mut n = stack_n.saturating_sub(1);
                    let open_k = add_offset(k, token.pair_offset)?;
                    while n > 0 && rewind_stack[n].k as usize != open_k {
                        n -= 1;
                    }
                    if n > 0 && rewind_stack[n].i == i {
                        force_zero = true;
                    }
                }

                if force_zero || (prev != 0 && rewind_stack[prev as usize].i == i) {
                    q_group_accepts_zero[group] = 1;
                    k = rewind_or_abort(
                        &rewind_stack,
                        &mut stack_n,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                    )?;
                    continue;
                } else if (token.mode & REMIMU_MODE_LAZY) != 0 {
                    q_group_state[group] += 1;
                    save_rewind_state(
                        &mut rewind_stack,
                        &mut stack_n,
                        tokens,
                        k,
                        i,
                        range_min,
                        range_max,
                        &mut q_group_state,
                        &mut q_group_stack,
                        false,
                    )?;
                    q_group_state[group] = 0;
                    k += 1;
                    continue;
                } else {
                    if (token.mode & REMIMU_MODE_POSSESSIVE) != 0 {
                        let mut k2 = k;
                        if q_group_state[group] == 0 {
                            k2 = add_offset(k, token.pair_offset)?;
                        }
                        if stack_n == 0 {
                            return Err(-1);
                        }
                        stack_n -= 1;
                        while stack_n > 0 && rewind_stack[stack_n].k as usize != k2 {
                            stack_n -= 1;
                        }
                        if stack_n == 0 {
                            return Err(-1);
                        }
                    }

                    let open_group = tokens[add_offset(k, token.pair_offset)?].mask[0] as usize;
                    if q_group_state[open_group] < i as u32 {
                        q_group_state[group] += 1;
                        save_rewind_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            tokens,
                            k,
                            i,
                            range_min,
                            range_max,
                            &mut q_group_state,
                            &mut q_group_stack,
                            false,
                        )?;
                        k = add_offset(k, token.pair_offset)?;
                        continue;
                    }
                    k += 1;
                    continue;
                }
            }

            just_rewinded = false;
            let group = token.mask[0] as usize;
            if (token.mode & REMIMU_MODE_LAZY) != 0 {
                save_rewind_state(
                    &mut rewind_stack,
                    &mut stack_n,
                    tokens,
                    k,
                    i,
                    range_min,
                    range_max,
                    &mut q_group_state,
                    &mut q_group_stack,
                    true,
                )?;
                q_group_stack[group] = stack_n as u32;
                k = add_offset(k, token.pair_offset)?;
                continue;
            }

            if q_group_state[group] < range_min as u32 && q_group_accepts_zero[group] == 0 {
                k = rewind_or_abort(
                    &rewind_stack,
                    &mut stack_n,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                )?;
                continue;
            }

            q_group_state[group] = 0;
            let cap_index = q_group_cap_index[group];
            if cap_index != u16::MAX {
                save_rewind_state(
                    &mut rewind_stack,
                    &mut stack_n,
                    tokens,
                    k,
                    i,
                    range_min,
                    range_max,
                    &mut q_group_state,
                    &mut q_group_stack,
                    true,
                )?;
            }
            k += 1;
            continue;
        } else if token.kind == REMIMU_KIND_OR {
            k += token.pair_offset as usize + 1;
            continue;
        } else if token.kind == REMIMU_KIND_NORMAL {
            if !just_rewinded {
                let mut n = 0u64;
                let old_i = i;
                while n < token.count_lo as u64
                    && text_byte(bytes, i) != 0
                    && token.check_mask(text_byte(bytes, i))
                {
                    i += 1;
                    n += 1;
                }
                if n < token.count_lo as u64 {
                    i = old_i;
                    k = rewind_or_abort(
                        &rewind_stack,
                        &mut stack_n,
                        tokens,
                        &mut q_group_state,
                        &mut q_group_stack,
                        &mut i,
                        &mut range_min,
                        &mut range_max,
                        &mut just_rewinded,
                    )?;
                    continue;
                }

                if (token.mode & REMIMU_MODE_LAZY) != 0 {
                    range_min = n;
                    range_max = (token.count_hi as u64).wrapping_sub(1);
                    save_rewind_state(
                        &mut rewind_stack,
                        &mut stack_n,
                        tokens,
                        k,
                        i,
                        range_min,
                        range_max,
                        &mut q_group_state,
                        &mut q_group_stack,
                        false,
                    )?;
                } else {
                    let mut limit = token.count_hi as u64;
                    if limit == 0 {
                        limit = !limit;
                    }
                    range_min = n;
                    while text_byte(bytes, i) != 0
                        && token.check_mask(text_byte(bytes, i))
                        && n + 1 < limit
                    {
                        i += 1;
                        n += 1;
                    }
                    range_max = n;
                    if (token.mode & REMIMU_MODE_POSSESSIVE) == 0 {
                        save_rewind_state(
                            &mut rewind_stack,
                            &mut stack_n,
                            tokens,
                            k,
                            i,
                            range_min,
                            range_max,
                            &mut q_group_state,
                            &mut q_group_stack,
                            false,
                        )?;
                    }
                }
                k += 1;
                continue;
            }

            just_rewinded = false;
            if (token.mode & REMIMU_MODE_LAZY) != 0 {
                let mut limit = range_max;
                if limit == 0 {
                    limit = !limit;
                }
                if token.check_mask(text_byte(bytes, i))
                    && text_byte(bytes, i) != 0
                    && range_min < limit
                {
                    i += 1;
                    range_min += 1;
                    save_rewind_state(
                        &mut rewind_stack,
                        &mut stack_n,
                        tokens,
                        k,
                        i,
                        range_min,
                        range_max,
                        &mut q_group_state,
                        &mut q_group_stack,
                        false,
                    )?;
                    k += 1;
                    continue;
                }

                k = rewind_or_abort(
                    &rewind_stack,
                    &mut stack_n,
                    tokens,
                    &mut q_group_state,
                    &mut q_group_stack,
                    &mut i,
                    &mut range_min,
                    &mut range_max,
                    &mut just_rewinded,
                )?;
                continue;
            }

            if range_max > range_min {
                i = i.saturating_sub(1);
                range_max -= 1;
                save_rewind_state(
                    &mut rewind_stack,
                    &mut stack_n,
                    tokens,
                    k,
                    i,
                    range_min,
                    range_max,
                    &mut q_group_state,
                    &mut q_group_stack,
                    false,
                )?;
                k += 1;
                continue;
            }

            k = rewind_or_abort(
                &rewind_stack,
                &mut stack_n,
                tokens,
                &mut q_group_state,
                &mut q_group_stack,
                &mut i,
                &mut range_min,
                &mut range_max,
                &mut just_rewinded,
            )?;
            continue;
        } else {
            return Err(-3);
        }
    }

    if caps != 0 {
        for state in rewind_stack.iter().take(stack_n) {
            let kind = tokens[state.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[state.k as usize].mask[0] as usize];
                if cap_index == u16::MAX {
                    continue;
                }
                let cap_index = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[cap_index] = state.i as i64;
                } else if cap_pos[cap_index] >= 0 {
                    cap_span[cap_index] = state.i as i64 - cap_pos[cap_index];
                }
            }
        }

        for n in 0..caps {
            if cap_span[n] == -1 {
                cap_pos[n] = -1;
            }
        }
    }

    Ok(i as usize)
}

pub fn regex_match(
    tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Option<usize> {
    regex_match_impl(tokens, text, start_i, cap_slots, cap_pos, cap_span).ok()
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];

    for (k, token) in tokens.iter().enumerate() {
        let mode = match token.mode {
            0 => "GREEDY",
            1 => "POSSESS",
            2 => "LAZY",
            _ => "UNKNOWN",
        };

        print!("{}\t{}\t", kind_to_str[token.kind as usize], mode);

        let mut c_old: i32 = -1;
        if token.kind == REMIMU_KIND_NORMAL {
            for c in 0..256u16 {
                let c8 = c as u8;
                if token.check_mask(c8) {
                    if c_old == -1 {
                        c_old = c as i32;
                    }
                } else if c_old != -1 {
                    let print_c = |value: u8| {
                        if (0x20..=0x7E).contains(&value) {
                            print!("{}", value as char);
                        } else {
                            print!("\\x{value:02x}");
                        }
                    };

                    if c as i32 - 1 == c_old {
                        print_c(c_old as u8);
                    } else if c as i32 - 2 == c_old {
                        print_c(c_old as u8);
                        print_c((c_old + 1) as u8);
                    } else {
                        print_c(c_old as u8);
                        print!("-");
                        print_c((c - 1) as u8);
                    }
                    c_old = -1;
                }
            }
        }

        println!(
            "\t{{{},{}}}\t({})",
            token.count_lo,
            token.count_hi.wrapping_sub(1),
            token.pair_offset
        );

        if token.kind == REMIMU_KIND_END {
            let _ = k;
            break;
        }
    }
}
