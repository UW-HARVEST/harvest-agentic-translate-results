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
        for n in 0..16 {
            self.mask[n] = !self.mask[n];
        }
    }
    pub fn check_mask(&self, byte: u8) -> bool {
        (self.mask[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
    }
    /// Pushes the token to the provided vector while ensuring proper constraints.
    pub fn push_to_vec(&mut self, tokens: &mut Vec<RegexToken>, max_len: usize) -> Result<(), i32> {
        let k = tokens.len();
        if k == 0 || tokens[k - 1].kind != self.kind || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND) {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
                self.mode &= !REMIMU_MODE_INVERTED;
            }
            if k >= max_len {
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
fn build_shorthand_mask(c: u8) -> [u16; 16] {
    let is_upper = c <= b'Z';
    let mut m = [0u16; 16];
    let lc = if is_upper { c + 0x20 } else { c };
    if lc == b'd' || lc == b'w' { m[3] |= 0x03FF; }
    if lc == b's' { m[0] |= 0x3E00; m[2] |= 1; }
    if lc == b'w' { m[4] |= 0xFFFE; m[5] |= 0x87FF; m[6] |= 0xFFFE; m[7] |= 0x07FF; }
    if is_upper { for n in 0..16 { m[n] = !m[n]; } }
    m
}

pub fn regex_parse(pattern: &str, tokens: &mut Vec<RegexToken>, token_count: &mut i16, flags: i32) -> Result<(), i32> {
    let tokens_len = *token_count as usize;
    let pat = pattern.as_bytes();
    let pattern_len = pat.len();

    tokens.clear();

    #[derive(Debug, Clone, Copy, PartialEq)]
    enum State { Normal, Quant, Mode, CCInit, CCNormal, CCRange }
    let mut state = State::Normal;
    let mut esc_state = 0;
    let mut char_class_mem: i32 = -1;
    let mut token = RegexToken::default();

    // start with invisible group
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;
    let mut i = 0usize;

    while i < pattern_len {
        let c = pat[i];

        if state == State::Quant {
            state = State::Mode;
            if c == b'?' { token.count_lo = 0; token.count_hi = 2; i += 1; continue; }
            else if c == b'+' { token.count_lo = 1; token.count_hi = 0; i += 1; continue; }
            else if c == b'*' { token.count_lo = 0; token.count_hi = 0; i += 1; continue; }
            else if c == b'{' {
                if i + 1 >= pattern_len || pat[i + 1] < b'0' || pat[i + 1] > b'9' {
                    state = State::Normal;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while i < pattern_len && pat[i] >= b'0' && pat[i] <= b'9' {
                        val = val * 10 + (pat[i] - b'0') as u32;
                        if val > 0xFFFF { return Err(-1); }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = val as u16 + 1;
                    if i < pattern_len && pat[i] == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if i < pattern_len && pat[i] >= b'0' && pat[i] <= b'9' {
                            let mut val2: u32 = 0;
                            while i < pattern_len && pat[i] >= b'0' && pat[i] <= b'9' {
                                val2 = val2 * 10 + (pat[i] - b'0') as u32;
                                if val2 > 0xFFFF { return Err(-1); }
                                i += 1;
                            }
                            if val2 < val { return Err(-1); }
                            token.count_hi = val2 as u16 + 1;
                        }
                    }
                    if i < pattern_len && pat[i] == b'}' { i += 1; continue; }
                    else { return Err(-1); }
                }
            }
        }

        if state == State::Mode {
            state = State::Normal;
            if c == b'?' { token.mode |= REMIMU_MODE_LAZY; i += 1; continue; }
            else if c == b'+' { token.mode |= REMIMU_MODE_POSSESSIVE; i += 1; continue; }
        }

        if state == State::Normal {
            if esc_state == 1 {
                esc_state = 0;
                match c {
                    b'n' => token.set_mask(b'\n'),
                    b'r' => token.set_mask(b'\r'),
                    b't' => token.set_mask(b'\t'),
                    b'v' => token.set_mask(0x0B),
                    b'f' => token.set_mask(0x0C),
                    b'x' => {
                        if i + 2 >= pattern_len { return Err(-1); }
                        let mut n0 = pat[i + 1];
                        let mut n1 = pat[i + 1]; // C bug replicated: both read from i+1
                        if n0 < b'0' || n0 > b'f' || n1 < b'0' || n1 > b'f' ||
                           (n0 > b'9' && n0 < b'A') || (n1 > b'9' && n1 < b'A') { return Err(-1); }
                        if n0 > b'F' { n0 -= 0x20; }
                        if n1 > b'F' { n1 -= 0x20; }
                        if n0 >= b'A' { n0 -= b'A' - 10; }
                        if n1 >= b'A' { n1 -= b'A' - 10; }
                        n0 -= b'0';
                        n1 -= b'0';
                        token.set_mask((n1 << 4) | n0);
                        i += 2;
                    }
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$' |
                    b'*' | b'+' | b'?' | b':' | b'.' | b'/' | b'\\' => {
                        token.set_mask(c);
                        state = State::Quant;
                    }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        let m = build_shorthand_mask(c);
                        for n in 0..16 { token.mask[n] |= m[n]; }
                        token.kind = REMIMU_KIND_NORMAL;
                        state = State::Quant;
                    }
                    b'b' => { token.kind = REMIMU_KIND_BOUND; state = State::Normal; }
                    b'B' => { token.kind = REMIMU_KIND_NBOUND; state = State::Normal; }
                    _ => return Err(-1),
                }
            } else {
                token.push_to_vec(tokens, tokens_len)?;
                match c {
                    b'\\' => { esc_state = 1; }
                    b'[' => {
                        state = State::CCInit;
                        char_class_mem = -1;
                        token.kind = REMIMU_KIND_NORMAL;
                        if i + 1 < pattern_len && pat[i + 1] == b'^' {
                            token.mode |= REMIMU_MODE_INVERTED;
                            i += 1;
                        }
                    }
                    b'(' => {
                        paren_count += 1;
                        state = State::Normal;
                        token.kind = REMIMU_KIND_OPEN;
                        token.count_lo = 0;
                        token.count_hi = 1;
                        if i + 2 < pattern_len && pat[i + 1] == b'?' && pat[i + 2] == b':' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            i += 2;
                        } else if i + 2 < pattern_len && pat[i + 1] == b'?' && pat[i + 2] == b'>' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.push_to_vec(tokens, tokens_len)?;
                            state = State::Normal;
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.count_lo = 1;
                            token.count_hi = 2;
                            i += 2;
                        }
                    }
                    b')' => {
                        paren_count -= 1;
                        let k = tokens.len();
                        if paren_count < 0 || k == 0 { return Err(-1); }
                        token.kind = REMIMU_KIND_CLOSE;
                        state = State::Quant;
                        let mut balance: i32 = 0;
                        let mut found: i64 = -1;
                        for l in (0..k).rev() {
                            if tokens[l].kind == REMIMU_KIND_NCOPEN || tokens[l].kind == REMIMU_KIND_OPEN {
                                if balance == 0 { found = l as i64; break; }
                                else { balance -= 1; }
                            } else if tokens[l].kind == REMIMU_KIND_CLOSE { balance += 1; }
                        }
                        if found == -1 { return Err(-1); }
                        let diff = k as i64 - found;
                        if diff > 32767 { return Err(-1); }
                        token.pair_offset = -diff as i16;
                        tokens[found as usize].pair_offset = diff as i16;
                        if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                            token.push_to_vec(tokens, tokens_len)?;
                            token.kind = REMIMU_KIND_CLOSE;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.pair_offset = (-diff - 2) as i16;
                            tokens[found as usize - 1].pair_offset = (diff + 2) as i16;
                        }
                    }
                    b'?' | b'+' | b'*' | b'{' => return Err(-1),
                    b'.' => {
                        for n in 0..16 { token.mask[n] = 0xFFFF; }
                        if flags & REMIMU_FLAG_DOT_NO_NEWLINES != 0 {
                            token.mask[1] ^= 0x04; // \n
                            token.mask[1] ^= 0x20; // \r
                        }
                        state = State::Quant;
                    }
                    b'^' => { token.kind = REMIMU_KIND_CARET; state = State::Normal; }
                    b'$' => { token.kind = REMIMU_KIND_DOLLAR; state = State::Normal; }
                    b'|' => { token.kind = REMIMU_KIND_OR; state = State::Normal; }
                    _ => { token.set_mask(c); state = State::Quant; }
                }
            }
        } else if state == State::CCInit || state == State::CCNormal || state == State::CCRange {
            if c == b'\\' && esc_state == 0 { esc_state = 1; i += 1; continue; }
            let mut esc_c: u8 = 0;
            if esc_state == 1 {
                esc_state = 0;
                match c {
                    b'n' => esc_c = b'\n',
                    b'r' => esc_c = b'\r',
                    b't' => esc_c = b'\t',
                    b'v' => esc_c = 0x0B,
                    b'f' => esc_c = 0x0C,
                    b'x' => {
                        if i + 2 >= pattern_len { return Err(-1); }
                        let mut n0 = pat[i + 1];
                        let mut n1 = pat[i + 1];
                        if n0 < b'0' || n0 > b'f' || n1 < b'0' || n1 > b'f' ||
                           (n0 > b'9' && n0 < b'A') || (n1 > b'9' && n1 < b'A') { return Err(-1); }
                        if n0 > b'F' { n0 -= 0x20; }
                        if n1 > b'F' { n1 -= 0x20; }
                        if n0 >= b'A' { n0 -= b'A' - 10; }
                        if n1 >= b'A' { n1 -= b'A' - 10; }
                        n0 -= b'0';
                        n1 -= b'0';
                        esc_c = (n1 << 4) | n0;
                        i += 2;
                    }
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' | b'|' | b'^' | b'$' |
                    b'*' | b'+' | b'?' | b':' | b'.' | b'/' | b'\\' => { esc_c = c; }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        if state == State::CCRange { return Err(-1); }
                        let m = build_shorthand_mask(c);
                        for n in 0..16 { token.mask[n] |= m[n]; }
                        char_class_mem = -1;
                        i += 1; continue;
                    }
                    _ => return Err(-1),
                }
            }
            let effective_c = if esc_c != 0 { esc_c } else { c };
            if state == State::CCInit {
                char_class_mem = effective_c as i32;
                token.set_mask(effective_c);
                state = State::CCNormal;
            } else if state == State::CCNormal {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    state = State::Quant;
                    i += 1; continue;
                } else if c == b'-' && esc_c == 0 && char_class_mem >= 0 {
                    state = State::CCRange;
                    i += 1; continue;
                } else {
                    char_class_mem = effective_c as i32;
                    token.set_mask(effective_c);
                    state = State::CCNormal;
                }
            } else if state == State::CCRange {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    token.set_mask(b'-');
                    state = State::Quant;
                    i += 1; continue;
                } else {
                    if char_class_mem == -1 { return Err(-1); }
                    if (effective_c as i32) < char_class_mem { return Err(-1); }
                    let mut j = effective_c;
                    while j > char_class_mem as u8 {
                        token.set_mask(j);
                        j -= 1;
                    }
                    state = State::CCNormal;
                    char_class_mem = -1;
                }
            }
        }
        i += 1;
    }

    if paren_count > 0 { return Err(-1); }
    if esc_state != 0 { return Err(-1); }
    if state == State::CCInit || state == State::CCNormal || state == State::CCRange { return Err(-1); }

    token.push_to_vec(tokens, tokens_len)?;

    // add invisible close
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, tokens_len)?;

    // add end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, tokens_len)?;

    let k = tokens.len();
    tokens[0].pair_offset = (k as i16) - 2;
    tokens[k - 2].pair_offset = -((k as i16) - 2);

    *token_count = k as i16;

    // copy quantifiers from )s to (s, smuggle group index
    let mut n: u64 = 0;
    for k2 in 0..k {
        if tokens[k2].kind == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16;
            n += 1;
            let k3 = (k2 as i32 + tokens[k2].pair_offset as i32) as usize;
            tokens[k3].count_lo = tokens[k2].count_lo;
            tokens[k3].count_hi = tokens[k2].count_hi;
            tokens[k3].mask[0] = n as u16;
            n += 1;
            tokens[k3].mode = tokens[k2].mode;
            if n > 1024 { return Err(-1); }
        } else if tokens[k2].kind == REMIMU_KIND_OR || tokens[k2].kind == REMIMU_KIND_OPEN || tokens[k2].kind == REMIMU_KIND_NCOPEN {
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            for l in (k2 + 1)..k {
                if tokens[l].kind == REMIMU_KIND_OR && balance == 0 { found = l as i64; break; }
                else if tokens[l].kind == REMIMU_KIND_CLOSE {
                    if balance == 0 { found = l as i64; break; }
                    else { balance -= 1; }
                } else if tokens[l].kind == REMIMU_KIND_NCOPEN || tokens[l].kind == REMIMU_KIND_OPEN {
                    balance += 1;
                }
            }
            if found == -1 { return Err(-1); }
            let diff = found - k2 as i64;
            if diff > 32767 { return Err(-1); }
            if tokens[k2].kind == REMIMU_KIND_OR {
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
pub fn regex_match(tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64]) -> Option<usize> {
    let text = text.as_bytes();
    let stack_size_max: usize = 1024;
    let aux_stats_size: usize = 1024;
    let cap_slots = (cap_slots as usize).min(aux_stats_size);

    let mut q_group_accepts_zero = vec![0u8; aux_stats_size];
    let mut q_group_state = vec![0u32; aux_stats_size];
    let mut q_group_stack = vec![0u32; aux_stats_size];
    let mut q_group_cap_index = vec![0xFFFFu16; aux_stats_size];

    let mut k: usize = 0;
    let mut caps: usize = 0;

    while tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            q_group_cap_index[tokens[k].mask[0] as usize] = caps as u16;
            let close_k = (k as i32 + tokens[k].pair_offset as i32) as usize;
            q_group_cap_index[tokens[close_k].mask[0] as usize] = caps as u16;
            cap_pos[caps] = -1;
            cap_span[caps] = -1;
            caps += 1;
        }
        k += 1;
        if tokens[k].kind == REMIMU_KIND_CLOSE || tokens[k].kind == REMIMU_KIND_OPEN || tokens[k].kind == REMIMU_KIND_NCOPEN {
            let idx = tokens[k].mask[0] as usize;
            if idx >= aux_stats_size { return None; } // OOM
            q_group_state[idx] = 0;
            q_group_stack[idx] = 0;
            q_group_accepts_zero[idx] = 0;
        }
    }
    let tokens_len = k;

    let mut rewind_stack: Vec<RegexMatcherState> = Vec::with_capacity(stack_size_max);
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    let check_mask = |k: usize, byte: u8| -> bool {
        (tokens[k].mask[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
    };

    let w_mask: [u64; 16] = {
        let mut m = [0u64; 16];
        m[3] = 0x03FF; m[4] = 0xFFFE; m[5] = 0x87FF; m[6] = 0xFFFE; m[7] = 0x07FF;
        m
    };
    let check_is_w = |byte: u8| -> bool {
        (w_mask[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
    };

    let text_at = |idx: u64| -> u8 {
        if (idx as usize) < text.len() { text[idx as usize] } else { 0 }
    };

    macro_rules! rewind_do_save_raw {
        ($kk:expr, $is_dummy:expr) => {{
            if stack_n >= stack_size_max { return None; } // OOM
            let mut s = RegexMatcherState::new($kk as u32, i);
            s.range_min = range_min;
            s.range_max = range_max;
            s.prev = 0;
            if $is_dummy { s.prev = 0xFAC7; }
            else if tokens[$kk].kind == REMIMU_KIND_CLOSE {
                let idx = tokens[$kk].mask[0] as usize;
                s.group_state = q_group_state[idx];
                s.prev = q_group_stack[idx];
                q_group_stack[idx] = stack_n as u32;
            }
            if stack_n < rewind_stack.len() { rewind_stack[stack_n] = s; }
            else { rewind_stack.push(s); }
            stack_n += 1;
        }};
    }

    macro_rules! rewind_or_abort {
        () => {{
            if stack_n == 0 { return None; }
            stack_n -= 1;
            while stack_n > 0 && rewind_stack[stack_n].prev == 0xFAC7 { stack_n -= 1; }
            just_rewinded = true;
            range_min = rewind_stack[stack_n].range_min;
            range_max = rewind_stack[stack_n].range_max;
            i = rewind_stack[stack_n].i;
            k = rewind_stack[stack_n].k as usize;
            if tokens[k].kind == REMIMU_KIND_CLOSE {
                let idx = tokens[k].mask[0] as usize;
                q_group_state[idx] = rewind_stack[stack_n].group_state;
                q_group_stack[idx] = rewind_stack[stack_n].prev;
            }
            k -= 1; // because of k += 1 in loop
        }};
    }

    k = 0;
    while k < tokens_len {
        if tokens[k].kind == REMIMU_KIND_CARET {
            if i != 0 { rewind_or_abort!(); }
            k += 1; continue;
        } else if tokens[k].kind == REMIMU_KIND_DOLLAR {
            if text_at(i) != 0 { rewind_or_abort!(); }
            k += 1; continue;
        } else if tokens[k].kind == REMIMU_KIND_BOUND {
            if i == 0 && !check_is_w(text_at(i)) { rewind_or_abort!(); }
            else if i != 0 && text_at(i) == 0 && !check_is_w(text_at(i - 1)) { rewind_or_abort!(); }
            else if i != 0 && text_at(i) != 0 && check_is_w(text_at(i - 1)) == check_is_w(text_at(i)) { rewind_or_abort!(); }
        } else if tokens[k].kind == REMIMU_KIND_NBOUND {
            if i == 0 && check_is_w(text_at(i)) { rewind_or_abort!(); }
            else if i != 0 && text_at(i) == 0 && check_is_w(text_at(i - 1)) { rewind_or_abort!(); }
            else if i != 0 && text_at(i) != 0 && check_is_w(text_at(i - 1)) != check_is_w(text_at(i)) { rewind_or_abort!(); }
        } else {
            // deliberately unmatchable token
            if tokens[k].count_hi == 1 {
                if tokens[k].kind == REMIMU_KIND_OPEN || tokens[k].kind == REMIMU_KIND_NCOPEN {
                    k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                } else {
                    k += 1;
                }
                k += 1; continue;
            }

            if tokens[k].kind == REMIMU_KIND_OPEN || tokens[k].kind == REMIMU_KIND_NCOPEN {
                if !just_rewinded {
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 && (tokens[k].count_lo == 0 || q_group_accepts_zero[tokens[(k as i32 + tokens[k].pair_offset as i32) as usize].mask[0] as usize] != 0) {
                        range_min = 0;
                        range_max = 0;
                        rewind_do_save_raw!(k, false);
                        k = (k as i32 + tokens[k].pair_offset as i32) as usize; // past matching )
                    } else {
                        range_min = 1;
                        range_max = 0;
                        rewind_do_save_raw!(k, false);
                    }
                } else {
                    just_rewinded = false;
                    let orig_k = k;

                    if range_min != 0 {
                        k += range_min as usize;
                        if tokens[k - 1].kind == REMIMU_KIND_OR {
                            k = (k as i64 + tokens[k - 1].pair_offset as i64 - 1) as usize;
                        } else if tokens[k - 1].kind == REMIMU_KIND_OPEN || tokens[k - 1].kind == REMIMU_KIND_NCOPEN {
                            k = (k as i64 + tokens[k - 1].mask[15] as i64 - 1) as usize;
                        }

                        if tokens[k].kind == REMIMU_KIND_END { return None; } // -3 invalid

                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            if tokens[k].count_lo == 0 || q_group_accepts_zero[tokens[k].mask[0] as usize] != 0 {
                                q_group_state[tokens[k].mask[0] as usize] = 0;
                                if (tokens[k].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[tokens[k].mask[0] as usize] = 0;
                                }
                                k += 1; continue;
                            } else {
                                rewind_or_abort!();
                                k += 1; continue;
                            }
                        }
                        // assert tokens[k].kind == REMIMU_KIND_OR
                    }

                    let k_diff = k as i64 - orig_k as i64;
                    range_min = (k_diff + 1) as u64;
                    rewind_do_save_raw!(k - k_diff as usize, false);
                }
            } else if tokens[k].kind == REMIMU_KIND_CLOSE {
                // unquantified
                if tokens[k].count_lo == 1 && tokens[k].count_hi == 2 {
                    let cap_index = q_group_cap_index[tokens[k].mask[0] as usize];
                    if cap_index != 0xFFFF { rewind_do_save_raw!(k, true); }
                } else {
                    // quantified
                    if !just_rewinded {
                        let prev = q_group_stack[tokens[k].mask[0] as usize];
                        range_max = tokens[k].count_hi as u64;
                        if range_max > 0 { range_max -= 1; }
                        range_min = if q_group_accepts_zero[tokens[k].mask[0] as usize] != 0 { 0 } else { tokens[k].count_lo as u64 };

                        // minimum not yet met
                        if (q_group_state[tokens[k].mask[0] as usize] as u64 + 1) < range_min {
                            q_group_state[tokens[k].mask[0] as usize] += 1;
                            rewind_do_save_raw!(k, false);
                            k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                            // k -= 1 then k += 1 cancel out, but we need to actually hit the group node
                            // so don't increment
                            continue;
                        }
                        // maximum exceeded
                        else if tokens[k].count_hi != 0 && (q_group_state[tokens[k].mask[0] as usize] as u64 + 1) > range_max {
                            range_max -= 1;
                            rewind_or_abort!();
                            k += 1; continue;
                        }

                        // detect zero-length matches
                        let mut force_zero = false;
                        if prev != 0 && rewind_stack[prev as usize].i > i {
                            let mut n = stack_n - 1;
                            let open_k = (k as i32 + tokens[k].pair_offset as i32) as u32;
                            while n > 0 && rewind_stack[n].k != open_k { n -= 1; }
                            if rewind_stack[n].i == i { force_zero = true; }
                        }

                        if force_zero || (prev != 0 && rewind_stack[prev as usize].i == i) {
                            q_group_accepts_zero[tokens[k].mask[0] as usize] = 1;
                            rewind_or_abort!();
                        } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            q_group_state[tokens[k].mask[0] as usize] += 1;
                            rewind_do_save_raw!(k, false);
                            q_group_state[tokens[k].mask[0] as usize] = 0;
                        } else {
                            // greedy
                            if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let mut k2 = k;
                                if q_group_state[tokens[k].mask[0] as usize] == 0 {
                                    k2 = (k as i32 + tokens[k].pair_offset as i32) as usize;
                                }
                                if stack_n == 0 { return None; }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].k != k2 as u32 { stack_n -= 1; }
                                if stack_n == 0 { return None; }
                            }
                            let open_idx = tokens[(k as i32 + tokens[k].pair_offset as i32) as usize].mask[0] as usize;
                            if (q_group_state[open_idx] as u64) < i {
                                q_group_state[tokens[k].mask[0] as usize] += 1;
                                rewind_do_save_raw!(k, false);
                                k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                                continue;
                            }
                        }
                    } else {
                        just_rewinded = false;
                        if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            rewind_do_save_raw!(k, true);
                            q_group_stack[tokens[k].mask[0] as usize] = stack_n as u32;
                            k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                            continue;
                        } else {
                            if q_group_state[tokens[k].mask[0] as usize] as u64 > range_min && q_group_accepts_zero[tokens[k].mask[0] as usize] == 0 {
                                // Wait - C code checks < not >. Let me re-check.
                                // C: if (q_group_state[tokens[k].mask[0]] < range_min && !q_group_accepts_zero[tokens[k].mask[0]])
                                // So: rewind if state < range_min and not accepts_zero
                                // Actually let me fix this:
                            }
                            if (q_group_state[tokens[k].mask[0] as usize] as u64) < range_min && q_group_accepts_zero[tokens[k].mask[0] as usize] == 0 {
                                rewind_or_abort!();
                            } else {
                                q_group_state[tokens[k].mask[0] as usize] = 0;
                                let cap_index = q_group_cap_index[tokens[k].mask[0] as usize];
                                if cap_index != 0xFFFF { rewind_do_save_raw!(k, true); }
                            }
                        }
                    }
                }
            } else if tokens[k].kind == REMIMU_KIND_OR {
                k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                // k -= 1 then k += 1 cancel, but we need k to point to pair_offset target - 1
                // Actually in C: k += tokens[k].pair_offset; k -= 1; then the for loop does k++
                // So net effect: k = k + pair_offset
                // But we already set k = k + pair_offset above, and the loop will do k += 1
                // So we need to not increment. Let me just continue without incrementing.
                continue;
            } else if tokens[k].kind == REMIMU_KIND_NORMAL {
                if !just_rewinded {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k].count_lo as u64 && text_at(i) != 0 && check_mask(k, text_at(i)) {
                        i += 1;
                        n += 1;
                    }
                    if n < tokens[k].count_lo as u64 {
                        i = old_i;
                        rewind_or_abort!();
                        k += 1; continue;
                    }
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        range_min = n;
                        range_max = tokens[k].count_hi as u64 - 1;
                        rewind_do_save_raw!(k, false);
                    } else {
                        let mut limit_val: u64 = tokens[k].count_hi as u64;
                        if limit_val == 0 { limit_val = u64::MAX; }
                        range_min = n;
                        while text_at(i) != 0 && check_mask(k, text_at(i)) && n + 1 < limit_val {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            rewind_do_save_raw!(k, false);
                        }
                    }
                } else {
                    just_rewinded = false;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        let mut limit_val = range_max;
                        if limit_val == 0 { limit_val = u64::MAX; }
                        if check_mask(k, text_at(i)) && text_at(i) != 0 && range_min < limit_val {
                            i += 1;
                            range_min += 1;
                            rewind_do_save_raw!(k, false);
                        } else {
                            rewind_or_abort!();
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            rewind_do_save_raw!(k, false);
                        } else {
                            rewind_or_abort!();
                        }
                    }
                }
            }
        }
        k += 1;
    }

    // Process captures
    if caps != 0 {
        for n in 0..stack_n {
            let s = &rewind_stack[n];
            let sk = s.k as usize;
            let kind = tokens[sk].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[sk].mask[0] as usize];
                if cap_index == 0xFFFF { continue; }
                let ci = cap_index as usize;
                if tokens[sk].kind == REMIMU_KIND_OPEN {
                    cap_pos[ci] = s.i as i64;
                } else if cap_pos[ci] >= 0 {
                    cap_span[ci] = s.i as i64 - cap_pos[ci];
                }
            }
        }
        for n in 0..caps {
            if cap_span[n] == -1 { cap_pos[n] = -1; }
        }
    }

    Some(i as usize)
}
pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = ["NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END"];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];
    let mut k = 0;
    loop {
        let kind_s = kind_to_str[tokens[k].kind as usize];
        let mode_s = mode_to_str[tokens[k].mode as usize];
        print!("{}\t{}\t", kind_s, mode_s);

        if tokens[k].kind == REMIMU_KIND_NORMAL {
            let mut c_old: i32 = -1;
            for c in 0..256u16 {
                let check = (tokens[k].mask[(c >> 4) as usize] & (1 << (c & 0xF))) != 0;
                if check {
                    if c_old == -1 { c_old = c as i32; }
                } else if c_old != -1 {
                    let end = c as i32 - 1;
                    if end == c_old {
                        print_c_smart(c_old as u8);
                    } else if end == c_old + 1 {
                        print_c_smart(c_old as u8);
                        print_c_smart((c_old + 1) as u8);
                    } else {
                        print_c_smart(c_old as u8);
                        print!("-");
                        print_c_smart(end as u8);
                    }
                    c_old = -1;
                }
            }
        }

        let hi = tokens[k].count_hi as i32 - 1;
        println!("\t{{{},{}}}\t({})", tokens[k].count_lo, hi, tokens[k].pair_offset);

        if tokens[k].kind == REMIMU_KIND_END { break; }
        k += 1;
    }
}

fn print_c_smart(c: u8) {
    if c >= 0x20 && c <= 0x7E { print!("{}", c as char); }
    else { print!("\\x{:02x}", c); }
}