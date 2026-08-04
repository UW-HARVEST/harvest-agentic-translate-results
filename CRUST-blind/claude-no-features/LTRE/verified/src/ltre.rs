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
        self.bits[(c as usize) / 8] |= 1u8 << ((c as usize) % 8);
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
    // POSIX isprint: printable including space, c >= 0x20 && c < 0x7f
    c >= 0x20 && c < 0x7f
}

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

pub fn symset_fmt(set: &SymSet) -> String {
    // Output shall be parsable by parse_symset and satisfy
    // parse_symset . symset_fmt == id
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym = 0i32;
    let mut nnsym = 0i32;

    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    let append = |buf: &mut Vec<u8>, c: u8| {
        let ism = is_metachar(c);
        if !is_print(c) && !ism {
            // \xNN
            let s = format!("\\x{:02x}", c);
            buf.extend_from_slice(s.as_bytes());
        } else {
            if ism {
                buf.push(b'\\');
            }
            buf.push(c);
        }
    };

    let mut chr: i32 = 0;
    while chr < 256 {
        let c = chr as u8;
        let in_set = set.contains(c);
        if in_set {
            nsym += 1;
        } else {
            nnsym += 1;
        }
        if in_set {
            append(&mut buf, c);
        } else {
            append(&mut nbuf, c);
        }

        // make character ranges
        let start = chr;
        while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
            chr += 1;
        }
        if chr - start >= 2 {
            let target_in = set.contains(chr as u8);
            if target_in {
                buf.push(b'-');
                nsym -= 1;
            } else {
                nbuf.push(b'-');
                nnsym -= 1;
            }
        }
        if chr - start >= 1 {
            // append the end character
            let c2 = chr as u8;
            let in_set2 = set.contains(c2);
            if in_set2 {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            if in_set2 {
                append(&mut buf, c2);
            } else {
                append(&mut nbuf, c2);
            }
        }
        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        // strip leading [ and trailing ]
        let inner = &buf[1..buf.len() - 1];
        return String::from_utf8_lossy(inner).into_owned();
    } else if nnsym == 1 {
        // nbuf starts with ^[ ... ], replace [ with ^ and strip trailing ]
        // Per the C: nbufp[-2] = '\0', nbuf[1] = '^'; return nbuf + 1
        // So we get "^^X" where the first ^ replaces [ and we skip leading ^.
        // Actually nbuf = "^[X]", nbuf[1] = '^' makes it "^^X]", then "\0" at -2 makes "^^X", return nbuf+1 = "^X"
        // Wait. nbuf is "^[X]". nbuf[1] = '^' → "^^X]" — no wait, nbuf[1] was '['. So nbuf becomes "^^X]". Then nbufp[-2]='\0' meaning replace ']' (nbufp points past the null), so position nbufp-2 is X, no wait.
        // Let me re-read. After the loop: *nbufp++ = ']' so nbufp points after ']', then *nbufp++ = '\0' so nbufp points after '\0'. So nbufp[-2] = '\0' replaces ']' with '\0'. So nbuf becomes "^[X\0".
        // Then nbuf[1] = '^' makes it "^^X\0". return nbuf+1 = "^X".
        // So in our case: nbuf is "^[X]", we want "^X".
        let mut out = Vec::new();
        out.push(b'^');
        // skip first 2 bytes (^[) and the trailing ]
        out.extend_from_slice(&nbuf[2..nbuf.len() - 1]);
        return String::from_utf8_lossy(&out).into_owned();
    }

    // return shorter
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
        let mut states = Vec::new();
        states.push(NState::new());
        Nfa {
            states,
            initial: 0,
            final_: 0,
            complemented: false,
        }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
}
pub fn nfa_free(_nfa: Nfa) {}
pub fn dfa_free(_dfa: Dfa) {}

pub fn nfa_clone(orig: &Nfa) -> Nfa {
    orig.clone()
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt.as_mut() {
        *v += offset;
    }
}

pub fn nfa_concat(nfa1: &mut Nfa, mut nfa2: Nfa) {
    // Mimics the C semantics: if nfa1 has initial == final, replace nfa1 with nfa2.
    // Else if nfa2 has initial != final, "memcpy nfa2.initial into nfa1.final_"
    // by overlaying nfa2.initial onto nfa1.final_, then nfa1.final_ = nfa2.final_ (re-mapped).
    if nfa1.initial == nfa1.final_ {
        // Replace
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        // No-op (nothing to concat)
        return;
    }
    // Append nfa2.states to nfa1.states (offset by base), but we need to:
    //   * overlay nfa2's initial state onto nfa1.final_
    //   * the rest of nfa2 states get appended; their indices shift
    let base = nfa1.states.len();
    // Move nfa2's initial state contents into nfa1.final_
    let nfa2_initial_idx = nfa2.initial;
    // Build new mapping: for nfa2 state i, mapped index is:
    //   if i == nfa2.initial: nfa1.final_
    //   else: base + (i adjusted to skip the slot of nfa2.initial)
    // Simplest: copy ALL nfa2 states (including initial), with shift, and then
    // also overlay initial onto nfa1.final_. But then nothing should reference
    // the duplicate "initial" slot. Per C semantics, anything referencing nfa2.initial
    // gets remapped to nfa1.final_; we can do this by remapping references.

    // For simplicity, let's copy all of nfa2's states with a shift, then patch
    // nfa1.final_ to be a copy of nfa2.initial (since C does memcpy initial into final).
    // The unused old initial-slot becomes orphan; we leave it (memory not freed,
    // but matching state-count is not required for correctness of language).
    // However, we need nfa1.final_ to be effectively merged. Let's do:
    //  - nfa1.final_ <- contents of nfa2.initial (with shifted refs)
    //  - then push remaining nfa2 states (excluding initial) shifted appropriately
    //  - new final_ = mapped index of nfa2.final_

    // Remap function: i -> if i == nfa2.initial: nfa1.final_, else if i < nfa2.initial: base + i, else: base + i - 1
    let map_idx = |i: usize| -> usize {
        if i == nfa2_initial_idx {
            nfa1.final_
        } else if i < nfa2_initial_idx {
            base + i
        } else {
            base + i - 1
        }
    };

    // Apply shifts to nfa2 states
    for st in nfa2.states.iter_mut() {
        if let Some(t) = st.target.as_mut() {
            *t = map_idx(*t);
        }
        if let Some(e) = st.epsilon0.as_mut() {
            *e = map_idx(*e);
        }
        if let Some(e) = st.epsilon1.as_mut() {
            *e = map_idx(*e);
        }
    }

    // Overlay nfa2.initial onto nfa1.final_
    let init_state = nfa2.states[nfa2_initial_idx].clone();
    nfa1.states[nfa1.final_] = init_state;

    // Push the rest (skip the slot at nfa2_initial_idx)
    for (i, st) in nfa2.states.into_iter().enumerate() {
        if i != nfa2_initial_idx {
            nfa1.states.push(st);
        }
    }

    nfa1.final_ = map_idx(nfa2.final_);
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    let mut s = NState::new();
    s.epsilon0 = Some(nfa.initial);
    nfa.states.push(s);
    nfa.initial = new_idx;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    nfa.states.push(NState::new());
    nfa.states[nfa.final_].epsilon0 = Some(new_idx);
    nfa.final_ = new_idx;
}

pub fn nfa_uncomplement(nfa: &mut Nfa) -> Result<(), String> {
    if !nfa.complemented {
        return Ok(());
    }
    let dfa = ltre_compile(nfa.clone());
    let uncomplemented = ltre_uncompile(&dfa);
    *nfa = uncomplemented;
    Ok(())
}

pub fn nfa_dump(nfa: &Nfa) {
    println!("graph LR");
    println!("  I( ) --> {}", nfa.initial);
    println!("  {} --> F( )", nfa.final_);
    for (i, st) in nfa.states.iter().enumerate() {
        if let Some(e) = st.epsilon0 {
            println!("  {} --> {}", i, e);
        }
        if let Some(e) = st.epsilon1 {
            println!("  {} --> {}", i, e);
        }
        if !st.label.is_empty() {
            if let Some(t) = st.target {
                let fmt = symset_fmt(&st.label);
                println!("  {} --{}--> {}", i, fmt, t);
            }
        }
    }
}

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
    // Treat as unsigned
    let mut un = n as u32;
    while (un >> 7) != 0 {
        buf.push(((un & 0x7f) as u8) | 0x80);
        un >>= 7;
    }
    buf.push((un & 0x7f) as u8);
    let _ = &mut n; // silence unused var
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: u32 = 0;
    let mut c: u32 = 0;
    loop {
        if *p >= buf.len() {
            return Err("unexpected end of buffer".to_string());
        }
        let b = buf[*p];
        *p += 1;
        n |= ((b & 0x7f) as u32) << (c * 7);
        c += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    Ok(n as i32)
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf = Vec::new();
    leb128_put(&mut buf, dfa.states.len() as i32);
    for st in dfa.states.iter() {
        let flags = ((st.accepting as u8) << 1) | (st.terminating as u8);
        buf.push(flags);
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            while chr < 255 && st.transitions[chr] == st.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8); // run length
            leb128_put(&mut buf, st.transitions[chr] as i32);
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
        if p >= buf.len() {
            return Err("unexpected end".to_string());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;
        let mut chr = 0usize;
        while chr < 256 {
            if p >= buf.len() {
                return Err("unexpected end".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            for _ in 0..=len {
                if chr >= 256 {
                    break;
                }
                states[id].transitions[chr] = target;
                chr += 1;
            }
        }
    }
    Ok((Dfa { states, initial: 0 }, p))
}

pub fn dfa_dump(dfa: &Dfa) {
    println!("graph LR");
    println!("  I( ) --> {}", dfa.initial);
    for (i1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            println!("  {} --> F( )", i1);
        }
        for i2 in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if ds1.transitions[chr] == i2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let fmt = symset_fmt(&transitions);
            println!("  {} --{}--> {}", i1, fmt, i2);
        }
    }
}

// ===== Parsing =====

pub struct ParseContext<'a> {
    pub chars: &'a [u8],
    pub pos: usize,
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
        let c = self.chars.get(self.pos).copied();
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
            None => Err("unexpected end of input".to_string()),
        }
    }
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
fn is_upper(c: u8) -> bool {
    c >= b'A' && c <= b'Z'
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let c = ctx.peek().unwrap_or(0);
    if !is_digit(c) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !is_digit(c) {
            break;
        }
        let digit = (c - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            // signal overflow: advance past digits and return error with sentinel
            // Match C semantics: keep consuming digits, return UINT_MAX
            while let Some(c2) = ctx.peek() {
                if !is_digit(c2) {
                    break;
                }
                ctx.pos += 1;
            }
            return Err("natural number overflow".to_string());
        }
        natural = natural * 10 + digit;
        ctx.pos += 1;
    }
    Ok(natural)
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte: u8 = 0;
    for _ in 0..2 {
        byte = byte.wrapping_shl(4);
        let c = match ctx.peek() {
            Some(c) => c,
            None => return Err("expected hex digit".to_string()),
        };
        if is_digit(c) {
            byte |= c - b'0';
        } else if is_xdigit(c) {
            byte |= to_lower(c) - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.pos += 1;
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = match ctx.peek() {
        Some(c) => c,
        None => return Err("unknown escape".to_string()),
    };
    if is_metachar(c) {
        ctx.pos += 1;
        return Ok(c);
    }
    ctx.pos += 1;
    match c {
        b'a' => Ok(0x07),
        b'b' => Ok(0x08),
        b'f' => Ok(0x0c),
        b'n' => Ok(b'\n'),
        b'r' => Ok(b'\r'),
        b't' => Ok(b'\t'),
        b'v' => Ok(0x0b),
        b'x' => parse_hexbyte(ctx),
        _ => {
            ctx.pos -= 1;
            Err("unknown escape".to_string())
        }
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    if ctx.peek() == Some(b'\\') {
        ctx.pos += 1;
        return parse_escape(ctx);
    }
    let c = match ctx.peek() {
        Some(c) => c,
        None => return Err("expected symbol".to_string()),
    };
    if is_metachar(c) {
        return Err("unexpected metacharacter".to_string());
    }
    if !is_print(c) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.pos += 1;
    Ok(c)
}

fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0..=255u8 {
        if is_digit(c) {
            s.insert(c);
        }
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

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}
fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

// Try to parse \d, \D, \s, \S, \w, \W, or '.'. On success, returns the symset.
// On failure, restores position and returns None.
fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let start = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.pos += 1;
        match ctx.peek() {
            Some(b'd') => {
                ctx.pos += 1;
                return Ok(digits_set());
            }
            Some(b'D') => {
                ctx.pos += 1;
                return Ok(not_digits_set());
            }
            Some(b's') => {
                ctx.pos += 1;
                return Ok(spaces_set());
            }
            Some(b'S') => {
                ctx.pos += 1;
                return Ok(not_spaces_set());
            }
            Some(b'w') => {
                ctx.pos += 1;
                return Ok(wordchar_set());
            }
            Some(b'W') => {
                ctx.pos += 1;
                return Ok(not_wordchar_set());
            }
            _ => {
                ctx.pos = start;
            }
        }
    }
    if ctx.peek() == Some(b'.') {
        ctx.pos += 1;
        let mut s = SymSet::empty();
        for c in 0..=255u8 {
            if c != b'\n' {
                s.insert(c);
            }
        }
        return Ok(s);
    }
    Err("expected shorthand class".to_string())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.pos += 1;
        complement = true;
    }

    let last_pos = ctx.pos;

    // try shorthand
    match parse_shorthand(ctx) {
        Ok(mut s) => {
            if complement {
                s.invert();
            }
            return Ok(s);
        }
        Err(_) => {
            ctx.pos = last_pos;
        }
    }

    if ctx.peek() == Some(b'[') {
        ctx.pos += 1;
        let mut symset = SymSet::empty();
        while ctx.peek() != Some(b']') {
            let sub = parse_symset(ctx)?;
            symset.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.pos += 1;
        if complement {
            symset.invert();
        }
        return Ok(symset);
    }

    if ctx.peek() == Some(b'<') {
        ctx.pos += 1;
        let mut symset = SymSet::full();
        while ctx.peek() != Some(b'>') {
            let sub = parse_symset(ctx)?;
            symset.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.pos += 1;
        if complement {
            symset.invert();
        }
        return Ok(symset);
    }

    // try symbol [- symbol]
    let saved = ctx.pos;
    let begin = match parse_symbol(ctx) {
        Ok(c) => c,
        Err(e) => {
            ctx.pos = saved;
            return Err(e);
        }
    };
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.pos += 1;
        end = parse_symbol(ctx)?;
    }
    let mut symset = SymSet::empty();
    // open upper bound: chr from begin until chr == (end+1) (mod 256), via do-while
    let end_open = end.wrapping_add(1);
    let mut chr = begin;
    loop {
        symset.insert(chr);
        chr = chr.wrapping_add(1);
        if chr == end_open {
            break;
        }
    }
    if complement {
        symset.invert();
    }
    Ok(symset)
}

// Parse atom, factor, term, regex
fn parse_atom(ctx: &mut ParseContext) -> Result<Nfa, String> {
    if ctx.peek() == Some(b'(') {
        ctx.pos += 1;
        let sub = parse_regex(ctx)?;
        if ctx.peek() != Some(b')') {
            return Err("expected ')'".to_string());
        }
        ctx.pos += 1;
        return Ok(sub);
    }

    // chars: initial -> final (labeled transition with symset)
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    nfa.states[0].target = Some(1);

    let label = parse_symset(ctx)?;
    nfa.states[0].label = label;

    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    match ctx.peek() {
        Some(b'*') => {
            ctx.pos += 1;
            nfa_uncomplement(&mut atom)?;
            // atom.final.epsilon1 = atom.initial
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            // atom.initial.epsilon1 = atom.final
            atom.states[atom.initial].epsilon1 = Some(atom.final_);
            return Ok(atom);
        }
        Some(b'+') => {
            ctx.pos += 1;
            nfa_uncomplement(&mut atom)?;
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            return Ok(atom);
        }
        Some(b'?') => {
            ctx.pos += 1;
            nfa_uncomplement(&mut atom)?;
            if atom.states[atom.initial].epsilon1.is_some() {
                nfa_pad_initial(&mut atom);
            }
            atom.states[atom.initial].epsilon1 = Some(atom.final_);
            return Ok(atom);
        }
        _ => {}
    }

    let last_pos = ctx.pos;
    if ctx.peek() == Some(b'{') {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let min;
        let mut min_overflow = false;
        match parse_natural(ctx) {
            Ok(v) => min = v,
            Err(e) => {
                if e == "natural number overflow" {
                    min_overflow = true;
                    min = 0;
                } else {
                    min = 0;
                }
            }
        }
        if min_overflow {
            return Err("natural number overflow".to_string());
        }
        let mut max = min;
        let mut max_unbounded = false;
        if ctx.peek() == Some(b',') {
            ctx.pos += 1;
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
            return Err("expected '}'".to_string());
        }
        ctx.pos += 1;

        if min > max && !max_unbounded {
            ctx.pos = last_pos;
            return Err("misbounded quantifier".to_string());
        }

        // Build atoms
        let mut atoms = Nfa::new_single();
        let count_iter: u64 = if max_unbounded {
            (min as u64) + 1
        } else {
            max as u64
        };
        let mut i: u64 = 0;
        while i < count_iter {
            let mut clone = atom.clone();
            if i >= min as u64 {
                if max_unbounded {
                    clone.states[clone.final_].epsilon1 = Some(clone.initial);
                    nfa_pad_initial(&mut clone);
                    nfa_pad_final(&mut clone);
                }
                clone.states[clone.initial].epsilon1 = Some(clone.final_);
            }
            nfa_concat(&mut atoms, clone);
            if i == u32::MAX as u64 {
                break;
            }
            i += 1;
        }
        return Ok(atoms);
    }

    Ok(atom)
}

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'~') {
        ctx.pos += 1;
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
        ctx.pos += 1;
        let mut alt = parse_term(ctx)?;

        // De Morgan
        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        // Pattern: pad initial of re, pad final of alt
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Append alt's states into re with offset
        let offset = re.states.len();
        let alt_initial = alt.initial + offset;
        let alt_final = alt.final_ + offset;
        for mut st in alt.states.into_iter() {
            shift_option(&mut st.target, offset);
            shift_option(&mut st.epsilon0, offset);
            shift_option(&mut st.epsilon1, offset);
            re.states.push(st);
        }

        // re.initial.epsilon1 = alt.initial
        re.states[re.initial].epsilon1 = Some(alt_initial);
        // re.final.epsilon0 = alt.final
        re.states[re.final_].epsilon0 = Some(alt_final);
        re.final_ = alt_final;

        if intersect {
            re.complemented = !re.complemented;
        }
    }
    Ok(re)
}

pub fn ltre_parse(regex: &str) -> Result<Nfa, String> {
    let mut ctx = ParseContext::new(regex);
    let nfa = parse_regex(&mut ctx)?;
    if !ctx.is_eof() {
        return Err("expected end of input".to_string());
    }
    Ok(nfa)
}

pub fn ltre_fixed_string(s: &str) -> Nfa {
    let mut nfa = Nfa::new_single();
    for &b in s.as_bytes() {
        // Add a new state, connect current final to it via labeled transition
        let new_idx = nfa.states.len();
        nfa.states.push(NState::new());
        let cur_final = nfa.final_;
        nfa.states[cur_final].target = Some(new_idx);
        nfa.states[cur_final].label.insert(b);
        nfa.final_ = new_idx;
    }
    nfa
}

pub fn ltre_partial(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    nfa_pad_initial(nfa);
    nfa_pad_final(nfa);
    let init = nfa.initial;
    let fin = nfa.final_;
    nfa.states[init].target = Some(init);
    nfa.states[fin].target = Some(fin);
    nfa.states[init].label = SymSet::full();
    nfa.states[fin].label = SymSet::full();
    Ok(())
}

pub fn ltre_ignorecase(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    for st in nfa.states.iter_mut() {
        for chr in 0..=255u8 {
            if st.label.contains(chr) {
                st.label.insert(to_lower(chr));
                st.label.insert(to_upper(chr));
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    (bs[idx / 8] >> (idx % 8)) & 1 != 0
}
fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx / 8] |= 1 << (idx % 8);
}

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    if bitset_test(bitset, st_id) {
        return;
    }
    bitset_set(bitset, st_id);
    if let Some(e) = nfa.states[st_id].epsilon0 {
        epsilon_closure_into(nfa, e, bitset);
    }
    if let Some(e) = nfa.states[st_id].epsilon1 {
        epsilon_closure_into(nfa, e, bitset);
    }
}

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let bs_size = (nfa_size + 7) / 8;
    let mut bs = vec![0u8; bs_size];
    epsilon_closure_into(nfa, start, &mut bs);
    bs
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;
    let mut out = vec![0u8; bs_size];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) && nfa.states[id].label.contains(chr) {
            if let Some(t) = nfa.states[id].target {
                epsilon_closure_into(nfa, t, &mut out);
            }
        }
    }
    out
}

fn find_or_create_dead(_states: &mut Vec<DState>) -> usize {
    0 // unused helper
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    let mut states: Vec<DState> = Vec::new();
    // initial state: epsilon-closure of nfa.initial
    let initial_bs = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let mut initial_accepting = bitset_test(&initial_bs, nfa.final_);
    if nfa.complemented {
        initial_accepting = !initial_accepting;
    }
    states.push(DState {
        transitions: [0; 256],
        accepting: initial_accepting,
        terminating: false,
        bitset: initial_bs,
    });

    let mut idx = 0;
    while idx < states.len() {
        // For each char compute next bitset and find/create state
        for chr in 0..256 {
            let cur_bs = states[idx].bitset.clone();
            let next_bs = step_powerset(&nfa, &cur_bs, chr as u8);
            // search for existing state with same bitset
            let mut found = None;
            for (i, s) in states.iter().enumerate() {
                if s.bitset == next_bs {
                    found = Some(i);
                    break;
                }
            }
            let target_idx = if let Some(i) = found {
                i
            } else {
                let mut acc = bitset_test(&next_bs, nfa.final_);
                if nfa.complemented {
                    acc = !acc;
                }
                let new_state = DState {
                    transitions: [0; 256],
                    accepting: acc,
                    terminating: false,
                    bitset: next_bs,
                };
                states.push(new_state);
                states.len() - 1
            };
            states[idx].transitions[chr] = target_idx;
        }
        idx += 1;
    }

    let mut dfa = Dfa { states, initial: 0 };
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }
    // distinguishability matrix: dis[i][j]
    let mut dis = vec![vec![false; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                dis[i][j] = true;
                dis[j][i] = true;
            }
        }
    }
    let mut done = false;
    while !done {
        done = true;
        for i in 0..n {
            for j in (i + 1)..n {
                if dis[i][j] {
                    continue;
                }
                for chr in 0..256 {
                    let ti = dfa.states[i].transitions[chr];
                    let tj = dfa.states[j].transitions[chr];
                    if ti != tj && dis[ti][tj] {
                        dis[i][j] = true;
                        dis[j][i] = true;
                        done = false;
                        break;
                    }
                }
            }
        }
    }

    // Merge indistinguishable states. Build a representative map: each state -> its smallest equivalent index.
    let mut rep: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in 0..i {
            if !dis[i][j] {
                // i is equivalent to j (j < i). Use j's rep.
                rep[i] = rep[j];
                break;
            }
        }
    }

    // Determine which indices are kept (rep[i] == i)
    let kept: Vec<usize> = (0..n).filter(|&i| rep[i] == i).collect();
    let mut new_index_of = vec![0usize; n];
    for (new_i, &old_i) in kept.iter().enumerate() {
        new_index_of[old_i] = new_i;
    }
    // Map any state to its new (kept-rep) index
    let map = |old: usize| -> usize { new_index_of[rep[old]] };

    let mut new_states: Vec<DState> = Vec::with_capacity(kept.len());
    for &old_i in kept.iter() {
        let mut s = dfa.states[old_i].clone();
        for chr in 0..256 {
            s.transitions[chr] = map(s.transitions[chr]);
        }
        new_states.push(s);
    }

    // Compute terminating: state is terminating if all transitions point to itself
    for (i, s) in new_states.iter_mut().enumerate() {
        let mut term = true;
        for chr in 0..256 {
            if s.transitions[chr] != i {
                term = false;
                break;
            }
        }
        s.terminating = term;
    }

    let new_initial = map(dfa.initial);
    dfa.states = new_states;
    dfa.initial = new_initial;
}

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut idx = dfa.initial;
    for &b in input {
        if dfa.states[idx].terminating {
            break;
        }
        idx = dfa.states[idx].transitions[b as usize];
    }
    dfa.states[idx].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    if dfap.is_none() {
        *dfap = Some(ltre_compile(nfa.clone()));
    }
    let dfa = dfap.as_ref().unwrap();
    ltre_matches(dfa, input)
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    // Initial NFA: 2 + dfa_size states (initial, final, plus one per dfa state).
    // Then we may add more for "binary tree" extras during the loop.
    let mut nfa = Nfa {
        states: Vec::new(),
        initial: 0,
        final_: 1,
        complemented: false,
    };
    nfa.states.push(NState::new()); // initial = 0
    nfa.states.push(NState::new()); // final = 1
    let dfa_state_offset = nfa.states.len(); // 2
    let nstates: Vec<usize> = (0..dfa_size)
        .map(|_| {
            nfa.states.push(NState::new());
            nfa.states.len() - 1
        })
        .collect();

    // initial.epsilon1 = nstates[dfa.initial]
    nfa.states[0].epsilon1 = Some(nstates[dfa.initial]);

    // accepting states get epsilon1 -> final
    for (i, ds) in dfa.states.iter().enumerate() {
        if ds.accepting {
            nfa.states[nstates[i]].epsilon1 = Some(nfa.final_);
        }
    }

    // For each dfa state ds1, build labeled transitions to other dfa states.
    // Each labeled transition needs its own nstate (since nstate has only one labeled target).
    // Use binary tree of epsilons rooted at nstates[ds1].
    for ds1 in 0..dfa_size {
        // Find unique target states from ds1
        let mut targets: Vec<(usize, SymSet)> = Vec::new();
        for ds2 in 0..dfa_size {
            let mut symset = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[ds1].transitions[chr] == ds2 {
                    symset.insert(chr as u8);
                    empty = false;
                }
            }
            if !empty {
                targets.push((ds2, symset));
            }
        }

        // Use the existing nstate at nstates[ds1] as the "root" / first source.
        // For each subsequent transition, allocate a new nstate.
        let mut free_idx: Option<usize> = None;
        for (ti, (ds2, symset)) in targets.into_iter().enumerate() {
            let src;
            if ti == 0 {
                src = nstates[ds1];
                free_idx = Some(src);
            } else {
                // allocate new state
                let new_idx = nfa.states.len();
                nfa.states.push(NState::new());

                let f = free_idx.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_idx);
                    // stay
                } else {
                    nfa.states[f].epsilon0 = Some(new_idx);
                    free_idx = Some(new_idx);
                }
                src = new_idx;
            }
            nfa.states[src].target = Some(nstates[ds2]);
            nfa.states[src].label = symset;
        }
    }

    // Suppress dead-code warning
    let _ = dfa_state_offset;

    nfa
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    enum Prec {
        Alt = 0,
        Concat = 1,
        Quant = 2,
        Symset = 3,
    }
    #[derive(Clone)]
    struct Arrow {
        label: Option<String>, // None = empty /[]/, Some("") = epsilon /()/
        prec: Prec,
    }

    let dfa_size = dfa.states.len();
    let n = dfa_size + 1;
    let aux = dfa_size;

    let mut arrows: Vec<Vec<Arrow>> =
        vec![
            vec![
                Arrow {
                    label: None,
                    prec: Prec::Symset
                };
                n
            ];
            n
        ];

    // epsilon transition from aux to dfa.initial
    arrows[aux][dfa.initial] = Arrow {
        label: Some(String::new()),
        prec: Prec::Symset,
    };

    for ds1 in 0..dfa_size {
        if dfa.states[ds1].accepting {
            arrows[ds1][aux] = Arrow {
                label: Some(String::new()),
                prec: Prec::Symset,
            };
        }
        for ds2 in 0..dfa_size {
            let mut symset = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[ds1].transitions[chr] == ds2 {
                    symset.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let fmt = symset_fmt(&symset);
            arrows[ds1][ds2] = Arrow {
                label: Some(fmt),
                prec: Prec::Symset,
            };
        }
    }

    loop {
        let mut best_fit = usize::MAX;
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
                best_fit = id1;
            }
        }
        if best_fit == usize::MAX {
            break;
        }

        let bf = best_fit;
        for id1 in 0..n {
            if id1 == bf {
                continue;
            }
            for id2 in 0..n {
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

                let in_label = in_arrow.label.as_ref().unwrap();
                let out_label = out_arrow.label.as_ref().unwrap();

                // Determine first/second arrows
                let mut first: Option<Arrow> = None;
                let mut second: Option<Arrow> = None;

                let self_is_empty = self_arrow.label.is_none()
                    || self_arrow.label.as_ref().unwrap().is_empty();

                if self_is_empty {
                    first = Some(in_arrow.clone());
                    second = Some(out_arrow.clone());
                } else {
                    let self_label = self_arrow.label.as_ref().unwrap();
                    // try in.label suffix == self.label
                    let in_len = in_label.len();
                    let self_len = self_label.len();
                    let mut handled = false;
                    if in_arrow.prec >= Prec::Concat
                        && self_arrow.prec >= Prec::Concat
                        && in_len >= self_len
                    {
                        let diff = in_len - self_len;
                        if &in_label[diff..] == self_label.as_str() {
                            // hacky checks
                            let in_bytes = in_label.as_bytes();
                            let mut nevermind = false;
                            if diff >= 1 {
                                let c = in_bytes[diff - 1];
                                if (c == b'^' || c == b'-' || c == b'\\')
                                    && (diff == 1 || in_bytes[diff - 2] != b'\\')
                                {
                                    nevermind = true;
                                }
                            }
                            if !nevermind && diff >= 2 {
                                if &in_bytes[diff - 2..diff] == b"\\x"
                                    && (diff == 2 || in_bytes[diff - 3] != b'\\')
                                {
                                    nevermind = true;
                                }
                            }
                            if !nevermind && diff >= 3 {
                                if &in_bytes[diff - 3..diff - 1] == b"\\x"
                                    && (diff == 3 || in_bytes[diff - 4] != b'\\')
                                {
                                    nevermind = true;
                                }
                            }

                            if !nevermind {
                                let mut s = String::new();
                                let in_pre = &in_label[..diff];
                                if diff != 0 && in_arrow.prec < Prec::Concat {
                                    s.push('(');
                                }
                                s.push_str(in_pre);
                                if diff != 0 && in_arrow.prec < Prec::Concat {
                                    s.push(')');
                                }
                                if self_arrow.prec <= Prec::Quant {
                                    s.push('(');
                                }
                                s.push_str(self_label);
                                if self_arrow.prec <= Prec::Quant {
                                    s.push(')');
                                }
                                s.push('+');
                                first = Some(Arrow {
                                    label: Some(s),
                                    prec: Prec::Concat,
                                });
                                second = Some(out_arrow.clone());
                                handled = true;
                            }
                        }
                    }

                    if !handled {
                        // try out.label prefix == self.label
                        let out_len = out_label.len();
                        if out_arrow.prec >= Prec::Concat
                            && self_arrow.prec >= Prec::Concat
                            && out_len >= self_len
                            && &out_label[..self_len] == self_label.as_str()
                        {
                            let diff = out_len - self_len;
                            let out_post = &out_label[self_len..];
                            let mut s = String::new();
                            if self_arrow.prec <= Prec::Quant {
                                s.push('(');
                            }
                            s.push_str(self_label);
                            if self_arrow.prec <= Prec::Quant {
                                s.push(')');
                            }
                            s.push('+');
                            if diff != 0 && out_arrow.prec < Prec::Concat {
                                s.push('(');
                            }
                            s.push_str(out_post);
                            if diff != 0 && out_arrow.prec < Prec::Concat {
                                s.push(')');
                            }
                            second = Some(Arrow {
                                label: Some(s),
                                prec: Prec::Concat,
                            });
                            first = Some(in_arrow.clone());
                        } else {
                            // (in)(self)*(out)
                            let mut s = String::new();
                            if self_arrow.prec <= Prec::Quant {
                                s.push('(');
                            }
                            s.push_str(self_label);
                            if self_arrow.prec <= Prec::Quant {
                                s.push(')');
                            }
                            s.push('*');
                            if out_arrow.prec < Prec::Concat {
                                s.push('(');
                            }
                            s.push_str(out_label);
                            if out_arrow.prec < Prec::Concat {
                                s.push(')');
                            }
                            second = Some(Arrow {
                                label: Some(s),
                                prec: Prec::Concat,
                            });
                            first = Some(in_arrow.clone());
                        }
                    }
                }
                let first = first.unwrap();
                let second = second.unwrap();

                // Concatenate first and second to form bypass
                let bypass: Arrow;
                let first_label = first.label.as_ref().unwrap();
                let second_label = second.label.as_ref().unwrap();
                if first_label.is_empty() {
                    bypass = second.clone();
                } else if second_label.is_empty() {
                    bypass = first.clone();
                } else {
                    let mut s = String::new();
                    if first.prec < Prec::Concat {
                        s.push('(');
                    }
                    s.push_str(first_label);
                    if first.prec < Prec::Concat {
                        s.push(')');
                    }
                    if second.prec < Prec::Concat {
                        s.push('(');
                    }
                    s.push_str(second_label);
                    if second.prec < Prec::Concat {
                        s.push(')');
                    }
                    bypass = Arrow {
                        label: Some(s),
                        prec: Prec::Concat,
                    };
                }

                // Merge with existing
                let merged: Arrow;
                if bypass.label.is_none() {
                    merged = existing.clone();
                } else if existing.label.is_none() {
                    merged = bypass.clone();
                } else if existing.label.as_ref().unwrap().is_empty() {
                    // ()|(bypass) == (bypass)?
                    let bp_label = bypass.label.as_ref().unwrap();
                    let mut s = String::new();
                    if bypass.prec <= Prec::Quant {
                        s.push('(');
                    }
                    s.push_str(bp_label);
                    if bypass.prec <= Prec::Quant {
                        s.push(')');
                    }
                    s.push('?');
                    merged = Arrow {
                        label: Some(s),
                        prec: Prec::Quant,
                    };
                } else {
                    // (existing)|(bypass)
                    let ex_label = existing.label.as_ref().unwrap();
                    let bp_label = bypass.label.as_ref().unwrap();
                    let mut s = String::new();
                    s.push_str(ex_label);
                    s.push('|');
                    s.push_str(bp_label);
                    merged = Arrow {
                        label: Some(s),
                        prec: Prec::Alt,
                    };
                }

                arrows[id1][id2] = merged;
            }
        }

        // clear arrows to/from best_fit
        for id in 0..n {
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

    match &arrows[aux][aux].label {
        Some(s) => s.clone(),
        None => "[]".to_string(),
    }
}
