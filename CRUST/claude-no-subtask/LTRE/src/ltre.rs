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

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn c_isprint(c: u8) -> bool {
    c >= 0x20 && c < 0x7f
}

fn c_isdigit(c: u8) -> bool {
    c >= b'0' && c <= b'9'
}

fn c_isxdigit(c: u8) -> bool {
    c_isdigit(c) || (c >= b'a' && c <= b'f') || (c >= b'A' && c <= b'F')
}

fn c_isalpha(c: u8) -> bool {
    (c >= b'a' && c <= b'z') || (c >= b'A' && c <= b'Z')
}

fn c_isalnum(c: u8) -> bool {
    c_isalpha(c) || c_isdigit(c)
}

fn c_isspace(c: u8) -> bool {
    c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' || c == 0x0b || c == 0x0c
}

fn c_tolower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        c + 32
    } else {
        c
    }
}

fn c_toupper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' {
        c - 32
    } else {
        c
    }
}

pub fn symset_fmt(set: &SymSet) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;

    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    let mut chr: i32 = 0;
    while chr < 256 {
        loop {
            let in_set = set.contains(chr as u8);
            if in_set {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            let im = is_metachar(chr as u8);
            {
                let target = if in_set { &mut buf } else { &mut nbuf };
                if !c_isprint(chr as u8) && !im {
                    let s = format!("\\x{:02x}", chr as u8);
                    target.extend_from_slice(s.as_bytes());
                } else {
                    if im {
                        target.push(b'\\');
                    }
                    target.push(chr as u8);
                }
            }

            let start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - start >= 2 {
                let target = if in_set { &mut buf } else { &mut nbuf };
                target.push(b'-');
                if in_set {
                    nsym -= 1;
                } else {
                    nnsym -= 1;
                }
            }
            if chr - start >= 1 {
                continue;
            }
            break;
        }
        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        buf.pop();
        return String::from_utf8(buf[1..].to_vec()).unwrap();
    } else if nnsym == 1 {
        nbuf.pop();
        nbuf[1] = b'^';
        return String::from_utf8(nbuf[1..].to_vec()).unwrap();
    }

    let chosen = if buf.len() < nbuf.len() { buf } else { nbuf };
    String::from_utf8(chosen).unwrap()
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
        Nfa {
            states: vec![NState::new()],
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

pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ {
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        return;
    }

    let n2_init = nfa2.initial;
    let nfa1_final = nfa1.final_;
    let base = nfa1.states.len();

    let map_idx = |i: usize| -> usize {
        if i == n2_init {
            nfa1_final
        } else if i < n2_init {
            base + i
        } else {
            base + i - 1
        }
    };

    let map_opt = |o: Option<usize>| -> Option<usize> { o.map(map_idx) };

    let init_state = nfa2.states[n2_init].clone();
    nfa1.states[nfa1_final] = NState {
        label: init_state.label,
        target: map_opt(init_state.target),
        epsilon0: map_opt(init_state.epsilon0),
        epsilon1: map_opt(init_state.epsilon1),
    };

    for (i, state) in nfa2.states.iter().enumerate() {
        if i == n2_init {
            continue;
        }
        nfa1.states.push(NState {
            label: state.label,
            target: map_opt(state.target),
            epsilon0: map_opt(state.epsilon0),
            epsilon1: map_opt(state.epsilon1),
        });
    }

    nfa1.final_ = map_idx(nfa2.final_);
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    let old_initial = nfa.initial;
    nfa.states.push(NState {
        label: SymSet::empty(),
        target: None,
        epsilon0: Some(old_initial),
        epsilon1: None,
    });
    nfa.initial = new_idx;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    let old_final = nfa.final_;
    nfa.states.push(NState {
        label: SymSet::empty(),
        target: None,
        epsilon0: None,
        epsilon1: None,
    });
    nfa.states[old_final].epsilon0 = Some(new_idx);
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
        buf.push(((n & 0x7f) | 0x80) as u8);
        n >>= 7;
    }
    buf.push(n as u8);
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: i32 = 0;
    let mut c: u32 = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128: out of bounds".to_string());
        }
        let b = buf[*p];
        *p += 1;
        n |= ((b & 0x7f) as i32) << (c * 7);
        c += 1;
        if b & 0x80 == 0 {
            break;
        }
    }
    Ok(n)
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let dfa_size = dfa.states.len() as i32;
    let mut buf: Vec<u8> = Vec::new();
    leb128_put(&mut buf, dfa_size);

    for ds in &dfa.states {
        let flags = ((ds.accepting as u8) << 1) | (ds.terminating as u8);
        buf.push(flags);
        let mut chr: usize = 0;
        while chr < 256 {
            let start = chr;
            while chr < 255 && ds.transitions[chr] == ds.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8);
            leb128_put(&mut buf, ds.transitions[chr] as i32);
            chr += 1;
        }
    }
    buf
}

pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p: usize = 0;
    let dfa_size = leb128_get(buf, &mut p)? as usize;

    let mut states: Vec<DState> = (0..dfa_size)
        .map(|_| DState {
            transitions: [0usize; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        })
        .collect();

    for id in 0..dfa_size {
        if p >= buf.len() {
            return Err("deserialize: out of bounds".into());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;

        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("deserialize: out of bounds (run)".into());
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

pub fn dfa_dump(_dfa: &Dfa) {}

// ---------------- Parser ----------------

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
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
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

fn cur(ctx: &ParseContext) -> u8 {
    ctx.peek().unwrap_or(0)
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let c = cur(ctx);
    if !c_isdigit(c) {
        return Err("expected natural number".to_string());
    }

    let mut natural: u32 = 0;
    while c_isdigit(cur(ctx)) {
        let digit = (cur(ctx) - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            while c_isdigit(cur(ctx)) {
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
        byte <<= 4;
        let chr = cur(ctx);
        if c_isdigit(chr) {
            byte |= chr - b'0';
        } else if c_isxdigit(chr) {
            byte |= c_tolower(chr) - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.pos += 1;
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = cur(ctx);
    if is_metachar(c) {
        ctx.pos += 1;
        return Ok(c);
    }
    let c = match ctx.next() {
        Some(c) => c,
        None => return Err("unknown escape".to_string()),
    };
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
    if ctx.is_eof() {
        return Err("expected symbol".to_string());
    }
    let c = cur(ctx);
    if c == b'\\' {
        ctx.pos += 1;
        return parse_escape(ctx);
    }
    if is_metachar(c) {
        return Err("unexpected metacharacter".to_string());
    }
    if !c_isprint(c) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.pos += 1;
    Ok(c)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let start = ctx.pos;
    if cur(ctx) == b'\\' {
        ctx.pos += 1;
        let kind = ctx.next();
        if let Some(k) = kind {
            let mut s = SymSet::empty();
            match k {
                b'd' => {
                    for c in 0..=255u32 {
                        if c_isdigit(c as u8) {
                            s.insert(c as u8);
                        }
                    }
                    return Ok(s);
                }
                b'D' => {
                    for c in 0..=255u32 {
                        if !c_isdigit(c as u8) {
                            s.insert(c as u8);
                        }
                    }
                    return Ok(s);
                }
                b's' => {
                    for c in 0..=255u32 {
                        if c_isspace(c as u8) {
                            s.insert(c as u8);
                        }
                    }
                    return Ok(s);
                }
                b'S' => {
                    for c in 0..=255u32 {
                        if !c_isspace(c as u8) {
                            s.insert(c as u8);
                        }
                    }
                    return Ok(s);
                }
                b'w' => {
                    for c in 0..=255u32 {
                        if c as u8 == b'_' || c_isalnum(c as u8) {
                            s.insert(c as u8);
                        }
                    }
                    return Ok(s);
                }
                b'W' => {
                    for c in 0..=255u32 {
                        if c as u8 != b'_' && !c_isalnum(c as u8) {
                            s.insert(c as u8);
                        }
                    }
                    return Ok(s);
                }
                _ => {}
            }
        }
        ctx.pos = start;
    }

    if cur(ctx) == b'.' {
        ctx.pos += 1;
        let mut s = SymSet::empty();
        for c in 0..=255u32 {
            if c as u8 != b'\n' {
                s.insert(c as u8);
            }
        }
        return Ok(s);
    }

    Err("expected shorthand class".to_string())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if cur(ctx) == b'^' {
        ctx.pos += 1;
        complement = true;
    }

    let last_pos = ctx.pos;

    let saved_pos = ctx.pos;
    match parse_shorthand(ctx) {
        Ok(mut s) => {
            if complement {
                s.invert();
            }
            return Ok(s);
        }
        Err(_) => {
            ctx.pos = saved_pos;
        }
    }

    if cur(ctx) == b'[' {
        ctx.pos += 1;
        let mut symset = SymSet::empty();
        while cur(ctx) != b']' {
            if ctx.is_eof() {
                return Err("expected ']'".to_string());
            }
            let sub = parse_symset(ctx)?;
            symset.union_with(&sub);
        }
        if cur(ctx) != b']' {
            return Err("expected ']'".to_string());
        }
        ctx.pos += 1;
        if complement {
            symset.invert();
        }
        return Ok(symset);
    }
    ctx.pos = last_pos;

    if cur(ctx) == b'<' {
        ctx.pos += 1;
        let mut symset = SymSet::full();
        while cur(ctx) != b'>' {
            if ctx.is_eof() {
                return Err("expected '>'".to_string());
            }
            let sub = parse_symset(ctx)?;
            symset.intersect_with(&sub);
        }
        if cur(ctx) != b'>' {
            return Err("expected '>'".to_string());
        }
        ctx.pos += 1;
        if complement {
            symset.invert();
        }
        return Ok(symset);
    }
    ctx.pos = last_pos;

    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if cur(ctx) == b'-' {
        ctx.pos += 1;
        end = parse_symbol(ctx)?;
    }
    let mut symset = SymSet::empty();
    let mut chr = begin;
    let end_open = end.wrapping_add(1);
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

fn parse_atom(ctx: &mut ParseContext) -> Result<Nfa, String> {
    if cur(ctx) == b'(' {
        ctx.pos += 1;
        let sub = parse_regex(ctx)?;
        if cur(ctx) != b')' {
            return Err("expected ')'".to_string());
        }
        ctx.pos += 1;
        return Ok(sub);
    }

    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    let label = parse_symset(ctx)?;
    nfa.states[0].label = label;
    nfa.states[0].target = Some(1);
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    let c = cur(ctx);
    if c == b'*' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let f = atom.final_;
        let i = atom.initial;
        atom.states[f].epsilon1 = Some(i);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        let init = atom.initial;
        let fin = atom.final_;
        atom.states[init].epsilon1 = Some(fin);
        return Ok(atom);
    }
    if c == b'+' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let f = atom.final_;
        let i = atom.initial;
        atom.states[f].epsilon1 = Some(i);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        return Ok(atom);
    }
    if c == b'?' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let init = atom.initial;
        if atom.states[init].epsilon1.is_some() {
            nfa_pad_initial(&mut atom);
        }
        let init = atom.initial;
        let fin = atom.final_;
        atom.states[init].epsilon1 = Some(fin);
        return Ok(atom);
    }

    if c == b'{' {
        let last_pos = ctx.pos;
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;

        let mut min: u32 = match parse_natural(ctx) {
            Ok(v) => v,
            Err(e) => {
                if e == "natural number overflow" {
                    return Err(e);
                }
                0
            }
        };

        let mut max: u32 = min;
        let mut max_unbounded = false;
        if cur(ctx) == b',' {
            ctx.pos += 1;
            match parse_natural(ctx) {
                Ok(v) => {
                    max = v;
                }
                Err(e) => {
                    if e == "natural number overflow" {
                        return Err(e);
                    }
                    max_unbounded = true;
                }
            }
        }

        if cur(ctx) != b'}' {
            return Err("expected '}'".to_string());
        }
        ctx.pos += 1;

        if min > max && !max_unbounded {
            ctx.pos = last_pos;
            return Err("misbounded quantifier".to_string());
        }

        let mut atoms = Nfa::new_single();

        let mut i: u32 = 0;
        loop {
            let cond_continue = if max_unbounded { i <= min } else { i < max };
            if !cond_continue {
                break;
            }

            let mut clone = nfa_clone(&atom);
            if i >= min {
                if max_unbounded {
                    let f = clone.final_;
                    let init = clone.initial;
                    clone.states[f].epsilon1 = Some(init);
                    nfa_pad_initial(&mut clone);
                    nfa_pad_final(&mut clone);
                }
                let init = clone.initial;
                let fin = clone.final_;
                clone.states[init].epsilon1 = Some(fin);
            }

            nfa_concat(&mut atoms, clone);

            if i == u32::MAX {
                break;
            }
            i += 1;
        }

        let _ = min; // silence
        let _ = max;
        return Ok(atoms);
    }

    Ok(atom)
}

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut complement = false;
    if cur(ctx) == b'~' {
        ctx.pos += 1;
        complement = true;
    }

    let mut term = Nfa::new_single();

    while !matches!(cur(ctx), b')' | b'|' | b'&') && !ctx.is_eof() {
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

    while matches!(cur(ctx), b'|' | b'&') {
        let intersect = cur(ctx) == b'&';
        ctx.pos += 1;
        let mut alt = parse_term(ctx)?;

        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        let alt_initial_old = alt.initial;
        let alt_final_old = alt.final_;
        let base = re.states.len();
        for state in alt.states.iter() {
            re.states.push(NState {
                label: state.label,
                target: state.target.map(|x| x + base),
                epsilon0: state.epsilon0.map(|x| x + base),
                epsilon1: state.epsilon1.map(|x| x + base),
            });
        }
        let alt_initial_new = alt_initial_old + base;
        let alt_final_new = alt_final_old + base;

        let re_initial = re.initial;
        let re_final = re.final_;
        re.states[re_initial].epsilon1 = Some(alt_initial_new);
        re.states[re_final].epsilon0 = Some(alt_final_new);
        re.final_ = alt_final_new;

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
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Nfa::new_single();
    }
    let n = bytes.len();
    let mut states: Vec<NState> = (0..=n).map(|_| NState::new()).collect();
    for i in 0..n {
        states[i].label.insert(bytes[i]);
        states[i].target = Some(i + 1);
    }
    Nfa {
        states,
        initial: 0,
        final_: n,
        complemented: false,
    }
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
    for state in nfa.states.iter_mut() {
        let mut to_add: Vec<u8> = Vec::new();
        for c in 0..=255u32 {
            if state.label.contains(c as u8) {
                to_add.push(c_tolower(c as u8));
                to_add.push(c_toupper(c as u8));
            }
        }
        for c in to_add {
            state.label.insert(c);
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

// --- Powerset construction & DFA compile ---

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
    let s = &nfa.states[st_id];
    if let Some(e0) = s.epsilon0 {
        epsilon_closure_into(nfa, e0, bitset);
    }
    if let Some(e1) = s.epsilon1 {
        epsilon_closure_into(nfa, e1, bitset);
    }
}

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let mut bs = vec![0u8; (nfa_size + 7) / 8];
    epsilon_closure_into(nfa, start, &mut bs);
    bs
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let mut out = vec![0u8; (nfa_size + 7) / 8];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) {
            let st = &nfa.states[id];
            if st.label.contains(chr) {
                if let Some(t) = st.target {
                    epsilon_closure_into(nfa, t, &mut out);
                }
            }
        }
    }
    out
}

fn find_or_create_dead(_states: &mut Vec<DState>) -> usize {
    0
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();

    let mut states: Vec<DState> = Vec::new();
    let initial_bs = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    {
        let mut accepting = bitset_test(&initial_bs, nfa.final_);
        accepting ^= nfa.complemented;
        states.push(DState {
            transitions: [0usize; 256],
            accepting,
            terminating: false,
            bitset: initial_bs,
        });
    }

    let mut i = 0usize;
    while i < states.len() {
        for chr in 0..256usize {
            let next_bs = step_powerset(&nfa, &states[i].bitset, chr as u8);
            let mut found: Option<usize> = None;
            for (j, st) in states.iter().enumerate() {
                if st.bitset == next_bs {
                    found = Some(j);
                    break;
                }
            }
            let target = if let Some(j) = found {
                j
            } else {
                let mut accepting = bitset_test(&next_bs, nfa.final_);
                accepting ^= nfa.complemented;
                let new_idx = states.len();
                states.push(DState {
                    transitions: [0usize; 256],
                    accepting,
                    terminating: false,
                    bitset: next_bs,
                });
                new_idx
            };
            states[i].transitions[chr] = target;
        }
        i += 1;
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
    let row_size = (n + 7) / 8;
    let mut dis: Vec<Vec<u8>> = vec![vec![0u8; row_size]; n];

    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                bitset_set(&mut dis[i], j);
                bitset_set(&mut dis[j], i);
            }
        }
    }

    let mut done = false;
    while !done {
        done = true;
        for id1 in 0..n {
            for id2 in (id1 + 1)..n {
                if !bitset_test(&dis[id1], id2) {
                    for chr in 0..256 {
                        let t1 = dfa.states[id1].transitions[chr];
                        let t2 = dfa.states[id2].transitions[chr];
                        if t1 != t2 && bitset_test(&dis[t1], t2) {
                            bitset_set(&mut dis[id1], id2);
                            bitset_set(&mut dis[id2], id1);
                            done = false;
                            break;
                        }
                    }
                }
            }
        }
    }

    let mut rep: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in 0..i {
            if !bitset_test(&dis[i], j) {
                rep[i] = rep[j];
                break;
            }
        }
    }

    let mut new_index: Vec<Option<usize>> = vec![None; n];
    let mut new_states: Vec<DState> = Vec::new();
    for i in 0..n {
        if rep[i] == i {
            new_index[i] = Some(new_states.len());
            new_states.push(dfa.states[i].clone());
        }
    }

    for st in new_states.iter_mut() {
        for chr in 0..256 {
            let old = st.transitions[chr];
            let canonical = rep[old];
            st.transitions[chr] = new_index[canonical].unwrap();
        }
    }

    for (idx, st) in new_states.iter_mut().enumerate() {
        let mut term = true;
        for chr in 0..256 {
            if st.transitions[chr] != idx {
                term = false;
                break;
            }
        }
        st.terminating = term;
    }

    let new_initial = new_index[rep[dfa.initial]].unwrap();
    dfa.states = new_states;
    dfa.initial = new_initial;
}

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut idx = dfa.initial;
    for &c in input {
        if dfa.states[idx].terminating {
            break;
        }
        idx = dfa.states[idx].transitions[c as usize];
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

    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };

    let mut nstates_idx: Vec<usize> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        nstates_idx.push(nfa.states.len());
        nfa.states.push(NState::new());
    }

    let nfa_initial = nfa.initial;
    let nfa_final = nfa.final_;
    nfa.states[nfa_initial].epsilon1 = Some(nstates_idx[dfa.initial]);

    for i in 0..dfa_size {
        if dfa.states[i].accepting {
            nfa.states[nstates_idx[i]].epsilon1 = Some(nfa_final);
        }
    }

    for id1 in 0..dfa_size {
        let mut free_node: Option<usize> = None;

        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if dfa.states[id1].transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }

            let src: usize;
            if free_node.is_none() {
                free_node = Some(nstates_idx[id1]);
                src = nstates_idx[id1];
            } else {
                let new_idx = nfa.states.len();
                nfa.states.push(NState::new());
                src = new_idx;

                let f = free_node.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_idx);
                } else {
                    nfa.states[f].epsilon0 = Some(new_idx);
                    free_node = Some(new_idx);
                }
            }

            nfa.states[src].target = Some(nstates_idx[id2]);
            nfa.states[src].label = transitions;
        }
    }

    nfa
}

// --- Decompile ---
#[derive(PartialEq, Eq, Clone, Copy, PartialOrd, Ord, Debug)]
enum Prec {
    Alt = 0,
    Concat = 1,
    Quant = 2,
    Symset = 3,
}

#[derive(Clone, Debug)]
struct Arrow {
    label: Option<String>, // None=>[], Some("")=>()
    prec: Prec,
}

fn dec_concat(first: &Arrow, second: &Arrow) -> Arrow {
    // (in)(self)*(out) etc (the 'first' / 'second' already have *).
    // Implements: bypass = first . second
    if first.label.is_none() || second.label.is_none() {
        return Arrow {
            label: None,
            prec: Prec::Symset,
        };
    }
    let f = first.label.as_ref().unwrap();
    let s = second.label.as_ref().unwrap();
    if f.is_empty() {
        return second.clone();
    }
    if s.is_empty() {
        return first.clone();
    }
    let mut p = String::new();
    if first.prec < Prec::Concat {
        p.push('(');
    }
    p.push_str(f);
    if first.prec < Prec::Concat {
        p.push(')');
    }
    if second.prec < Prec::Concat {
        p.push('(');
    }
    p.push_str(s);
    if second.prec < Prec::Concat {
        p.push(')');
    }
    Arrow {
        label: Some(p),
        prec: Prec::Concat,
    }
}

fn dec_alt(existing: &Arrow, bypass: &Arrow) -> Arrow {
    if bypass.label.is_none() {
        return existing.clone();
    }
    if existing.label.is_none() {
        return bypass.clone();
    }
    let e = existing.label.as_ref().unwrap();
    let b = bypass.label.as_ref().unwrap();
    if e.is_empty() {
        // ()|(bypass) == (bypass)?
        let mut p = String::new();
        if bypass.prec <= Prec::Quant {
            p.push('(');
        }
        p.push_str(b);
        if bypass.prec <= Prec::Quant {
            p.push(')');
        }
        p.push('?');
        return Arrow {
            label: Some(p),
            prec: Prec::Quant,
        };
    }
    let mut p = String::new();
    p.push_str(e);
    p.push('|');
    p.push_str(b);
    Arrow {
        label: Some(p),
        prec: Prec::Alt,
    }
}

fn compute_first_second(in_arr: &Arrow, out_arr: &Arrow, self_arr: &Arrow) -> (Arrow, Arrow) {
    let in_label = in_arr.label.as_ref().unwrap();
    let out_label = out_arr.label.as_ref().unwrap();

    // case: self is None ([]) or "" ()
    if self_arr.label.is_none() || self_arr.label.as_ref().unwrap().is_empty() {
        return (in_arr.clone(), out_arr.clone());
    }
    let self_label = self_arr.label.as_ref().unwrap();

    let in_b = in_label.as_bytes();
    let out_b = out_label.as_bytes();
    let s_b = self_label.as_bytes();

    // Try first: in.label = in_pre + self.label  =>  (in_pre)(self)+(out)
    let mut chose_first: Option<Arrow> = None;
    let mut chose_first_nevermind = false;
    if in_arr.prec >= Prec::Concat && self_arr.prec >= Prec::Concat && in_b.len() >= s_b.len() {
        let diff = in_b.len() - s_b.len();
        if &in_b[diff..] == s_b {
            // sanity checks
            let mut nevermind = false;
            if diff >= 1
                && b"^-\\".contains(&in_b[diff - 1])
                && (diff == 1 || in_b[diff - 2] != b'\\')
            {
                nevermind = true;
            }
            if !nevermind
                && diff >= 2
                && &in_b[diff - 2..diff] == b"\\x"
                && (diff == 2 || in_b[diff - 3] != b'\\')
            {
                nevermind = true;
            }
            if !nevermind
                && diff >= 3
                && &in_b[diff - 3..diff - 1] == b"\\x"
                && (diff == 3 || in_b[diff - 4] != b'\\')
            {
                nevermind = true;
            }
            if !nevermind {
                let mut p = String::new();
                if diff != 0 && in_arr.prec < Prec::Concat {
                    p.push('(');
                }
                p.push_str(std::str::from_utf8(&in_b[..diff]).unwrap());
                if diff != 0 && in_arr.prec < Prec::Concat {
                    p.push(')');
                }
                if self_arr.prec <= Prec::Quant {
                    p.push('(');
                }
                p.push_str(self_label);
                if self_arr.prec <= Prec::Quant {
                    p.push(')');
                }
                p.push('+');
                chose_first = Some(Arrow {
                    label: Some(p),
                    prec: Prec::Concat,
                });
            } else {
                chose_first_nevermind = true;
            }
        }
    }
    if let Some(first) = chose_first {
        return (first, out_arr.clone());
    }

    // The 'else' fall-through behavior in C — including when 'nevermind' is hit —
    // tries the (out)/second variant first. (In C, the `nevermind` label is the
    // start of the `if (out.prec >= CONCAT && ...)` block.)
    let _ = chose_first_nevermind;

    // Try second: out.label = self.label + out_post  =>  (in)(self)+(out_post)
    if out_arr.prec >= Prec::Concat && self_arr.prec >= Prec::Concat && out_b.len() >= s_b.len() {
        if &out_b[..s_b.len()] == s_b {
            let post = &out_b[s_b.len()..]; // out_post
            let diff = out_b.len() - s_b.len();
            let mut p = String::new();
            if self_arr.prec <= Prec::Quant {
                p.push('(');
            }
            p.push_str(self_label);
            if self_arr.prec <= Prec::Quant {
                p.push(')');
            }
            p.push('+');
            if diff != 0 && out_arr.prec < Prec::Concat {
                p.push('(');
            }
            p.push_str(std::str::from_utf8(post).unwrap());
            if diff != 0 && out_arr.prec < Prec::Concat {
                p.push(')');
            }
            return (
                in_arr.clone(),
                Arrow {
                    label: Some(p),
                    prec: Prec::Concat,
                },
            );
        }
    }

    // Fallback: (in)(self)*(out)
    let mut p = String::new();
    if self_arr.prec <= Prec::Quant {
        p.push('(');
    }
    p.push_str(self_label);
    if self_arr.prec <= Prec::Quant {
        p.push(')');
    }
    p.push('*');
    if out_arr.prec < Prec::Concat {
        p.push('(');
    }
    p.push_str(out_label);
    if out_arr.prec < Prec::Concat {
        p.push(')');
    }
    (
        in_arr.clone(),
        Arrow {
            label: Some(p),
            prec: Prec::Concat,
        },
    )
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let n = dfa_size + 1;

    let none_arrow = Arrow {
        label: None,
        prec: Prec::Symset,
    };
    let mut arrows: Vec<Vec<Arrow>> = vec![vec![none_arrow.clone(); n]; n];

    arrows[dfa_size][dfa.initial] = Arrow {
        label: Some(String::new()),
        prec: Prec::Symset,
    };

    for id1 in 0..dfa_size {
        if dfa.states[id1].accepting {
            arrows[id1][dfa_size] = Arrow {
                label: Some(String::new()),
                prec: Prec::Symset,
            };
        }
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if dfa.states[id1].transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                arrows[id1][id2] = none_arrow.clone();
            } else {
                arrows[id1][id2] = Arrow {
                    label: Some(symset_fmt(&transitions)),
                    prec: Prec::Symset,
                };
            }
        }
    }

    loop {
        let mut best_fit: Option<usize> = None;
        let mut min_degree = i32::MAX;
        for id1 in 0..dfa_size {
            let mut degree: i32 = 0;
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
        let bf = match best_fit {
            Some(b) => b,
            None => break,
        };

        for id1 in 0..n {
            if id1 == bf {
                continue;
            }
            for id2 in 0..n {
                if id2 == bf {
                    continue;
                }
                let in_arr = arrows[id1][bf].clone();
                let out_arr = arrows[bf][id2].clone();
                if in_arr.label.is_none() || out_arr.label.is_none() {
                    continue;
                }
                let self_arr = arrows[bf][bf].clone();
                let existing = arrows[id1][id2].clone();

                let (first, second) = compute_first_second(&in_arr, &out_arr, &self_arr);
                let bypass = dec_concat(&first, &second);
                let merged = dec_alt(&existing, &bypass);
                arrows[id1][id2] = merged;
            }
        }

        for id in 0..n {
            arrows[id][bf] = none_arrow.clone();
            arrows[bf][id] = none_arrow.clone();
        }
    }

    match arrows[dfa_size][dfa_size].label.clone() {
        Some(s) => s,
        None => "[]".to_string(),
    }
}
