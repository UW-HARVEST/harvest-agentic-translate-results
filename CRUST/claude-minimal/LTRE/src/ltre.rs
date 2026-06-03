// Rust port of ltre.c. We map the C linked-list-based NFA into an index-based
// representation: NState references hold `Option<usize>` indices into the
// `Nfa.states` Vec. We mirror DFA states similarly. The high-level algorithms
// (parsing, powerset construction, minimization, decompilation) follow the C.

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymSet {
    bits: [u8; 256 / 8],
}
impl SymSet {
    pub fn empty() -> Self {
        SymSet { bits: [0; 32] }
    }
    pub fn full() -> Self {
        SymSet { bits: [0xff; 32] }
    }
    pub fn contains(&self, c: u8) -> bool {
        (self.bits[(c as usize) / 8] >> ((c as usize) % 8)) & 1 != 0
    }
    pub fn insert(&mut self, c: u8) {
        self.bits[(c as usize) / 8] |= 1 << ((c as usize) % 8);
    }
    pub fn invert(&mut self) {
        for b in self.bits.iter_mut() {
            *b = !*b;
        }
    }
    pub fn union_with(&mut self, other: &SymSet) {
        for i in 0..32 {
            self.bits[i] |= other.bits[i];
        }
    }
    pub fn intersect_with(&mut self, other: &SymSet) {
        for i in 0..32 {
            self.bits[i] &= other.bits[i];
        }
    }
    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }
}

fn is_print(c: u8) -> bool {
    c >= 0x20 && c < 0x7f
}
fn is_digit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}
fn is_xdigit(c: u8) -> bool {
    is_digit(c) || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}
fn is_alpha(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z')
}
fn is_alnum(c: u8) -> bool {
    is_alpha(c) || is_digit(c)
}
fn is_space(c: u8) -> bool {
    // matches C isspace
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c
}
fn to_lower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        c + 32
    } else {
        c
    }
}
fn to_upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' {
        c - 32
    } else {
        c
    }
}

pub fn symset_fmt(set: &SymSet) -> String {
    // Mirrors the C symset_fmt. Output shall be parsable by `parse_symset` and
    // satisfy `parse_symset . symset_fmt == id`.
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym = 0i32;
    let mut nnsym = 0i32;
    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    let mut chr: i32 = 0;
    while chr < 256 {
        // append_chr label
        let c = chr as u8;
        let in_set = set.contains(c);
        if in_set {
            nsym += 1;
        } else {
            nnsym += 1;
        }
        let target: &mut Vec<u8> = if in_set { &mut buf } else { &mut nbuf };
        let is_metachar = c != 0 && METACHARS.contains(&c);
        if !is_print(c) && !is_metachar {
            target.extend_from_slice(format!("\\x{:02x}", c).as_bytes());
        } else {
            if is_metachar {
                target.push(b'\\');
            }
            target.push(c);
        }
        // make character ranges
        let start = chr;
        while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
            chr += 1;
        }
        if chr - start >= 2 {
            let was_in = set.contains(chr as u8);
            let target2: &mut Vec<u8> = if was_in { &mut buf } else { &mut nbuf };
            target2.push(b'-');
            if was_in {
                nsym -= 1;
            } else {
                nnsym -= 1;
            }
        }
        if chr - start >= 1 {
            // goto append_chr: re-enter without incrementing (chr stays put)
            continue;
        }
        // for-loop chr++ equivalent
        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        // bufp[-2] = '\0'; return buf+1; -- drop trailing ']' and leading '['
        // We pushed: '[' then content then ']'. The C `bufp[-2] = '\0'` removes
        // the last char before ']' actually. Let me re-read...
        // Actually `bufp[-2] = '\0'` overwrites the position of the last char,
        // because bufp points after '\0'. Wait: bufp points after ']'. bufp[-1]
        // is '\0', bufp[-2] is ']', so it terminates before the ']'. Then return
        // buf+1 skips the '['. So it's: skip '[' and trailing ']'.
        let s = &buf[1..buf.len() - 1];
        return String::from_utf8_lossy(s).into_owned();
    } else if nnsym == 1 {
        // nbufp[-2] = '\0', nbuf[1] = '^' -> drop ']', overwrite '[' with '^',
        // return nbuf+1. So: nbuf[1..len-1] but nbuf[1] becomes '^'.
        // nbuf was: '^','[', content, ']' --> we return from index 1, with nbuf[1]='^'.
        // So result = "^" + content (without the leading '[' and without trailing ']').
        let mut out = vec![b'^'];
        out.extend_from_slice(&nbuf[2..nbuf.len() - 1]);
        return String::from_utf8_lossy(&out).into_owned();
    }

    if buf.len() < nbuf.len() {
        String::from_utf8_lossy(&buf).into_owned()
    } else {
        String::from_utf8_lossy(&nbuf).into_owned()
    }
}

#[derive(Clone, Debug)]
pub struct NState {
    pub label: SymSet,
    pub target: Option<usize>,
    pub epsilon0: Option<usize>,
    pub epsilon1: Option<usize>,
}
impl NState {
    pub fn new() -> Self {
        NState {
            label: SymSet::empty(),
            target: None,
            epsilon0: None,
            epsilon1: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Nfa {
    pub states: Vec<NState>,
    pub initial: usize,
    pub final_: usize,
    pub complemented: bool,
}
impl Nfa {
    pub fn new_single() -> Self {
        let s = NState::new();
        Nfa {
            states: vec![s],
            initial: 0,
            final_: 0,
            complemented: false,
        }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
}

pub fn nfa_clone(orig: &Nfa) -> Nfa {
    orig.clone()
}

/// Concatenate `nfa2` to the end of `nfa1`, transferring all of `nfa2`'s states
/// into `nfa1` and merging `nfa2.initial` onto `nfa1.final_`.
pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    // mirror of C: if nfap->initial == nfap->final, replace nfap with nfa
    if nfa1.initial == nfa1.final_ && nfa1.states.len() == 1 {
        // The C version frees nfap and replaces it with nfa entirely.
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ && nfa2.states.len() == 1 {
        // empty NFA on the right: concatenation does nothing
        return;
    }
    // Otherwise, copy nfa2.initial into nfa1.final_ and append the rest.
    let offset = nfa1.states.len();
    let initial2 = nfa2.initial;
    let final2 = nfa2.final_;
    // Compute the new final index: it's `final2` mapped through the rewrite.
    // We will copy nfa2.states[initial2] onto nfa1.states[nfa1.final_] (with
    // index shift); other nfa2 states get appended at offsets relative to
    // (offset - 1) since one state (initial2) was merged.
    // Map nfa2 state index `i` -> if i == initial2 then nfa1.final_;
    //   else offset + (i if i < initial2 else i - 1).
    let map_idx = |i: usize, nfa1_final: usize| -> usize {
        if i == initial2 {
            nfa1_final
        } else if i < initial2 {
            offset + i
        } else {
            offset + i - 1
        }
    };
    // Overwrite nfa1.states[nfa1.final_] with nfa2.states[initial2], remapped.
    let nfa1_final = nfa1.final_;
    let init_state = &nfa2.states[initial2];
    let new_state = NState {
        label: init_state.label,
        target: init_state.target.map(|i| map_idx(i, nfa1_final)),
        epsilon0: init_state.epsilon0.map(|i| map_idx(i, nfa1_final)),
        epsilon1: init_state.epsilon1.map(|i| map_idx(i, nfa1_final)),
    };
    nfa1.states[nfa1_final] = new_state;
    // Append the remaining states from nfa2 (skipping initial2).
    for (i, s) in nfa2.states.iter().enumerate() {
        if i == initial2 {
            continue;
        }
        nfa1.states.push(NState {
            label: s.label,
            target: s.target.map(|j| map_idx(j, nfa1_final)),
            epsilon0: s.epsilon0.map(|j| map_idx(j, nfa1_final)),
            epsilon1: s.epsilon1.map(|j| map_idx(j, nfa1_final)),
        });
    }
    nfa1.final_ = map_idx(final2, nfa1_final);
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let new_state = NState {
        label: SymSet::empty(),
        target: None,
        epsilon0: Some(nfa.initial),
        epsilon1: None,
    };
    nfa.states.push(new_state);
    nfa.initial = nfa.states.len() - 1;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    let new_state = NState::new();
    nfa.states.push(new_state);
    nfa.states[nfa.final_].epsilon0 = Some(new_idx);
    nfa.final_ = new_idx;
}

pub fn nfa_uncomplement(nfa: &mut Nfa) -> Result<(), String> {
    if !nfa.complemented {
        return Ok(());
    }
    let dfa = ltre_compile(nfa.clone());
    let new_nfa = ltre_uncompile(&dfa);
    *nfa = new_nfa;
    Ok(())
}

pub fn nfa_dump(_nfa: &Nfa) {}

#[derive(Clone)]
pub struct DState {
    pub transitions: [usize; 256],
    pub accepting: bool,
    pub terminating: bool,
    pub bitset: Vec<u8>,
}
impl std::fmt::Debug for DState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DState")
            .field("accepting", &self.accepting)
            .field("terminating", &self.terminating)
            .finish()
    }
}

#[derive(Clone, Debug)]
pub struct Dfa {
    pub states: Vec<DState>,
    pub initial: usize,
}
impl Dfa {
    pub fn new() -> Self {
        Dfa {
            states: Vec::new(),
            initial: 0,
        }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
}

fn leb128_put(buf: &mut Vec<u8>, mut n: i32) {
    while (n >> 7) != 0 {
        buf.push(((n & 0x7f) as u8) | 0x80);
        n >>= 7;
    }
    buf.push(n as u8);
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: i32 = 0;
    let mut c = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128 overflow".into());
        }
        let b = buf[*p];
        n |= ((b & 0x7f) as i32) << (c * 7);
        c += 1;
        *p += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    Ok(n)
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf = Vec::new();
    let dfa_size = dfa.states.len() as i32;
    leb128_put(&mut buf, dfa_size);
    for state in &dfa.states {
        let flag = ((state.accepting as u8) << 1) | (state.terminating as u8);
        buf.push(flag);
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            while chr < 255 && state.transitions[chr] == state.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8); // run length
            leb128_put(&mut buf, state.transitions[chr] as i32);
            chr += 1;
        }
    }
    buf
}

pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p = 0usize;
    let dfa_size = leb128_get(buf, &mut p)? as usize;
    let mut states: Vec<DState> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        states.push(DState {
            transitions: [0; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        });
    }
    for id in 0..dfa_size {
        let flag = buf[p];
        p += 1;
        states[id].accepting = (flag >> 1) & 1 != 0;
        states[id].terminating = flag & 1 != 0;
        let mut chr = 0usize;
        while chr < 256 {
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            for _ in 0..=len {
                states[id].transitions[chr] = target;
                chr += 1;
            }
        }
    }
    Ok((Dfa { states, initial: 0 }, p))
}

pub fn dfa_dump(_dfa: &Dfa) {}

pub fn nfa_free(_nfa: Nfa) {}
pub fn dfa_free(_dfa: Dfa) {}

// ----------------- Parser -----------------

struct ParseContext<'a> {
    chars: &'a [u8],
    pos: usize,
}
impl<'a> ParseContext<'a> {
    fn new(s: &'a str) -> Self {
        ParseContext {
            chars: s.as_bytes(),
            pos: 0,
        }
    }
    fn peek(&self) -> Option<u8> {
        self.chars.get(self.pos).copied()
    }
    fn next(&mut self) -> Option<u8> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }
    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
    fn expect_char(&mut self) -> Result<u8, String> {
        match self.next() {
            Some(c) => Ok(c),
            None => Err("unexpected end of input".into()),
        }
    }
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let c = ctx.peek().ok_or_else(|| "expected natural number".to_string())?;
    if !is_digit(c) {
        return Err("expected natural number".into());
    }
    let mut n: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !is_digit(c) {
            break;
        }
        let digit = (c - b'0') as u32;
        if n > u32::MAX / 10 || n * 10 > u32::MAX - digit {
            // consume remaining digits then signal overflow
            while let Some(c2) = ctx.peek() {
                if !is_digit(c2) {
                    break;
                }
                ctx.next();
            }
            return Err("natural number overflow".into());
        }
        n = n * 10 + digit;
        ctx.next();
    }
    Ok(n)
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte: u8 = 0;
    for _ in 0..2 {
        byte <<= 4;
        let c = ctx.peek().ok_or_else(|| "expected hex digit".to_string())?;
        if is_digit(c) {
            byte |= c - b'0';
        } else if is_xdigit(c) {
            byte |= to_lower(c) - b'a' + 10;
        } else {
            return Err("expected hex digit".into());
        }
        ctx.next();
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or_else(|| "unknown escape".to_string())?;
    if METACHARS.contains(&c) {
        ctx.next();
        return Ok(c);
    }
    ctx.next();
    match c {
        b'a' => Ok(0x07),
        b'b' => Ok(0x08),
        b'f' => Ok(0x0c),
        b'n' => Ok(b'\n'),
        b'r' => Ok(b'\r'),
        b't' => Ok(b'\t'),
        b'v' => Ok(0x0b),
        b'x' => parse_hexbyte(ctx),
        _ => Err("unknown escape".into()),
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or_else(|| "expected symbol".to_string())?;
    if c == b'\\' {
        ctx.next();
        return parse_escape(ctx);
    }
    if METACHARS.contains(&c) {
        return Err("unexpected metacharacter".into());
    }
    if !is_print(c) {
        return Err("unexpected nonprintable character".into());
    }
    ctx.next();
    Ok(c)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let start = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        if let Some(c) = ctx.next() {
            let mut set = SymSet::empty();
            match c {
                b'd' => {
                    for ch in 0..=255u8 {
                        if is_digit(ch) {
                            set.insert(ch);
                        }
                    }
                    return Ok(set);
                }
                b'D' => {
                    for ch in 0..=255u8 {
                        if !is_digit(ch) {
                            set.insert(ch);
                        }
                    }
                    return Ok(set);
                }
                b's' => {
                    for ch in 0..=255u8 {
                        if is_space(ch) {
                            set.insert(ch);
                        }
                    }
                    return Ok(set);
                }
                b'S' => {
                    for ch in 0..=255u8 {
                        if !is_space(ch) {
                            set.insert(ch);
                        }
                    }
                    return Ok(set);
                }
                b'w' => {
                    for ch in 0..=255u8 {
                        if ch == b'_' || is_alnum(ch) {
                            set.insert(ch);
                        }
                    }
                    return Ok(set);
                }
                b'W' => {
                    for ch in 0..=255u8 {
                        if ch != b'_' && !is_alnum(ch) {
                            set.insert(ch);
                        }
                    }
                    return Ok(set);
                }
                _ => {}
            }
        }
        ctx.pos = start;
    }
    if ctx.peek() == Some(b'.') {
        ctx.next();
        let mut set = SymSet::empty();
        for ch in 0..=255u8 {
            if ch != b'\n' {
                set.insert(ch);
            }
        }
        return Ok(set);
    }
    Err("expected shorthand class".into())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.next();
        complement = true;
    }
    let last_pos = ctx.pos;
    if let Ok(mut s) = parse_shorthand(ctx) {
        if complement {
            s.invert();
        }
        return Ok(s);
    }
    ctx.pos = last_pos;
    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut set = SymSet::empty();
        while ctx.peek() != Some(b']') {
            let sub = parse_symset(ctx)?;
            set.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".into());
        }
        ctx.next();
        if complement {
            set.invert();
        }
        return Ok(set);
    }
    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut set = SymSet::full();
        while ctx.peek() != Some(b'>') {
            let sub = parse_symset(ctx)?;
            set.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".into());
        }
        ctx.next();
        if complement {
            set.invert();
        }
        return Ok(set);
    }
    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.next();
        end = parse_symbol(ctx)?;
    }
    let mut set = SymSet::empty();
    // C: end++ (open upper bound), with do-while loop wrapping at 256
    let endp1 = end.wrapping_add(1);
    let mut chr = begin;
    loop {
        set.insert(chr);
        chr = chr.wrapping_add(1);
        if chr == endp1 {
            break;
        }
    }
    if complement {
        set.invert();
    }
    Ok(set)
}

fn parse_atom(ctx: &mut ParseContext) -> Result<Nfa, String> {
    if ctx.peek() == Some(b'(') {
        ctx.next();
        let sub = parse_regex(ctx)?;
        if ctx.peek() != Some(b')') {
            return Err("expected ')'".into());
        }
        ctx.next();
        return Ok(sub);
    }
    let label = parse_symset(ctx)?;
    let mut nfa = Nfa {
        states: vec![
            NState {
                label,
                target: Some(1),
                epsilon0: None,
                epsilon1: None,
            },
            NState::new(),
        ],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    // ensure target is correctly set
    nfa.states[0].target = Some(1);
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;
    match ctx.peek() {
        Some(b'*') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            // atom.final->epsilon1 = atom.initial
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            // atom.initial->epsilon1 = atom.final
            atom.states[atom.initial].epsilon1 = Some(atom.final_);
            return Ok(atom);
        }
        Some(b'+') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            return Ok(atom);
        }
        Some(b'?') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            if atom.states[atom.initial].epsilon1.is_some() {
                nfa_pad_initial(&mut atom);
            }
            atom.states[atom.initial].epsilon1 = Some(atom.final_);
            return Ok(atom);
        }
        Some(b'{') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let last_pos = ctx.pos;
            // parse min, default 0 if missing
            let min: u32;
            let mut overflowed_min = false;
            match parse_natural(ctx) {
                Ok(v) => min = v,
                Err(e) => {
                    if e == "natural number overflow" {
                        return Err(e);
                    }
                    min = 0;
                    overflowed_min = false; // ignore
                }
            }
            let _ = overflowed_min;
            let mut max: u32 = min;
            let mut max_unbounded = false;
            if ctx.peek() == Some(b',') {
                ctx.next();
                match parse_natural(ctx) {
                    Ok(v) => max = v,
                    Err(e) => {
                        if e == "natural number overflow" {
                            return Err(e);
                        }
                        max_unbounded = true;
                    }
                }
            }
            if ctx.peek() != Some(b'}') {
                return Err("expected '}'".into());
            }
            ctx.next();
            if min > max && !max_unbounded {
                ctx.pos = last_pos;
                return Err("misbounded quantifier".into());
            }
            // Build atoms = empty NFA (one state)
            let mut atoms = Nfa::new_single();
            // total iterations: if max_unbounded, i in [0, min] inclusive => min+1 copies.
            // else i in [0, max) => max copies.
            let limit: u64 = if max_unbounded {
                (min as u64) + 1
            } else {
                max as u64
            };
            for i in 0..limit {
                let mut clone = nfa_clone(&atom);
                if i >= min as u64 {
                    if max_unbounded {
                        clone.states[clone.final_].epsilon1 = Some(clone.initial);
                        nfa_pad_initial(&mut clone);
                        nfa_pad_final(&mut clone);
                    }
                    clone.states[clone.initial].epsilon1 = Some(clone.final_);
                }
                nfa_concat(&mut atoms, clone);
            }
            return Ok(atoms);
        }
        _ => {}
    }
    Ok(atom)
}

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'~') {
        ctx.next();
        complement = true;
    }
    let mut term = Nfa::new_single();
    while let Some(c) = ctx.peek() {
        if c == b')' || c == b'|' || c == b'&' {
            break;
        }
        let mut factor = parse_factor(ctx)?;
        nfa_uncomplement(&mut factor)?;
        nfa_concat(&mut term, factor);
    }
    if complement {
        term.complemented = true;
    }
    Ok(term)
}

fn parse_regex(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut re = parse_term(ctx)?;
    while let Some(c) = ctx.peek() {
        if c != b'|' && c != b'&' {
            break;
        }
        let intersect = c == b'&';
        ctx.next();
        let mut alt = parse_term(ctx)?;
        // De Morgan's law: a&b == ~(~a|~b)
        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;
        // -->O-->(re)--->
        //     -->(alt)-->O-->
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);
        // Now merge alt into re. We need to:
        //  re.initial->epsilon1 = alt.initial (after offset)
        //  re.final->epsilon0 = alt.final
        //  re.final = alt.final
        let offset = re.states.len();
        for s in alt.states.iter() {
            re.states.push(NState {
                label: s.label,
                target: s.target.map(|i| i + offset),
                epsilon0: s.epsilon0.map(|i| i + offset),
                epsilon1: s.epsilon1.map(|i| i + offset),
            });
        }
        let alt_initial = alt.initial + offset;
        let alt_final = alt.final_ + offset;
        let re_initial = re.initial;
        let re_final = re.final_;
        re.states[re_initial].epsilon1 = Some(alt_initial);
        re.states[re_final].epsilon0 = Some(alt_final);
        re.final_ = alt_final;
        re.complemented ^= intersect;
    }
    Ok(re)
}

pub fn ltre_parse(regex: &str) -> Result<Nfa, String> {
    // Reject any non-printable character early (matches C: '\b', '\t', etc.
    // would be rejected by parse_symbol's isprint check but only when seen as
    // a symbol; a lone control character at top-level still flows through.)
    let mut ctx = ParseContext::new(regex);
    let nfa = parse_regex(&mut ctx)?;
    if !ctx.is_eof() {
        return Err("expected end of input".into());
    }
    Ok(nfa)
}

pub fn ltre_fixed_string(s: &str) -> Nfa {
    let mut nfa = Nfa::new_single();
    for &b in s.as_bytes() {
        let new_idx = nfa.states.len();
        nfa.states.push(NState::new());
        nfa.states[nfa.final_].target = Some(new_idx);
        nfa.states[nfa.final_].label.insert(b);
        nfa.final_ = new_idx;
    }
    nfa
}

pub fn ltre_partial(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    nfa_pad_initial(nfa);
    nfa_pad_final(nfa);
    let i = nfa.initial;
    let f = nfa.final_;
    nfa.states[i].target = Some(i);
    nfa.states[f].target = Some(f);
    nfa.states[i].label = SymSet::full();
    nfa.states[f].label = SymSet::full();
    Ok(())
}

pub fn ltre_ignorecase(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    for state in nfa.states.iter_mut() {
        let mut to_add = SymSet::empty();
        for chr in 0..=255u8 {
            if state.label.contains(chr) {
                to_add.insert(to_lower(chr));
                to_add.insert(to_upper(chr));
            }
        }
        state.label.union_with(&to_add);
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

// ----------------- Compilation -----------------

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    if bitset_test(bitset, st_id) {
        return;
    }
    bitset_set(bitset, st_id);
    if let Some(e0) = nfa.states[st_id].epsilon0 {
        epsilon_closure_into(nfa, e0, bitset);
    }
    if let Some(e1) = nfa.states[st_id].epsilon1 {
        epsilon_closure_into(nfa, e1, bitset);
    }
}

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let mut bitset = vec![0u8; (nfa_size + 7) / 8];
    epsilon_closure_into(nfa, start, &mut bitset);
    bitset
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let mut out = vec![0u8; (nfa_size + 7) / 8];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) && nfa.states[id].label.contains(chr) {
            if let Some(t) = nfa.states[id].target {
                epsilon_closure_into(nfa, t, &mut out);
            }
        }
    }
    out
}

fn dfa_step(dfa: &mut Dfa, dstate_idx: Option<usize>, chr: u8, nfa: &Nfa) -> usize {
    let nfa_size = nfa.states.len();
    let bitset_union = if let Some(d) = dstate_idx {
        step_powerset(nfa, &dfa.states[d].bitset, chr)
    } else {
        epsilon_closure_vec(nfa, nfa.initial, nfa_size)
    };

    // search for an existing dstate with matching bitset
    let mut found: Option<usize> = None;
    for (i, s) in dfa.states.iter().enumerate() {
        if s.bitset == bitset_union {
            found = Some(i);
            break;
        }
    }
    let target = if let Some(i) = found {
        i
    } else {
        let accepting_raw = bitset_test(&bitset_union, nfa.final_);
        let accepting = accepting_raw ^ nfa.complemented;
        let new = DState {
            transitions: [0; 256],
            accepting,
            terminating: false,
            bitset: bitset_union,
        };
        dfa.states.push(new);
        dfa.states.len() - 1
    };
    if let Some(d) = dstate_idx {
        dfa.states[d].transitions[chr as usize] = target;
    }
    target
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let mut dfa = Dfa::new();
    // initial state
    dfa_step(&mut dfa, None, 0, &nfa);
    let mut idx = 0usize;
    while idx < dfa.states.len() {
        for chr in 0..=255u8 {
            dfa_step(&mut dfa, Some(idx), chr, &nfa);
            if chr == 255 {
                break;
            }
        }
        idx += 1;
    }
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }
    // distinguishability matrix as Vec<Vec<bool>> of size n*n
    let mut dis = vec![vec![false; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                dis[i][j] = true;
                dis[j][i] = true;
            }
        }
    }
    loop {
        let mut changed = false;
        for i in 0..n {
            for j in (i + 1)..n {
                if dis[i][j] {
                    continue;
                }
                for chr in 0..256usize {
                    let ti = dfa.states[i].transitions[chr];
                    let tj = dfa.states[j].transitions[chr];
                    if ti != tj && dis[ti][tj] {
                        dis[i][j] = true;
                        dis[j][i] = true;
                        changed = true;
                        break;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    // Build equivalence classes; pick canonical = lowest index in class
    let mut canon = (0..n).collect::<Vec<usize>>();
    for i in 0..n {
        for j in 0..i {
            if !dis[i][j] {
                canon[i] = canon[j];
                break;
            }
        }
    }
    // Re-number canonical states sequentially preserving order
    let mut new_id = vec![usize::MAX; n];
    let mut counter = 0usize;
    let mut order = Vec::new();
    for i in 0..n {
        if canon[i] == i {
            new_id[i] = counter;
            order.push(i);
            counter += 1;
        }
    }
    for i in 0..n {
        if canon[i] != i {
            new_id[i] = new_id[canon[i]];
        }
    }
    let mut new_states: Vec<DState> = Vec::with_capacity(counter);
    for &i in &order {
        let mut trans = [0usize; 256];
        for c in 0..256 {
            trans[c] = new_id[dfa.states[i].transitions[c]];
        }
        new_states.push(DState {
            transitions: trans,
            accepting: dfa.states[i].accepting,
            terminating: false,
            bitset: Vec::new(),
        });
    }
    // compute terminating: a state is terminating iff all transitions point to itself
    for i in 0..new_states.len() {
        let mut term = true;
        for c in 0..256 {
            if new_states[i].transitions[c] != i {
                term = false;
                break;
            }
        }
        new_states[i].terminating = term;
    }
    dfa.states = new_states;
    dfa.initial = new_id[dfa.initial];
}

fn find_or_create_dead(_states: &mut Vec<DState>) -> usize {
    0
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    (bs[idx / 8] >> (idx % 8)) & 1 != 0
}
fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx / 8] |= 1 << (idx % 8);
}
fn all_bitset_indices(bs: &[u8]) -> impl Iterator<Item = usize> + '_ {
    let mut out = Vec::new();
    for (byte_i, &b) in bs.iter().enumerate() {
        if b != 0 {
            for bit_i in 0..8 {
                if b & (1 << bit_i) != 0 {
                    out.push(byte_i * 8 + bit_i);
                }
            }
        }
    }
    out.into_iter()
}

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut state = dfa.initial;
    for &b in input {
        if dfa.states[state].terminating {
            break;
        }
        state = dfa.states[state].transitions[b as usize];
    }
    dfa.states[state].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    if dfap.is_none() {
        *dfap = Some(Dfa::new());
    }
    let dfa = dfap.as_mut().unwrap();
    if dfa.states.is_empty() {
        dfa_step(dfa, None, 0, nfa);
    }
    let mut state = dfa.initial;
    let mut i = 0;
    while i < input.len() {
        let b = input[i];
        // ensure the transition exists; in our dfa, every state has all 256
        // transitions assigned to some index, but we lazy-build new states by
        // checking if the transition target is "valid" for this NFA.
        // We track "explored" via bitset, but for simplicity, always step.
        let next = dfa.states[state].transitions[b as usize];
        // Check if `next` is the initial dead state (id 0 by default).
        // The C code checks `if (!dstate->transitions[*input])`, meaning the
        // transition is NULL. We mark unexplored transitions specially using
        // a sentinel. To implement lazy, we'd need a separate "explored" flag.
        // For correctness, just continue stepping — eager construction in
        // dfa_step will have already been done if state was created.
        let _ = next;
        // To make this actually lazy, we re-step if needed:
        let dead = is_unexplored(&dfa.states[state]);
        if dead.contains(b as usize) {
            dfa_step(dfa, Some(state), b, nfa);
        }
        state = dfa.states[state].transitions[b as usize];
        i += 1;
    }
    dfa.states[state].accepting
}

// In our representation, every transition is set to some valid index when the
// state is created (via dfa_step or initially). For lazy mode, we want to know
// whether the transition was explicitly explored. As a simplification, we mark
// all transitions on a freshly-created state as "unexplored" by storing them
// initially as target = state's own index, and only consider them set after
// dfa_step is called for that (state, chr). However, given our `dfa_step`
// already handles bitset-based dedup, calling it repeatedly is a no-op for
// already-explored transitions. So we just always re-step:
struct UnexploredSet;
impl UnexploredSet {
    fn contains(&self, _: usize) -> bool {
        true
    }
}
fn is_unexplored(_s: &DState) -> UnexploredSet {
    UnexploredSet
}

// ----------------- Uncompile (DFA -> NFA) -----------------

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    // create initial + final + one nstate per dfa state
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    // mapping: nstates[id] = index in nfa.states for dfa state id
    let mut nstates_map = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        nfa.states.push(NState::new());
        nstates_map.push(nfa.states.len() - 1);
    }
    nfa.states[nfa.initial].epsilon1 = Some(nstates_map[dfa.initial]);
    for (id, ds) in dfa.states.iter().enumerate() {
        if ds.accepting {
            nfa.states[nstates_map[id]].epsilon1 = Some(nfa.final_);
        }
    }

    for id1 in 0..dfa_size {
        // For each unique target state, gather the transitions symset
        // we mirror the C: iterate ds2 in DFA order.
        let mut free_idx: Option<usize> = None;
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if dfa.states[id1].transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let src_idx: usize;
            if free_idx.is_none() {
                src_idx = nstates_map[id1];
                free_idx = Some(src_idx);
            } else {
                // alloc new state
                nfa.states.push(NState::new());
                let new_idx = nfa.states.len() - 1;
                src_idx = new_idx;
                let f = free_idx.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_idx);
                } else {
                    nfa.states[f].epsilon0 = Some(new_idx);
                    free_idx = Some(new_idx);
                }
            }
            nfa.states[src_idx].target = Some(nstates_map[id2]);
            nfa.states[src_idx].label = transitions;
        }
    }
    nfa
}

// ----------------- Decompile (DFA -> regex string) -----------------

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
    Alt = 0,
    Concat = 1,
    Quant = 2,
    Symset = 3,
}

#[derive(Clone)]
struct Arrow {
    label: Option<Vec<u8>>, // None => empty /[]/, Some(empty) => epsilon /()/
    prec: Prec,
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let aux = dfa_size;
    let total = dfa_size + 1;
    let mut arrows: Vec<Vec<Arrow>> = vec![
        vec![
            Arrow {
                label: None,
                prec: Prec::Symset,
            };
            total
        ];
        total
    ];

    // epsilon from aux to initial
    arrows[aux][dfa.initial] = Arrow {
        label: Some(Vec::new()),
        prec: Prec::Symset,
    };
    for id1 in 0..dfa_size {
        if dfa.states[id1].accepting {
            arrows[id1][aux] = Arrow {
                label: Some(Vec::new()),
                prec: Prec::Symset,
            };
        }
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if dfa.states[id1].transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                arrows[id1][id2] = Arrow {
                    label: None,
                    prec: Prec::Symset,
                };
            } else {
                let s = symset_fmt(&transitions);
                arrows[id1][id2] = Arrow {
                    label: Some(s.into_bytes()),
                    prec: Prec::Symset,
                };
            }
        }
    }

    loop {
        // pick best fit: minimal vertex degree among non-aux states
        let mut best_fit: Option<usize> = None;
        let mut min_degree = i32::MAX;
        for id1 in 0..dfa_size {
            let mut degree = 0i32;
            for id2 in 0..dfa_size {
                if arrows[id1][id2].label.is_some() {
                    degree += 1;
                }
                if arrows[id2][id1].label.is_some() {
                    degree += 1;
                }
            }
            if degree == 0 {
                continue;
            }
            if degree < min_degree {
                min_degree = degree;
                best_fit = Some(id1);
            }
        }
        if best_fit.is_none() {
            break;
        }
        let bf = best_fit.unwrap();
        for id1 in 0..total {
            if id1 == bf {
                continue;
            }
            for id2 in 0..total {
                if id2 == bf {
                    continue;
                }
                let in_arrow = arrows[id1][bf].clone();
                let out_arrow = arrows[bf][id2].clone();
                let self_arrow = arrows[bf][bf].clone();
                let existing = arrows[id1][id2].clone();
                if in_arrow.label.is_none() || out_arrow.label.is_none() {
                    continue;
                }

                // Compute first, second from in/self/out
                let in_label = in_arrow.label.clone().unwrap();
                let out_label = out_arrow.label.clone().unwrap();
                let self_label_opt = self_arrow.label.clone();

                let (first, second) = compute_first_second(
                    &in_arrow,
                    &in_label,
                    &out_arrow,
                    &out_label,
                    &self_arrow,
                    self_label_opt.as_deref(),
                );

                // Build bypass = first . second
                let bypass = compute_bypass(&first, &second);

                // Merge with existing
                let merged = compute_merge(&existing, &bypass);
                arrows[id1][id2] = merged;
            }
        }
        for id in 0..total {
            arrows[id][bf] = Arrow {
                label: None,
                prec: Prec::Symset,
            };
            arrows[bf][id] = Arrow {
                label: None,
                prec: Prec::Symset,
            };
        }
    }

    let result = arrows[aux][aux].label.clone();
    match result {
        Some(v) => String::from_utf8(v).unwrap_or_default(),
        None => "[]".to_string(),
    }
}

fn compute_first_second(
    in_arrow: &Arrow,
    in_label: &[u8],
    out_arrow: &Arrow,
    out_label: &[u8],
    self_arrow: &Arrow,
    self_label: Option<&[u8]>,
) -> (Arrow, Arrow) {
    // Case 1: self is None or empty -> (in)(out)
    if self_label.is_none() || self_label.unwrap().is_empty() {
        return (in_arrow.clone(), out_arrow.clone());
    }
    let self_label = self_label.unwrap();

    // Try the in_pre case: (in_pre)(self)+(out)
    if in_arrow.prec >= Prec::Concat
        && self_arrow.prec >= Prec::Concat
        && in_label.len() >= self_label.len()
    {
        let diff = in_label.len() - self_label.len();
        if &in_label[diff..] == self_label {
            // hackily try to avoid breaking apart symsets in the inbound arrow
            let mut nevermind = false;
            if diff >= 1 {
                let c = in_label[diff - 1];
                if (c == b'^' || c == b'-' || c == b'\\')
                    && (diff == 1 || in_label[diff - 2] != b'\\')
                {
                    nevermind = true;
                }
            }
            if !nevermind && diff >= 2 {
                if &in_label[diff - 2..diff] == b"\\x"
                    && (diff == 2 || in_label[diff - 3] != b'\\')
                {
                    nevermind = true;
                }
            }
            if !nevermind && diff >= 3 {
                if &in_label[diff - 3..diff - 1] == b"\\x"
                    && (diff == 3 || in_label[diff - 4] != b'\\')
                {
                    nevermind = true;
                }
            }
            if !nevermind {
                // first = (in_pre)(self)+
                let mut first_label: Vec<u8> = Vec::new();
                if diff != 0 && in_arrow.prec < Prec::Concat {
                    first_label.push(b'(');
                }
                first_label.extend_from_slice(&in_label[..diff]);
                if diff != 0 && in_arrow.prec < Prec::Concat {
                    first_label.push(b')');
                }
                if self_arrow.prec <= Prec::Quant {
                    first_label.push(b'(');
                }
                first_label.extend_from_slice(self_label);
                if self_arrow.prec <= Prec::Quant {
                    first_label.push(b')');
                }
                first_label.push(b'+');
                let first = Arrow {
                    label: Some(first_label),
                    prec: Prec::Concat,
                };
                return (first, out_arrow.clone());
            }
        }
    }

    // out_post case: (in)(self)+(out_post)
    if out_arrow.prec >= Prec::Concat
        && self_arrow.prec >= Prec::Concat
        && out_label.len() >= self_label.len()
    {
        let diff = out_label.len() - self_label.len();
        if &out_label[..self_label.len()] == self_label {
            let mut second_label = Vec::new();
            if self_arrow.prec <= Prec::Quant {
                second_label.push(b'(');
            }
            second_label.extend_from_slice(self_label);
            if self_arrow.prec <= Prec::Quant {
                second_label.push(b')');
            }
            second_label.push(b'+');
            if diff != 0 && out_arrow.prec < Prec::Concat {
                second_label.push(b'(');
            }
            second_label.extend_from_slice(&out_label[self_label.len()..]);
            if diff != 0 && out_arrow.prec < Prec::Concat {
                second_label.push(b')');
            }
            let second = Arrow {
                label: Some(second_label),
                prec: Prec::Concat,
            };
            return (in_arrow.clone(), second);
        }
    }

    // Default: (in)(self)*(out)
    let mut second_label = Vec::new();
    if self_arrow.prec <= Prec::Quant {
        second_label.push(b'(');
    }
    second_label.extend_from_slice(self_label);
    if self_arrow.prec <= Prec::Quant {
        second_label.push(b')');
    }
    second_label.push(b'*');
    if out_arrow.prec < Prec::Concat {
        second_label.push(b'(');
    }
    second_label.extend_from_slice(out_label);
    if out_arrow.prec < Prec::Concat {
        second_label.push(b')');
    }
    let second = Arrow {
        label: Some(second_label),
        prec: Prec::Concat,
    };
    (in_arrow.clone(), second)
}

fn compute_bypass(first: &Arrow, second: &Arrow) -> Arrow {
    let fl = first.label.as_ref().unwrap();
    let sl = second.label.as_ref().unwrap();
    if fl.is_empty() {
        return second.clone();
    }
    if sl.is_empty() {
        return first.clone();
    }
    let mut label = Vec::new();
    if first.prec < Prec::Concat {
        label.push(b'(');
    }
    label.extend_from_slice(fl);
    if first.prec < Prec::Concat {
        label.push(b')');
    }
    if second.prec < Prec::Concat {
        label.push(b'(');
    }
    label.extend_from_slice(sl);
    if second.prec < Prec::Concat {
        label.push(b')');
    }
    Arrow {
        label: Some(label),
        prec: Prec::Concat,
    }
}

fn compute_merge(existing: &Arrow, bypass: &Arrow) -> Arrow {
    if bypass.label.is_none() {
        return existing.clone();
    }
    if existing.label.is_none() {
        return bypass.clone();
    }
    let el = existing.label.as_ref().unwrap();
    let bl = bypass.label.as_ref().unwrap();
    if el.is_empty() {
        // ()|(bypass) == (bypass)?
        let mut label = Vec::new();
        if bypass.prec <= Prec::Quant {
            label.push(b'(');
        }
        label.extend_from_slice(bl);
        if bypass.prec <= Prec::Quant {
            label.push(b')');
        }
        label.push(b'?');
        return Arrow {
            label: Some(label),
            prec: Prec::Quant,
        };
    }
    // (existing)|(bypass)
    let mut label = Vec::new();
    label.extend_from_slice(el);
    label.push(b'|');
    label.extend_from_slice(bl);
    Arrow {
        label: Some(label),
        prec: Prec::Alt,
    }
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}
fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}
fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in b'0'..=b'9' {
        s.insert(c);
    }
    s
}
fn not_digits_set() -> SymSet {
    let mut s = digits_set();
    s.invert();
    s
}
fn spaces_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0..=255u8 {
        if is_space(c) {
            s.insert(c);
        }
    }
    s
}
fn not_spaces_set() -> SymSet {
    let mut s = spaces_set();
    s.invert();
    s
}
fn wordchar_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0..=255u8 {
        if c == b'_' || is_alnum(c) {
            s.insert(c);
        }
    }
    s
}
fn not_wordchar_set() -> SymSet {
    let mut s = wordchar_set();
    s.invert();
    s
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(ref mut v) = opt {
        *v += offset;
    }
}
