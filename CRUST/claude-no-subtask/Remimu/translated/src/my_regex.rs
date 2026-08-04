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
        // Skip duplicate consecutive BOUND/NBOUND tokens
        if !(k == 0
            || tokens[k - 1].kind != self.kind
            || (self.kind != REMIMU_KIND_BOUND && self.kind != REMIMU_KIND_NBOUND))
        {
            // duplicate BOUND/NBOUND — do not push, but still clear self
            *self = RegexToken::default();
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

#[derive(Debug, Clone, Copy)]
enum State {
    Normal,
    Quant,
    Mode,
    CharClassInit,
    CharClassNormal,
    CharClassRange,
}

fn set_shorthand_class(token: &mut RegexToken, c: u8) {
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
    for i in 0..16 {
        if is_upper {
            token.mask[i] |= !m[i];
        } else {
            token.mask[i] |= m[i];
        }
    }
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
    tokens.clear();

    let pattern_bytes = pattern.as_bytes();
    let pattern_len = pattern_bytes.len();

    let mut esc_state = 0;
    let mut state = State::Normal;
    let mut char_class_mem: i32 = -1;

    let mut token = RegexToken::default();
    // start with an invisible group specifier
    token.kind = REMIMU_KIND_OPEN;
    token.count_lo = 0;
    token.count_hi = 0;

    let mut paren_count: i32 = 0;
    let mut i: usize = 0;

    let pat_at = |idx: usize| -> u8 {
        if idx < pattern_len {
            pattern_bytes[idx]
        } else {
            0
        }
    };

    while i < pattern_len {
        let c = pattern_bytes[i];

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
                let next_c = pat_at(i + 1);
                if next_c == 0 || next_c < b'0' || next_c > b'9' {
                    state = State::Normal;
                } else {
                    i += 1;
                    let mut val: u32 = 0;
                    while pat_at(i) >= b'0' && pat_at(i) <= b'9' {
                        val = val.wrapping_mul(10);
                        val = val.wrapping_add((pat_at(i) - b'0') as u32);
                        if val > 0xFFFF {
                            return Err(-1);
                        }
                        i += 1;
                    }
                    token.count_lo = val as u16;
                    token.count_hi = (val + 1) as u16;
                    if pat_at(i) == b',' {
                        token.count_hi = 0;
                        i += 1;
                        if pat_at(i) >= b'0' && pat_at(i) <= b'9' {
                            let mut val2: u32 = 0;
                            while pat_at(i) >= b'0' && pat_at(i) <= b'9' {
                                val2 = val2.wrapping_mul(10);
                                val2 = val2.wrapping_add((pat_at(i) - b'0') as u32);
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
                    if pat_at(i) == b'}' {
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
            if esc_state == 1 {
                esc_state = 0;
                if c == b'n' {
                    token.set_mask(b'\n');
                    state = State::Quant;
                } else if c == b'r' {
                    token.set_mask(b'\r');
                    state = State::Quant;
                } else if c == b't' {
                    token.set_mask(b'\t');
                    state = State::Quant;
                } else if c == b'v' {
                    token.set_mask(0x0B);
                    state = State::Quant;
                } else if c == b'f' {
                    token.set_mask(0x0C);
                    state = State::Quant;
                } else if c == b'x' {
                    if pat_at(i + 1) == 0 || pat_at(i + 2) == 0 {
                        return Err(-1);
                    }
                    let mut n0 = pat_at(i + 1);
                    let mut n1 = pat_at(i + 2);
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
                    // The C bug: n0 = pattern[i+1], n1 = pattern[i+1] (not i+2). Replicate same:
                    // Actually let's preserve the C bug to match: it sets both to same byte.
                    // But functionally this is broken. For the tests to be correct vs C reference,
                    // we need to mirror exactly. The C code: n1 = pattern[i+1];
                    // Hmm. Replicate C behavior:
                    let _ = (n0, n1);
                    let mut nn0 = pat_at(i + 1);
                    let mut nn1 = pat_at(i + 1);
                    if nn0 > b'F' {
                        nn0 -= 0x20;
                    }
                    if nn1 > b'F' {
                        nn1 -= 0x20;
                    }
                    if nn0 >= b'A' {
                        nn0 -= b'A' - 10;
                    }
                    if nn1 >= b'A' {
                        nn1 -= b'A' - 10;
                    }
                    nn0 = nn0.wrapping_sub(b'0');
                    nn1 = nn1.wrapping_sub(b'0');
                    token.set_mask((nn1 << 4) | nn0);
                    i += 2;
                    state = State::Quant;
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
                    state = State::Quant;
                } else if c == b'd' || c == b's' || c == b'w' || c == b'D' || c == b'S' || c == b'W' {
                    set_shorthand_class(&mut token, c);
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
                token.push_to_vec(tokens, max_len)?;
                if c == b'\\' {
                    esc_state = 1;
                } else if c == b'[' {
                    state = State::CharClassInit;
                    char_class_mem = -1;
                    token.kind = REMIMU_KIND_NORMAL;
                    if pat_at(i + 1) == b'^' {
                        token.mode |= REMIMU_MODE_INVERTED;
                        i += 1;
                    }
                } else if c == b'(' {
                    paren_count += 1;
                    state = State::Normal;
                    token.kind = REMIMU_KIND_OPEN;
                    token.count_lo = 0;
                    token.count_hi = 1;
                    if pat_at(i + 1) == b'?' && pat_at(i + 2) == b':' {
                        token.kind = REMIMU_KIND_NCOPEN;
                        i += 2;
                    } else if pat_at(i + 1) == b'?' && pat_at(i + 2) == b'>' {
                        // outer NCOPEN
                        token.kind = REMIMU_KIND_NCOPEN;
                        token.push_to_vec(tokens, max_len)?;

                        state = State::Normal;
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
                    state = State::Quant;

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

                    // phantom group for atomic group emulation
                    if tokens[found as usize].mode == REMIMU_MODE_POSSESSIVE {
                        token.push_to_vec(tokens, max_len)?;
                        token.kind = REMIMU_KIND_CLOSE;
                        token.mode = REMIMU_MODE_POSSESSIVE;
                        token.pair_offset = -(diff as i16) - 2;
                        // safe: found - 1 >= 0 because we have outer NCOPEN before
                        if found >= 1 {
                            tokens[(found - 1) as usize].pair_offset = (diff as i16) + 2;
                        }
                    }
                } else if c == b'?' || c == b'+' || c == b'*' || c == b'{' {
                    return Err(-1);
                } else if c == b'.' {
                    for n in 0..16 {
                        token.mask[n] = 0xFFFF;
                    }
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
                i += 1;
                continue;
            }
        } else if matches!(
            state,
            State::CharClassInit | State::CharClassNormal | State::CharClassRange
        ) {
            if c == b'\\' && esc_state == 0 {
                esc_state = 1;
                i += 1;
                continue;
            }
            let mut esc_c: u8 = 0;
            let mut local_c = c;
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
                    if pat_at(i + 1) == 0 || pat_at(i + 2) == 0 {
                        return Err(-1);
                    }
                    // Replicate the same C bug — both n0 and n1 use pattern[i+1]
                    let mut nn0 = pat_at(i + 1);
                    let mut nn1 = pat_at(i + 1);
                    if nn0 < b'0'
                        || nn0 > b'f'
                        || nn1 < b'0'
                        || nn1 > b'f'
                        || (nn0 > b'9' && nn0 < b'A')
                        || (nn1 > b'9' && nn1 < b'A')
                    {
                        return Err(-1);
                    }
                    if nn0 > b'F' {
                        nn0 -= 0x20;
                    }
                    if nn1 > b'F' {
                        nn1 -= 0x20;
                    }
                    if nn0 >= b'A' {
                        nn0 -= b'A' - 10;
                    }
                    if nn1 >= b'A' {
                        nn1 -= b'A' - 10;
                    }
                    nn0 = nn0.wrapping_sub(b'0');
                    nn1 = nn1.wrapping_sub(b'0');
                    esc_c = (nn1 << 4) | nn0;
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
                } else if c == b'd' || c == b's' || c == b'w' || c == b'D' || c == b'S' || c == b'W' {
                    if matches!(state, State::CharClassRange) {
                        return Err(-1);
                    }
                    set_shorthand_class(&mut token, c);
                    char_class_mem = -1;
                    i += 1;
                    continue;
                } else {
                    return Err(-1);
                }
            }
            // Use esc_c as the unescaped value indicator. Note: in CC_NORMAL, when c is a
            // syntactically meaningful char like ']' or '-', `esc_c == 0` means it's treated
            // specially. Otherwise it's a literal character — we use local_c (which is c).
            // For consistency with C (which uses esc_c == 0 to detect non-escape), we keep
            // local_c untouched but use esc_c for the escape-only check.
            // For the actual mask-set we use whichever character is the resolved literal:
            // if esc_state was used, the escape char value (esc_c). Otherwise c.
            // But wait — in C, the `_REGEX_SET_MASK(c)` always uses `c` (the loop variable)
            // even after escape parsing. After escape parsing, c is unchanged but esc_c holds
            // the escape value. The C code never updates c after escape — it only uses esc_c
            // for the "is this a literal" check. So the mask is set with original c. That's a
            // C bug (or feature?). Replicate exactly:
            let _ = local_c;

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
                    let mut x: i32 = c as i32;
                    while x > char_class_mem {
                        token.set_mask(x as u8);
                        x -= 1;
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
        i += 1;
    }

    if paren_count > 0 {
        return Err(-1);
    }
    if esc_state != 0 {
        return Err(-1);
    }
    if matches!(
        state,
        State::CharClassInit | State::CharClassNormal | State::CharClassRange
    ) {
        return Err(-1);
    }

    token.push_to_vec(tokens, max_len)?;

    // closing invisible non-capturing group specifier
    token.kind = REMIMU_KIND_CLOSE;
    token.count_lo = 1;
    token.count_hi = 2;
    token.push_to_vec(tokens, max_len)?;

    // end token
    token.kind = REMIMU_KIND_END;
    token.push_to_vec(tokens, max_len)?;

    let k = tokens.len() as i64;
    tokens[0].pair_offset = (k - 2) as i16;
    tokens[(k - 2) as usize].pair_offset = -((k - 2) as i16);

    *token_count = k as i16;

    // Copy quantifiers from )s to (s. Smuggle quantified group index in mask[0].
    let mut n: u64 = 0;
    let k_usize = tokens.len();
    for k2 in 0..k_usize {
        let kind = tokens[k2].kind;
        if kind == REMIMU_KIND_CLOSE {
            tokens[k2].mask[0] = n as u16;
            n += 1;

            let k3 = k2 as i64 + tokens[k2].pair_offset as i64;
            let k3 = k3 as usize;
            tokens[k3].count_lo = tokens[k2].count_lo;
            tokens[k3].count_hi = tokens[k2].count_hi;
            tokens[k3].mask[0] = n as u16;
            tokens[k3].mode = tokens[k2].mode;
            n += 1;

            if n > 1024 {
                return Err(-1);
            }
        } else if kind == REMIMU_KIND_OR || kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
            let mut balance: i32 = 0;
            let mut found: i64 = -1;
            let mut l = k2 + 1;
            while l < tokens.len() {
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
            let diff = found - k2 as i64;
            if diff > 32767 {
                return Err(-1);
            }
            if kind == REMIMU_KIND_OR {
                tokens[k2].pair_offset = diff as i16;
            } else {
                tokens[k2].mask[15] = diff as u16;
            }
        }
    }

    Ok(())
}

// --- Matcher implementation ---

const STACK_SIZE_MAX: usize = 1024;
const AUX_STATS_SIZE: usize = 1024;

struct Matcher<'a> {
    tokens: &'a [RegexToken],
    bytes: &'a [u8],
    rewind_stack: Vec<RegexMatcherState>,

    q_group_accepts_zero: Vec<u8>,
    q_group_state: Vec<u32>,
    q_group_stack: Vec<u32>,
    q_group_cap_index: Vec<u16>,

    cap_pos: &'a mut [i64],
    cap_span: &'a mut [i64],
    cap_slots: u16,

    k: u32,
    i: u64,
    range_min: u64,
    range_max: u64,
    just_rewinded: bool,
    stack_n: u32,
    tokens_len: u32,
}

#[derive(Debug)]
enum MatchErr {
    NoMatch, // -1
    Oom,     // -2
    Invalid, // -3
}

impl<'a> Matcher<'a> {
    fn byte_at(&self, idx: u64) -> u8 {
        let i = idx as usize;
        if i < self.bytes.len() {
            self.bytes[i]
        } else {
            0
        }
    }

    fn check_is_w(byte: u8) -> bool {
        let w_mask: [u64; 16] = {
            let mut m = [0u64; 16];
            m[3] = 0x03FF;
            m[4] = 0xFFFE;
            m[5] = 0x87FF;
            m[6] = 0xFFFE;
            m[7] = 0x07FF;
            m
        };
        (w_mask[(byte >> 4) as usize] & (1u64 << (byte & 0xF))) != 0
    }

    fn rewind_save(&mut self, k: u32, is_dummy: bool) -> Result<(), MatchErr> {
        if self.stack_n as usize >= STACK_SIZE_MAX {
            return Err(MatchErr::Oom);
        }
        let mut s = RegexMatcherState::new(k, self.i);
        s.range_min = self.range_min;
        s.range_max = self.range_max;
        s.prev = 0;

        if is_dummy {
            s.prev = 0xFAC7;
        } else if self.tokens[s.k as usize].kind == REMIMU_KIND_CLOSE {
            let g_idx = self.tokens[s.k as usize].mask[0] as usize;
            s.group_state = self.q_group_state[g_idx];
            s.prev = self.q_group_stack[g_idx];
            self.q_group_stack[g_idx] = self.stack_n;
        }
        if self.rewind_stack.len() <= self.stack_n as usize {
            self.rewind_stack.push(s);
        } else {
            self.rewind_stack[self.stack_n as usize] = s;
        }
        self.stack_n += 1;
        Ok(())
    }

    fn rewind_or_abort(&mut self) -> Result<(), MatchErr> {
        if self.stack_n == 0 {
            return Err(MatchErr::NoMatch);
        }
        self.stack_n -= 1;
        while self.stack_n > 0 && self.rewind_stack[self.stack_n as usize].prev == 0xFAC7 {
            self.stack_n -= 1;
        }
        self.just_rewinded = true;
        let s_idx = self.stack_n as usize;
        self.range_min = self.rewind_stack[s_idx].range_min;
        self.range_max = self.rewind_stack[s_idx].range_max;
        let saved_i = self.rewind_stack[s_idx].i;
        // assert(saved_i <= i)
        self.i = saved_i;
        self.k = self.rewind_stack[s_idx].k;

        if self.tokens[self.k as usize].kind == REMIMU_KIND_CLOSE {
            let g_idx = self.tokens[self.k as usize].mask[0] as usize;
            self.q_group_state[g_idx] = self.rewind_stack[s_idx].group_state;
            self.q_group_stack[g_idx] = self.rewind_stack[s_idx].prev;
        }
        // The for loop will k += 1, so we need k -= 1 here.
        // But we use a while loop, so the caller does this.
        if self.k == 0 {
            // can't decrement further; but we set just_rewinded which bypasses certain things
            // In C, k -= 1 then k++ in for makes k same. We'll handle in main loop.
            self.k = u32::MAX; // sentinel: will overflow back to 0 on +=1
        } else {
            self.k -= 1;
        }
        Ok(())
    }

    fn check_mask(&self, k: u32, byte: u8) -> bool {
        if byte == 0 {
            return false; // C uses text[i] != 0 check before; but to mimic where applicable
        }
        self.tokens[k as usize].check_mask(byte)
    }

    fn run(&mut self, start_i: u64) -> Result<u64, MatchErr> {
        // Initialize quantified group state by walking through tokens once.
        let mut k: u32 = 0;
        let mut caps: u16 = 0;
        while self.tokens[k as usize].kind != REMIMU_KIND_END {
            if self.tokens[k as usize].kind == REMIMU_KIND_OPEN && caps < self.cap_slots {
                let m0 = self.tokens[k as usize].mask[0] as usize;
                let pair = (k as i64 + self.tokens[k as usize].pair_offset as i64) as usize;
                let m_close = self.tokens[pair].mask[0] as usize;
                if m0 < AUX_STATS_SIZE {
                    self.q_group_cap_index[m0] = caps;
                }
                if m_close < AUX_STATS_SIZE {
                    self.q_group_cap_index[m_close] = caps;
                }
                if (caps as usize) < self.cap_pos.len() {
                    self.cap_pos[caps as usize] = -1;
                }
                if (caps as usize) < self.cap_span.len() {
                    self.cap_span[caps as usize] = -1;
                }
                caps += 1;
            }
            k += 1;
            let kind = self.tokens[k as usize].kind;
            if kind == REMIMU_KIND_CLOSE || kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                let m0 = self.tokens[k as usize].mask[0] as usize;
                if m0 >= AUX_STATS_SIZE {
                    return Err(MatchErr::Oom);
                }
                self.q_group_state[m0] = 0;
                self.q_group_stack[m0] = 0;
                self.q_group_accepts_zero[m0] = 0;
            }
        }
        self.tokens_len = k;

        self.stack_n = 0;
        self.i = start_i;
        self.range_min = 0;
        self.range_max = 0;
        self.just_rewinded = false;

        self.k = 0;
        while self.k < self.tokens_len {
            // map sentinel u32::MAX (after k=0 rewind) to actual start
            if self.k == u32::MAX {
                self.k = 0;
            }
            let k_now = self.k as usize;
            let kind = self.tokens[k_now].kind;

            if kind == REMIMU_KIND_CARET {
                if self.i != 0 {
                    self.rewind_or_abort()?;
                }
            } else if kind == REMIMU_KIND_DOLLAR {
                if self.byte_at(self.i) != 0 {
                    self.rewind_or_abort()?;
                }
            } else if kind == REMIMU_KIND_BOUND {
                let b_here = self.byte_at(self.i);
                if self.i == 0 && !Self::check_is_w(b_here) {
                    self.rewind_or_abort()?;
                } else if self.i != 0 && b_here == 0 && !Self::check_is_w(self.byte_at(self.i - 1)) {
                    self.rewind_or_abort()?;
                } else if self.i != 0
                    && b_here != 0
                    && Self::check_is_w(self.byte_at(self.i - 1)) == Self::check_is_w(b_here)
                {
                    self.rewind_or_abort()?;
                }
            } else if kind == REMIMU_KIND_NBOUND {
                let b_here = self.byte_at(self.i);
                if self.i == 0 && Self::check_is_w(b_here) {
                    self.rewind_or_abort()?;
                } else if self.i != 0 && b_here == 0 && Self::check_is_w(self.byte_at(self.i - 1)) {
                    self.rewind_or_abort()?;
                } else if self.i != 0
                    && b_here != 0
                    && Self::check_is_w(self.byte_at(self.i - 1)) != Self::check_is_w(b_here)
                {
                    self.rewind_or_abort()?;
                }
            } else {
                // Deliberately unmatchable token (count_hi == 1 means {0,0})
                if self.tokens[k_now].count_hi == 1 {
                    if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                        let off = self.tokens[k_now].pair_offset as i64;
                        self.k = (self.k as i64 + off) as u32;
                    } else {
                        self.k = self.k.wrapping_add(1);
                    }
                    self.k = self.k.wrapping_add(1);
                    continue;
                }

                if kind == REMIMU_KIND_OPEN || kind == REMIMU_KIND_NCOPEN {
                    if !self.just_rewinded {
                        let pair_idx = (self.k as i64 + self.tokens[k_now].pair_offset as i64) as usize;
                        let pair_m0 = self.tokens[pair_idx].mask[0] as usize;
                        let lazy_zero = (self.tokens[k_now].mode & REMIMU_MODE_LAZY) != 0
                            && (self.tokens[k_now].count_lo == 0
                                || self.q_group_accepts_zero[pair_m0] != 0);
                        if lazy_zero {
                            self.range_min = 0;
                            self.range_max = 0;
                            let saved_k = self.k;
                            self.rewind_save(saved_k, false)?;
                            self.k = (self.k as i64 + self.tokens[k_now].pair_offset as i64) as u32;
                        } else {
                            self.range_min = 1;
                            self.range_max = 0;
                            let saved_k = self.k;
                            self.rewind_save(saved_k, false)?;
                        }
                    } else {
                        self.just_rewinded = false;
                        let orig_k = self.k;

                        if self.range_min != 0 {
                            self.k = self.k.wrapping_add(self.range_min as u32);
                            let prev_idx = self.k.wrapping_sub(1) as usize;
                            let prev_kind = self.tokens[prev_idx].kind;
                            if prev_kind == REMIMU_KIND_OR {
                                self.k = (self.k as i64
                                    + self.tokens[prev_idx].pair_offset as i64
                                    - 1) as u32;
                            } else if prev_kind == REMIMU_KIND_OPEN || prev_kind == REMIMU_KIND_NCOPEN {
                                self.k = (self.k as i64
                                    + self.tokens[prev_idx].mask[15] as i64
                                    - 1) as u32;
                            }

                            if self.tokens[self.k as usize].kind == REMIMU_KIND_END {
                                return Err(MatchErr::Invalid);
                            }

                            if self.tokens[self.k as usize].kind == REMIMU_KIND_CLOSE {
                                let m0 = self.tokens[self.k as usize].mask[0] as usize;
                                if self.tokens[self.k as usize].count_lo == 0
                                    || self.q_group_accepts_zero[m0] != 0
                                {
                                    self.q_group_state[m0] = 0;
                                    if (self.tokens[self.k as usize].mode & REMIMU_MODE_LAZY) == 0 {
                                        self.q_group_stack[m0] = 0;
                                    }
                                    self.k = self.k.wrapping_add(1);
                                    continue;
                                } else {
                                    self.rewind_or_abort()?;
                                    self.k = self.k.wrapping_add(1);
                                    continue;
                                }
                            }
                            // assert(tokens[k].kind == OR)
                        }

                        let k_diff = self.k as i64 - orig_k as i64;
                        self.range_min = (k_diff + 1) as u64;
                        let save_k = (self.k as i64 - k_diff) as u32;
                        self.rewind_save(save_k, false)?;
                    }
                } else if kind == REMIMU_KIND_CLOSE {
                    // unquantified
                    if self.tokens[k_now].count_lo == 1 && self.tokens[k_now].count_hi == 2 {
                        let m0 = self.tokens[k_now].mask[0] as usize;
                        let cap_index = self.q_group_cap_index[m0];
                        if cap_index != 0xFFFF {
                            let saved_k = self.k;
                            self.rewind_save(saved_k, true)?;
                        }
                    } else {
                        // quantified
                        if !self.just_rewinded {
                            let m0 = self.tokens[k_now].mask[0] as usize;
                            let prev = self.q_group_stack[m0];

                            self.range_max = self.tokens[k_now].count_hi as u64;
                            self.range_max = self.range_max.wrapping_sub(1);
                            self.range_min = if self.q_group_accepts_zero[m0] != 0 {
                                0
                            } else {
                                self.tokens[k_now].count_lo as u64
                            };

                            // minimum requirement not yet met
                            if (self.q_group_state[m0] as u64) + 1 < self.range_min {
                                self.q_group_state[m0] += 1;
                                let saved_k = self.k;
                                self.rewind_save(saved_k, false)?;
                                self.k = (self.k as i64 + self.tokens[k_now].pair_offset as i64) as u32;
                                self.k = self.k.wrapping_sub(1);
                                self.k = self.k.wrapping_add(1);
                                continue;
                            } else if self.tokens[k_now].count_hi != 0
                                && (self.q_group_state[m0] as u64) + 1 > self.range_max
                            {
                                self.range_max = self.range_max.wrapping_sub(1);
                                self.rewind_or_abort()?;
                                self.k = self.k.wrapping_add(1);
                                continue;
                            }

                            // detect zero-length matches
                            let mut force_zero = false;
                            if prev != 0
                                && (self.rewind_stack[prev as usize].i as u32) > (self.i as u32)
                            {
                                let mut n = (self.stack_n as i64) - 1;
                                let target_k = (self.k as i64
                                    + self.tokens[k_now].pair_offset as i64)
                                    as u32;
                                while n > 0 && self.rewind_stack[n as usize].k != target_k {
                                    n -= 1;
                                }
                                if n > 0 && self.rewind_stack[n as usize].i == self.i {
                                    force_zero = true;
                                }
                            }

                            if force_zero
                                || (prev != 0
                                    && (self.rewind_stack[prev as usize].i as u32)
                                        == (self.i as u32))
                            {
                                self.q_group_accepts_zero[m0] = 1;
                                self.rewind_or_abort()?;
                            } else if (self.tokens[k_now].mode & REMIMU_MODE_LAZY) != 0 {
                                self.q_group_state[m0] += 1;
                                let saved_k = self.k;
                                self.rewind_save(saved_k, false)?;
                                self.q_group_state[m0] = 0;
                            } else {
                                // greedy
                                if (self.tokens[k_now].mode & REMIMU_MODE_POSSESSIVE) != 0 {
                                    let mut k2 = self.k;
                                    if self.q_group_state[m0] == 0 {
                                        k2 = (self.k as i64
                                            + self.tokens[k_now].pair_offset as i64)
                                            as u32;
                                    }
                                    if self.stack_n == 0 {
                                        return Err(MatchErr::NoMatch);
                                    }
                                    self.stack_n -= 1;
                                    while self.stack_n > 0
                                        && self.rewind_stack[self.stack_n as usize].k != k2
                                    {
                                        self.stack_n -= 1;
                                    }
                                    if self.stack_n == 0 {
                                        return Err(MatchErr::NoMatch);
                                    }
                                }
                                let pair_idx = (self.k as i64
                                    + self.tokens[k_now].pair_offset as i64)
                                    as usize;
                                let pair_m0 = self.tokens[pair_idx].mask[0] as usize;
                                if (self.q_group_state[pair_m0] as u32) < (self.i as u32) {
                                    self.q_group_state[m0] += 1;
                                    let saved_k = self.k;
                                    self.rewind_save(saved_k, false)?;
                                    self.k = (self.k as i64
                                        + self.tokens[k_now].pair_offset as i64)
                                        as u32;
                                    self.k = self.k.wrapping_sub(1);
                                }
                            }
                        } else {
                            self.just_rewinded = false;

                            if (self.tokens[k_now].mode & REMIMU_MODE_LAZY) != 0 {
                                let m0 = self.tokens[k_now].mask[0] as usize;
                                let saved_k = self.k;
                                self.rewind_save(saved_k, true)?;
                                self.q_group_stack[m0] = self.stack_n;
                                self.k = (self.k as i64
                                    + self.tokens[k_now].pair_offset as i64)
                                    as u32;
                                self.k = self.k.wrapping_sub(1);
                            } else {
                                let m0 = self.tokens[k_now].mask[0] as usize;
                                if (self.q_group_state[m0] as u64) < self.range_min
                                    && self.q_group_accepts_zero[m0] == 0
                                {
                                    self.rewind_or_abort()?;
                                } else {
                                    self.q_group_state[m0] = 0;
                                    let cap_index = self.q_group_cap_index[m0];
                                    if cap_index != 0xFFFF {
                                        let saved_k = self.k;
                                        self.rewind_save(saved_k, true)?;
                                    }
                                }
                            }
                        }
                    }
                } else if kind == REMIMU_KIND_OR {
                    self.k = (self.k as i64 + self.tokens[k_now].pair_offset as i64) as u32;
                    self.k = self.k.wrapping_sub(1);
                } else if kind == REMIMU_KIND_NORMAL {
                    if !self.just_rewinded {
                        let mut n: u64 = 0;
                        let old_i = self.i;
                        while n < self.tokens[k_now].count_lo as u64
                            && self.byte_at(self.i) != 0
                            && self.tokens[k_now].check_mask(self.byte_at(self.i))
                        {
                            self.i += 1;
                            n += 1;
                        }
                        if n < self.tokens[k_now].count_lo as u64 {
                            self.i = old_i;
                            self.rewind_or_abort()?;
                            self.k = self.k.wrapping_add(1);
                            continue;
                        }

                        if (self.tokens[k_now].mode & REMIMU_MODE_LAZY) != 0 {
                            self.range_min = n;
                            self.range_max = (self.tokens[k_now].count_hi as u64).wrapping_sub(1);
                            let saved_k = self.k;
                            self.rewind_save(saved_k, false)?;
                        } else {
                            let mut limit = self.tokens[k_now].count_hi as u64;
                            if limit == 0 {
                                limit = !limit;
                            }
                            self.range_min = n;
                            while self.byte_at(self.i) != 0
                                && self.tokens[k_now].check_mask(self.byte_at(self.i))
                                && n + 1 < limit
                            {
                                self.i += 1;
                                n += 1;
                            }
                            self.range_max = n;
                            if (self.tokens[k_now].mode & REMIMU_MODE_POSSESSIVE) == 0 {
                                let saved_k = self.k;
                                self.rewind_save(saved_k, false)?;
                            }
                        }
                    } else {
                        self.just_rewinded = false;

                        if (self.tokens[k_now].mode & REMIMU_MODE_LAZY) != 0 {
                            let mut limit = self.range_max;
                            if limit == 0 {
                                limit = !limit;
                            }
                            let b = self.byte_at(self.i);
                            if b != 0
                                && self.tokens[k_now].check_mask(b)
                                && self.range_min < limit
                            {
                                self.i += 1;
                                self.range_min += 1;
                                let saved_k = self.k;
                                self.rewind_save(saved_k, false)?;
                            } else {
                                self.rewind_or_abort()?;
                            }
                        } else {
                            if self.range_max > self.range_min {
                                self.i -= 1;
                                self.range_max -= 1;
                                let saved_k = self.k;
                                self.rewind_save(saved_k, false)?;
                            } else {
                                self.rewind_or_abort()?;
                            }
                        }
                    }
                } else {
                    return Err(MatchErr::Invalid);
                }
            }
            self.k = self.k.wrapping_add(1);
        }

        // capture handling
        if caps != 0 {
            for n in 0..self.stack_n as usize {
                let s = &self.rewind_stack[n];
                let k_kind = self.tokens[s.k as usize].kind;
                if k_kind == REMIMU_KIND_OPEN || k_kind == REMIMU_KIND_CLOSE {
                    let m0 = self.tokens[s.k as usize].mask[0] as usize;
                    let cap_index = self.q_group_cap_index[m0];
                    if cap_index == 0xFFFF {
                        continue;
                    }
                    let ci = cap_index as usize;
                    if k_kind == REMIMU_KIND_OPEN {
                        if ci < self.cap_pos.len() {
                            self.cap_pos[ci] = s.i as i64;
                        }
                    } else if ci < self.cap_pos.len() && self.cap_pos[ci] >= 0 {
                        if ci < self.cap_span.len() {
                            self.cap_span[ci] = s.i as i64 - self.cap_pos[ci];
                        }
                    }
                }
            }
            for n in 0..caps as usize {
                if n < self.cap_span.len() && self.cap_span[n] == -1 {
                    if n < self.cap_pos.len() {
                        self.cap_pos[n] = -1;
                    }
                }
            }
        }

        Ok(self.i)
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
    let cap_slots = cap_slots.min(AUX_STATS_SIZE as u16);
    let mut matcher = Matcher {
        tokens,
        bytes: text.as_bytes(),
        rewind_stack: Vec::with_capacity(STACK_SIZE_MAX),
        q_group_accepts_zero: vec![0u8; AUX_STATS_SIZE],
        q_group_state: vec![0u32; AUX_STATS_SIZE],
        q_group_stack: vec![0u32; AUX_STATS_SIZE],
        q_group_cap_index: vec![0xFFFFu16; AUX_STATS_SIZE],
        cap_pos,
        cap_span,
        cap_slots,
        k: 0,
        i: 0,
        range_min: 0,
        range_max: 0,
        just_rewinded: false,
        stack_n: 0,
        tokens_len: 0,
    };
    match matcher.run(start_i as u64) {
        Ok(i) => Some(i as usize),
        Err(_) => None,
    }
}

pub fn print_regex_tokens(tokens: &[RegexToken]) {
    let kind_to_str = [
        "NORMAL", "OPEN", "NCOPEN", "CLOSE", "OR", "CARET", "DOLLAR", "BOUND", "NBOUND", "END",
    ];
    let mode_to_str = ["GREEDY", "POSSESS", "LAZY"];

    let mut k = 0usize;
    loop {
        if k >= tokens.len() {
            break;
        }
        let t = &tokens[k];
        let kind_idx = t.kind as usize;
        let mode_idx = t.mode as usize;
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
        let limit = if t.kind == 0 { 256 } else { 0 };
        for c in 0..limit {
            let mask_set = t.check_mask(c as u8);
            if mask_set {
                if c_old == -1 {
                    c_old = c as i32;
                }
            } else if c_old != -1 {
                if (c as i32) - 1 == c_old {
                    print_c_smart(c_old as u32);
                    c_old = -1;
                } else if (c as i32) - 2 == c_old {
                    print_c_smart(c_old as u32);
                    print_c_smart((c_old + 1) as u32);
                    c_old = -1;
                } else {
                    print_c_smart(c_old as u32);
                    print!("-");
                    print_c_smart((c as i32 - 1) as u32);
                    c_old = -1;
                }
            }
        }

        println!(
            "\t{{{},{}}}\t({})",
            t.count_lo,
            t.count_hi as i32 - 1,
            t.pair_offset
        );

        if t.kind == REMIMU_KIND_END {
            break;
        }
        k += 1;
    }
}

fn print_c_smart(c: u32) {
    if c >= 0x20 && c <= 0x7E {
        print!("{}", c as u8 as char);
    } else {
        print!("\\x{:02x}", c);
    }
}
