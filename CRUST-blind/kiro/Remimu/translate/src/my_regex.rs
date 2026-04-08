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
        if k == 0 || tokens[k - 1].kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND)
        {
            if self.mode & REMIMU_MODE_INVERTED != 0 {
                self.invert_mask();
                self.mode &= !REMIMU_MODE_INVERTED;
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
fn apply_shorthand_mask(token_mask: &mut [u16; 16], c_lower: u8, invert: bool) {
    let mut m = [0u16; 16];
    if c_lower == b'd' || c_lower == b'w' {
        m[3] |= 0x03FF;
    }
    if c_lower == b's' {
        m[0] |= 0x3E00;
        m[2] |= 1;
    }
    if c_lower == b'w' {
        m[4] |= 0xFFFE;
        m[5] |= 0x87FF;
        m[6] |= 0xFFFE;
        m[7] |= 0x07FF;
    }
    for i in 0..16 {
        token_mask[i] |= if invert { !m[i] } else { m[i] };
    }
}

fn parse_hex_escape(pattern: &[u8], i: usize) -> Result<u8, i32> {
    if i + 2 >= pattern.len() {
        return Err(-1);
    }
    // NOTE: The C code has a bug where it reads pattern[i+1] twice (for both n0 and n1).
    // We replicate that behavior exactly.
    let mut n0 = pattern[i + 1];
    let mut n1 = pattern[i + 1];
    if n0 < b'0' || n0 > b'f' || n1 < b'0' || n1 > b'f'
        || (n0 > b'9' && n0 < b'A') || (n1 > b'9' && n1 < b'A')
    {
        return Err(-1);
    }
    if n0 > b'F' { n0 -= 0x20; }
    if n1 > b'F' { n1 -= 0x20; }
    if n0 >= b'A' { n0 -= b'A' - 10; }
    if n1 >= b'A' { n1 -= b'A' - 10; }
    n0 -= b'0';
    n1 -= b'0';
    Ok((n1 << 4) | n0)
}

pub fn regex_parse(pattern: &str, tokens: &mut Vec<RegexToken>, token_count: &mut i16, flags: i32) -> Result<(), i32> {
    let tokens_len = *token_count as usize;
    let pat = pattern.as_bytes();

    let mut esc_state = 0;

    const STATE_NORMAL: u8 = 1;
    const STATE_QUANT: u8 = 2;
    const STATE_MODE: u8 = 3;
    const STATE_CC_INIT: u8 = 4;
    const STATE_CC_NORMAL: u8 = 5;
    const STATE_CC_RANGE: u8 = 6;

    let mut state = STATE_NORMAL;
    let mut char_class_mem: i32 = -1;
    let mut token = RegexToken::default();

    // start with invisible group specifier
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;
    let mut i = 0usize;

    while i < pat.len() {
        let c = pat[i];

        if state == STATE_QUANT {
            state = STATE_MODE;
            if c == b'?' {
                token.count_lo = 0;
                token.count_hi = 2;
                i += 1; continue;
            } else if c == b'+' {
                token.count_lo = 1;
                token.count_hi = 0;
                i += 1; continue;
            } else if c == b'*' {
                token.count_lo = 0;
                token.count_hi = 0;
                i += 1; continue;
            } else if c == b'{' {
                if i + 1 >= pat.len() || pat[i + 1] < b'0' || pat[i + 1] > b'9' {
                    state = STATE_NORMAL;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while i < pat.len() && pat[i] >= b'0' && pat[i] <= b'9' {
                        val = val * 10 + (pat[i] - b'0') as u32;
                        if val > 0xFFFF { return Err(-1); }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = val as u16 + 1;
                    if i < pat.len() && pat[i] == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if i < pat.len() && pat[i] >= b'0' && pat[i] <= b'9' {
                            let mut val2: u32 = 0;
                            while i < pat.len() && pat[i] >= b'0' && pat[i] <= b'9' {
                                val2 = val2 * 10 + (pat[i] - b'0') as u32;
                                if val2 > 0xFFFF { return Err(-1); }
                                i += 1;
                            }
                            if val2 < val { return Err(-1); }
                            token.count_hi = val2 as u16 + 1;
                        }
                    }
                    if i < pat.len() && pat[i] == b'}' {
                        i += 1; continue;
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
                i += 1; continue;
            } else if c == b'+' {
                token.mode |= REMIMU_MODE_POSSESSIVE;
                i += 1; continue;
            }
        }

        if state == STATE_NORMAL {
            if esc_state == 1 {
                esc_state = 0;
                match c {
                    b'n' => { token.set_mask(b'\n'); }
                    b'r' => { token.set_mask(b'\r'); }
                    b't' => { token.set_mask(b'\t'); }
                    b'v' => { token.set_mask(0x0B); }
                    b'f' => { token.set_mask(0x0C); }
                    b'x' => {
                        let val = parse_hex_escape(pat, i)?;
                        token.set_mask(val);
                        i += 2;
                    }
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' |
                    b'|' | b'^' | b'$' | b'*' | b'+' | b'?' | b':' |
                    b'.' | b'/' | b'\\' => {
                        token.set_mask(c);
                        state = STATE_QUANT;
                    }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        let is_upper = c <= b'Z';
                        let c_lower = if is_upper { c + 0x20 } else { c };
                        apply_shorthand_mask(&mut token.mask, c_lower, is_upper);
                        token.kind = REMIMU_KIND_NORMAL;
                        state = STATE_QUANT;
                    }
                    b'b' => {
                        token.kind = REMIMU_KIND_BOUND;
                        state = STATE_NORMAL;
                    }
                    b'B' => {
                        token.kind = REMIMU_KIND_NBOUND;
                        state = STATE_NORMAL;
                    }
                    _ => { return Err(-1); }
                }
            } else {
                token.push_to_vec(tokens, tokens_len)?;
                match c {
                    b'\\' => { esc_state = 1; }
                    b'[' => {
                        state = STATE_CC_INIT;
                        char_class_mem = -1;
                        token.kind = REMIMU_KIND_NORMAL;
                        if i + 1 < pat.len() && pat[i + 1] == b'^' {
                            token.mode |= REMIMU_MODE_INVERTED;
                            i += 1;
                        }
                    }
                    b'(' => {
                        paren_count += 1;
                        state = STATE_NORMAL;
                        token.kind = REMIMU_KIND_OPEN;
                        token.count_lo = 0;
                        token.count_hi = 1;
                        if i + 2 < pat.len() && pat[i + 1] == b'?' && pat[i + 2] == b':' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            i += 2;
                        } else if i + 2 < pat.len() && pat[i + 1] == b'?' && pat[i + 2] == b'>' {
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.push_to_vec(tokens, tokens_len)?;
                            state = STATE_NORMAL;
                            token.kind = REMIMU_KIND_NCOPEN;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.count_lo = 1;
                            token.count_hi = 2;
                            i += 2;
                        }
                    }
                    b')' => {
                        paren_count -= 1;
                        if paren_count < 0 || tokens.is_empty() { return Err(-1); }
                        token.kind = REMIMU_KIND_CLOSE;
                        state = STATE_QUANT;

                        let k = tokens.len();
                        let mut balance: i32 = 0;
                        let mut found: i64 = -1;
                        for l in (0..k).rev() {
                            if tokens[l].kind == REMIMU_KIND_NCOPEN || tokens[l].kind == REMIMU_KIND_OPEN {
                                if balance == 0 { found = l as i64; break; }
                                else { balance -= 1; }
                            } else if tokens[l].kind == REMIMU_KIND_CLOSE {
                                balance += 1;
                            }
                        }
                        if found == -1 { return Err(-1); }
                        let diff = k as i64 - found;
                        if diff > 32767 { return Err(-1); }
                        token.pair_offset = -(diff as i16);
                        tokens[found as usize].pair_offset = diff as i16;

                        if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                            token.push_to_vec(tokens, tokens_len)?;
                            token.kind = REMIMU_KIND_CLOSE;
                            token.mode = REMIMU_MODE_POSSESSIVE;
                            token.pair_offset = -(diff as i16) - 2;
                            tokens[found as usize - 1].pair_offset = diff as i16 + 2;
                        }
                    }
                    b'?' | b'+' | b'*' | b'{' => { return Err(-1); }
                    b'.' => {
                        for n in 0..16 { token.mask[n] = 0xFFFF; }
                        if flags & REMIMU_FLAG_DOT_NO_NEWLINES != 0 {
                            token.mask[1] ^= 0x04; // \n
                            token.mask[1] ^= 0x20; // \r
                        }
                        state = STATE_QUANT;
                    }
                    b'^' => { token.kind = REMIMU_KIND_CARET; state = STATE_NORMAL; }
                    b'$' => { token.kind = REMIMU_KIND_DOLLAR; state = STATE_NORMAL; }
                    b'|' => { token.kind = REMIMU_KIND_OR; state = STATE_NORMAL; }
                    _ => { token.set_mask(c); state = STATE_QUANT; }
                }
            }
        } else if state == STATE_CC_INIT || state == STATE_CC_NORMAL || state == STATE_CC_RANGE {
            if c == b'\\' && esc_state == 0 {
                esc_state = 1;
                i += 1; continue;
            }
            let mut esc_c: u8 = 0;
            if esc_state == 1 {
                esc_state = 0;
                match c {
                    b'n' => { esc_c = b'\n'; }
                    b'r' => { esc_c = b'\r'; }
                    b't' => { esc_c = b'\t'; }
                    b'v' => { esc_c = 0x0B; }
                    b'f' => { esc_c = 0x0C; }
                    b'x' => {
                        esc_c = parse_hex_escape(pat, i)?;
                        i += 2;
                    }
                    b'{' | b'}' | b'[' | b']' | b'-' | b'(' | b')' |
                    b'|' | b'^' | b'$' | b'*' | b'+' | b'?' | b':' |
                    b'.' | b'/' | b'\\' => { esc_c = c; }
                    b'd' | b's' | b'w' | b'D' | b'S' | b'W' => {
                        if state == STATE_CC_RANGE { return Err(-1); }
                        let is_upper = c <= b'Z';
                        let c_lower = if is_upper { c + 0x20 } else { c };
                        apply_shorthand_mask(&mut token.mask, c_lower, is_upper);
                        char_class_mem = -1;
                        i += 1; continue;
                    }
                    _ => { return Err(-1); }
                }
            }
            let effective_c = if esc_c != 0 { esc_c } else { c };
            if state == STATE_CC_INIT {
                char_class_mem = effective_c as i32;
                token.set_mask(effective_c);
                state = STATE_CC_NORMAL;
            } else if state == STATE_CC_NORMAL {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    state = STATE_QUANT;
                    i += 1; continue;
                } else if c == b'-' && esc_c == 0 && char_class_mem >= 0 {
                    state = STATE_CC_RANGE;
                    i += 1; continue;
                } else {
                    char_class_mem = effective_c as i32;
                    token.set_mask(effective_c);
                    state = STATE_CC_NORMAL;
                }
            } else if state == STATE_CC_RANGE {
                if c == b']' && esc_c == 0 {
                    char_class_mem = -1;
                    token.set_mask(b'-');
                    state = STATE_QUANT;
                    i += 1; continue;
                } else {
                    if char_class_mem == -1 { return Err(-1); }
                    if (effective_c as u8) < char_class_mem as u8 { return Err(-1); }
                    let mut j = effective_c;
                    while j > char_class_mem as u8 {
                        token.set_mask(j);
                        j -= 1;
                    }
                    state = STATE_CC_NORMAL;
                    char_class_mem = -1;
                }
            }
        }
        i += 1;
    }

    if paren_count > 0 { return Err(-1); }
    if esc_state != 0 { return Err(-1); }
    if state >= STATE_CC_INIT { return Err(-1); }

    token.push_to_vec(tokens, tokens_len)?;

    // add invisible non-capturing group close
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

    // copy quantifiers from )s to (s and assign quantified group indices
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
        } else if tokens[k2].kind == REMIMU_KIND_OR
            || tokens[k2].kind == REMIMU_KIND_OPEN
            || tokens[k2].kind == REMIMU_KIND_NCOPEN
        {
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            for l in (k2 + 1)..k {
                if tokens[l].kind == REMIMU_KIND_OR && balance == 0 {
                    found = l as i64;
                    break;
                } else if tokens[l].kind == REMIMU_KIND_CLOSE {
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
pub fn regex_match(
    tokens: &[RegexToken],
    text: &str,
    start_i: usize,
    cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Option<usize> {
    regex_match_inner(tokens, text.as_bytes(), start_i, cap_slots, cap_pos, cap_span)
}

fn check_is_w(byte: u8) -> bool {
    const W_MASK: [u16; 16] = [
        0, 0, 0, 0x03FF, 0xFFFE, 0x87FF, 0xFFFE, 0x07FF,
        0, 0, 0, 0, 0, 0, 0, 0,
    ];
    (W_MASK[(byte >> 4) as usize] & (1 << (byte & 0xF))) != 0
}

fn regex_match_inner(
    tokens: &[RegexToken],
    text: &[u8],
    start_i: usize,
    mut cap_slots: u16,
    cap_pos: &mut [i64],
    cap_span: &mut [i64],
) -> Option<usize> {
    const STACK_SIZE_MAX: usize = 1024;
    const AUX_STATS_SIZE: usize = 1024;

    if cap_slots > AUX_STATS_SIZE as u16 {
        cap_slots = AUX_STATS_SIZE as u16;
    }

    let mut q_group_accepts_zero = [0u8; AUX_STATS_SIZE];
    let mut q_group_state = [0u32; AUX_STATS_SIZE];
    let mut q_group_stack = [0u32; AUX_STATS_SIZE];
    let mut q_group_cap_index = [0xFFFFu16; AUX_STATS_SIZE];

    let mut tokens_len: usize = 0;
    let mut k: usize = 0;
    let mut caps: u16 = 0;

    while tokens[k].kind != REMIMU_KIND_END {
        if tokens[k].kind == REMIMU_KIND_OPEN && caps < cap_slots {
            let m0 = tokens[k].mask[0] as usize;
            let close_k = (k as i32 + tokens[k].pair_offset as i32) as usize;
            let cm0 = tokens[close_k].mask[0] as usize;
            q_group_cap_index[m0] = caps;
            q_group_cap_index[cm0] = caps;
            cap_pos[caps as usize] = -1;
            cap_span[caps as usize] = -1;
            caps += 1;
        }
        k += 1;
        if tokens[k].kind == REMIMU_KIND_CLOSE
            || tokens[k].kind == REMIMU_KIND_OPEN
            || tokens[k].kind == REMIMU_KIND_NCOPEN
        {
            let m0 = tokens[k].mask[0] as usize;
            if m0 >= AUX_STATS_SIZE {
                return None; // OOM
            }
            q_group_state[m0] = 0;
            q_group_stack[m0] = 0;
            q_group_accepts_zero[m0] = 0;
        }
    }
    tokens_len = k;

    let mut rewind_stack: Vec<RegexMatcherState> = (0..STACK_SIZE_MAX)
        .map(|_| RegexMatcherState::new(0, 0))
        .collect();
    let mut stack_n: usize = 0;

    let mut i: u64 = start_i as u64;
    let mut range_min: u64 = 0;
    let mut range_max: u64 = 0;
    let mut just_rewinded: bool = false;

    let text_byte = |idx: u64| -> u8 {
        if (idx as usize) < text.len() { text[idx as usize] } else { 0 }
    };

    macro_rules! rewind_save_raw {
        ($kk:expr, $is_dummy:expr) => {{
            if stack_n >= STACK_SIZE_MAX {
                return None; // OOM
            }
            let mut s = RegexMatcherState::new(0, 0);
            s.i = i;
            s.k = $kk as u32;
            s.range_min = range_min;
            s.range_max = range_max;
            s.prev = 0;
            if $is_dummy {
                s.prev = 0xFAC7;
            } else if tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[s.k as usize].mask[0] as usize;
                s.group_state = q_group_state[m0];
                s.prev = q_group_stack[m0];
                q_group_stack[m0] = stack_n as u32;
            }
            rewind_stack[stack_n] = s;
            stack_n += 1;
        }};
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
            i = rewind_stack[stack_n].i;
            k = rewind_stack[stack_n].k as usize;
            if tokens[k].kind == REMIMU_KIND_CLOSE {
                let m0 = tokens[k].mask[0] as usize;
                q_group_state[m0] = rewind_stack[stack_n].group_state;
                q_group_stack[m0] = rewind_stack[stack_n].prev;
            }
            k -= 1; // because of k += 1 in loop
        }};
    }

    k = 0;
    while k < tokens_len {
        let tb = text_byte(i);
        if tokens[k].kind == REMIMU_KIND_CARET {
            if i != 0 { rewind_or_abort!(); }
            k += 1; continue;
        } else if tokens[k].kind == REMIMU_KIND_DOLLAR {
            if tb != 0 { rewind_or_abort!(); }
            k += 1; continue;
        } else if tokens[k].kind == REMIMU_KIND_BOUND {
            if i == 0 && !check_is_w(tb) {
                rewind_or_abort!();
            } else if i != 0 && tb == 0 && !check_is_w(text_byte(i - 1)) {
                rewind_or_abort!();
            } else if i != 0 && tb != 0 && check_is_w(text_byte(i - 1)) == check_is_w(tb) {
                rewind_or_abort!();
            }
        } else if tokens[k].kind == REMIMU_KIND_NBOUND {
            if i == 0 && check_is_w(tb) {
                rewind_or_abort!();
            } else if i != 0 && tb == 0 && check_is_w(text_byte(i - 1)) {
                rewind_or_abort!();
            } else if i != 0 && tb != 0 && check_is_w(text_byte(i - 1)) != check_is_w(tb) {
                rewind_or_abort!();
            }
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
                    let close_m0 = tokens[(k as i32 + tokens[k].pair_offset as i32) as usize].mask[0] as usize;
                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0
                        && (tokens[k].count_lo == 0 || q_group_accepts_zero[close_m0] != 0)
                    {
                        range_min = 0;
                        range_max = 0;
                        rewind_save_raw!(k, false);
                        k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                        // automatic += 1 will put us past the matching )
                    } else {
                        range_min = 1;
                        range_max = 0;
                        rewind_save_raw!(k, false);
                    }
                } else {
                    just_rewinded = false;
                    let orig_k = k;

                    if range_min != 0 {
                        k += range_min as usize;
                        if tokens[k - 1].kind == REMIMU_KIND_OR {
                            k = (k as i32 - 1 + tokens[k - 1].pair_offset as i32 - 1) as usize;
                        } else if tokens[k - 1].kind == REMIMU_KIND_OPEN
                            || tokens[k - 1].kind == REMIMU_KIND_NCOPEN
                        {
                            k = (k as i32 - 1 + tokens[k - 1].mask[15] as i32 - 1) as usize;
                        }

                        if tokens[k].kind == REMIMU_KIND_END {
                            return None; // -3 invalid
                        }

                        if tokens[k].kind == REMIMU_KIND_CLOSE {
                            let m0 = tokens[k].mask[0] as usize;
                            if tokens[k].count_lo == 0 || q_group_accepts_zero[m0] != 0 {
                                q_group_state[m0] = 0;
                                if (tokens[k].mode & REMIMU_MODE_LAZY) == 0 {
                                    q_group_stack[m0] = 0;
                                }
                                k += 1; continue;
                            } else {
                                rewind_or_abort!();
                                k += 1; continue;
                            }
                        }
                    }

                    let k_diff = k - orig_k;
                    range_min = (k_diff + 1) as u64;
                    rewind_save_raw!(k - k_diff, false);
                }
            } else if tokens[k].kind == REMIMU_KIND_CLOSE {
                // unquantified
                if tokens[k].count_lo == 1 && tokens[k].count_hi == 2 {
                    let cap_index = q_group_cap_index[tokens[k].mask[0] as usize];
                    if cap_index != 0xFFFF {
                        rewind_save_raw!(k, true);
                    }
                } else {
                    // quantified
                    if !just_rewinded {
                        let m0 = tokens[k].mask[0] as usize;
                        let prev = q_group_stack[m0];

                        range_max = tokens[k].count_hi as u64;
                        if range_max > 0 { range_max -= 1; }
                        range_min = if q_group_accepts_zero[m0] != 0 { 0 } else { tokens[k].count_lo as u64 };

                        // minimum not yet met
                        if (q_group_state[m0] as u64 + 1) < range_min {
                            q_group_state[m0] += 1;
                            rewind_save_raw!(k, false);
                            k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                            k -= 1;
                            k += 1; continue;
                        }
                        // maximum exceeded
                        else if tokens[k].count_hi != 0
                            && (q_group_state[m0] as u64 + 1) > range_max
                        {
                            range_max -= 1;
                            rewind_or_abort!();
                            k += 1; continue;
                        }

                        // detect zero-length matches when backtracked
                        let mut force_zero = false;
                        if prev != 0 && rewind_stack[prev as usize].i as u32 > i as u32 {
                            let open_k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                            let mut n = stack_n - 1;
                            while n > 0 && rewind_stack[n].k != open_k as u32 {
                                n -= 1;
                            }
                            if rewind_stack[n].i == i {
                                force_zero = true;
                            }
                        }

                        // reject zero-length matches
                        if force_zero
                            || (prev != 0 && rewind_stack[prev as usize].i as u32 == i as u32)
                        {
                            q_group_accepts_zero[m0] = 1;
                            rewind_or_abort!();
                        } else if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            // lazy
                            q_group_state[m0] += 1;
                            rewind_save_raw!(k, false);
                            q_group_state[m0] = 0;
                        } else {
                            // greedy
                            if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                let k2 = if q_group_state[m0] == 0 {
                                    (k as i32 + tokens[k].pair_offset as i32) as u32
                                } else {
                                    k as u32
                                };
                                if stack_n == 0 { return None; }
                                stack_n -= 1;
                                while stack_n > 0 && rewind_stack[stack_n].k != k2 {
                                    stack_n -= 1;
                                }
                                if stack_n == 0 { return None; }
                            }
                            let open_k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                            let open_m0 = tokens[open_k].mask[0] as usize;
                            if (q_group_state[open_m0] as u32) < (i as u32) {
                                q_group_state[m0] += 1;
                                rewind_save_raw!(k, false);
                                k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                                k -= 1;
                            }
                        }
                    } else {
                        just_rewinded = false;
                        let m0 = tokens[k].mask[0] as usize;

                        if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                            // lazy rewind: try matching group again
                            rewind_save_raw!(k, true);
                            q_group_stack[m0] = stack_n as u32;
                            k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                            k -= 1;
                        } else {
                            // greedy rewind
                            if q_group_state[m0] < range_min as u32
                                && q_group_accepts_zero[m0] == 0
                            {
                                rewind_or_abort!();
                            } else {
                                q_group_state[m0] = 0;
                                let cap_index = q_group_cap_index[m0];
                                if cap_index != 0xFFFF {
                                    rewind_save_raw!(k, true);
                                }
                            }
                        }
                    }
                }
            } else if tokens[k].kind == REMIMU_KIND_OR {
                k = (k as i32 + tokens[k].pair_offset as i32) as usize;
                k -= 1;
            } else if tokens[k].kind == REMIMU_KIND_NORMAL {
                if !just_rewinded {
                    let mut n: u64 = 0;
                    let old_i = i;
                    while n < tokens[k].count_lo as u64
                        && text_byte(i) != 0
                        && tokens[k].check_mask(text_byte(i))
                    {
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
                        rewind_save_raw!(k, false);
                    } else {
                        let mut limit = tokens[k].count_hi as u64;
                        if limit == 0 { limit = u64::MAX; }
                        range_min = n;
                        while text_byte(i) != 0
                            && tokens[k].check_mask(text_byte(i))
                            && n + 1 < limit
                        {
                            i += 1;
                            n += 1;
                        }
                        range_max = n;
                        if (tokens[k].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                            rewind_save_raw!(k, false);
                        }
                    }
                } else {
                    just_rewinded = false;

                    if (tokens[k].mode & REMIMU_MODE_LAZY) != 0 {
                        let limit = if range_max == 0 { u64::MAX } else { range_max };
                        if tokens[k].check_mask(text_byte(i))
                            && text_byte(i) != 0
                            && range_min < limit
                        {
                            i += 1;
                            range_min += 1;
                            rewind_save_raw!(k, false);
                        } else {
                            rewind_or_abort!();
                        }
                    } else {
                        if range_max > range_min {
                            i -= 1;
                            range_max -= 1;
                            rewind_save_raw!(k, false);
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
            let kind = tokens[s.k as usize].kind;
            if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_CLOSE {
                let cap_index = q_group_cap_index[tokens[s.k as usize].mask[0] as usize];
                if cap_index == 0xFFFF { continue; }
                let ci = cap_index as usize;
                if kind == REMIMU_KIND_OPEN {
                    cap_pos[ci] = s.i as i64;
                } else if cap_pos[ci] >= 0 {
                    cap_span[ci] = s.i as i64 - cap_pos[ci];
                }
            }
        }
        for n in 0..caps as usize {
            if cap_span[n] == -1 {
                cap_pos[n] = -1;
            }
        }
    }

    Some(i as usize)
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_str = |k: u8| -> &'static str {
        match k {
            0 => "NORMAL", 1 => "OPEN", 2 => "NCOPEN", 3 => "CLOSE",
            4 => "OR", 5 => "CARET", 6 => "DOLLAR", 7 => "BOUND",
            8 => "NBOUND", 9 => "END", _ => "???",
        }
    };
    let mode_str = |m: u8| -> &'static str {
        match m {
            0 => "GREEDY", 1 => "POSSESS", 2 => "LAZY", _ => "GREEDY",
        }
    };
    for (_k, tok) in tokens.iter().enumerate() {
        print!("{}\t{}\t", kind_str(tok.kind), mode_str(tok.mode));
        if tok.kind == REMIMU_KIND_NORMAL {
            let mut c_old: i32 = -1;
            for c in 0..256u16 {
                let print_c_smart = |ch: u16| {
                    if ch >= 0x20 && ch <= 0x7E {
                        print!("{}", ch as u8 as char);
                    } else {
                        print!("\\x{:02x}", ch);
                    }
                };
                if tok.check_mask(c as u8) {
                    if c_old == -1 { c_old = c as i32; }
                } else if c_old != -1 {
                    if c as i32 - 1 == c_old {
                        print_c_smart(c_old as u16);
                    } else if c as i32 - 2 == c_old {
                        print_c_smart(c_old as u16);
                        print_c_smart(c_old as u16 + 1);
                    } else {
                        print_c_smart(c_old as u16);
                        print!("-");
                        print_c_smart(c - 1);
                    }
                    c_old = -1;
                }
            }
        }
        println!(
            "\t{{{},{}}}\t({})",
            tok.count_lo,
            tok.count_hi.wrapping_sub(1),
            tok.pair_offset
        );
        if tok.kind == REMIMU_KIND_END { break; }
    }
}