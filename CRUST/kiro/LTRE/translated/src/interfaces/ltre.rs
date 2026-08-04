const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymSet {
    bits: [u8; 256 / 8],
}
impl SymSet {
    pub fn empty() -> Self {
        SymSet { bits: [0u8; 32] }
    }
    pub fn full() -> Self {
        SymSet { bits: [0xffu8; 32] }
    }
    pub fn contains(&self, c: u8) -> bool {
        self.bits[c as usize / 8] & (1 << (c as usize % 8)) != 0
    }
    pub fn insert(&mut self, c: u8) {
        self.bits[c as usize / 8] |= 1 << (c as usize % 8);
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

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn is_print(c: u8) -> bool {
    c >= 0x20 && c <= 0x7e
}

pub fn symset_fmt(set: &SymSet) -> String {
    let mut buf = String::new();
    let mut nbuf = String::new();
    let mut nsym = 0i32;
    let mut nnsym = 0i32;

    nbuf.push('^');
    buf.push('[');
    nbuf.push('[');

    let mut chr: i32 = 0;
    while chr < 256 {
        let start = chr;
        let in_set = set.contains(chr as u8);
        // append_chr logic
        loop {
            let cur_in = set.contains(chr as u8);
            if cur_in { nsym += 1; } else { nnsym += 1; }
            let p = if cur_in { &mut buf } else { &mut nbuf };
            let c = chr as u8;
            if !is_print(c) && !is_metachar(c) {
                p.push_str(&format!("\\x{:02x}", c));
            } else {
                if is_metachar(c) {
                    p.push('\\');
                }
                p.push(c as char);
            }

            // make character ranges
            let range_start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - range_start >= 2 {
                let p2 = if cur_in { &mut buf } else { &mut nbuf };
                p2.push('-');
                if cur_in { nsym -= 1; } else { nnsym -= 1; }
            }
            if chr - range_start >= 1 {
                // goto append_chr equivalent: continue the loop with same chr
                continue;
            }
            break;
        }
        chr += 1;
    }

    buf.push(']');
    nbuf.push(']');

    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        // remove surrounding [ and ]
        return buf[1..buf.len()-1].to_string();
    } else if nnsym == 1 {
        // remove first char of nbuf (^), replace with ^
        let inner = &nbuf[1..]; // [...]
        let mut result = String::new();
        result.push('^');
        result.push_str(&inner[1..inner.len()-1]);
        return result;
    }

    if buf.len() < nbuf.len() { buf } else { nbuf }
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
        let mut nfa = Nfa {
            states: Vec::new(),
            initial: 0,
            final_: 0,
            complemented: false,
        };
        nfa.states.push(NState::new());
        nfa
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

fn alloc_state(nfa: &mut Nfa) -> usize {
    let id = nfa.states.len();
    nfa.states.push(NState::new());
    id
}

pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ {
        *nfa1 = nfa2;
    } else if nfa2.initial != nfa2.final_ {
        // In the C code, nfa2.initial is memcpy'd into nfa1.final_ and then freed.
        // We need to remap nfa2 indices: nfa2.initial -> nfa1.final_, all others get offset.
        // Build a mapping from old nfa2 index -> new index in nfa1
        let base = nfa1.states.len();
        let mut remap = vec![0usize; nfa2.states.len()];
        remap[nfa2.initial] = nfa1.final_;
        let mut next_id = base;
        for i in 0..nfa2.states.len() {
            if i != nfa2.initial {
                remap[i] = next_id;
                next_id += 1;
            }
        }

        let map_opt = |opt: Option<usize>| -> Option<usize> { opt.map(|t| remap[t]) };

        // Copy nfa2.initial content into nfa1.final_
        let init2 = &nfa2.states[nfa2.initial];
        nfa1.states[nfa1.final_].label = init2.label;
        nfa1.states[nfa1.final_].target = map_opt(init2.target);
        nfa1.states[nfa1.final_].epsilon0 = map_opt(init2.epsilon0);
        nfa1.states[nfa1.final_].epsilon1 = map_opt(init2.epsilon1);

        // Add all nfa2 states except initial
        for i in 0..nfa2.states.len() {
            if i == nfa2.initial { continue; }
            let st = &nfa2.states[i];
            let mut ns = NState::new();
            ns.label = st.label;
            ns.target = map_opt(st.target);
            ns.epsilon0 = map_opt(st.epsilon0);
            ns.epsilon1 = map_opt(st.epsilon1);
            nfa1.states.push(ns);
        }

        nfa1.final_ = remap[nfa2.final_];
    }
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let new_id = alloc_state(nfa);
    nfa.states[new_id].epsilon0 = Some(nfa.initial);
    nfa.initial = new_id;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_id = alloc_state(nfa);
    nfa.states[nfa.final_].epsilon0 = Some(new_id);
    nfa.final_ = new_id;
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
    for (id, st) in nfa.states.iter().enumerate() {
        if let Some(e0) = st.epsilon0 {
            println!("  {} --> {}", id, e0);
        }
        if let Some(e1) = st.epsilon1 {
            println!("  {} --> {}", id, e1);
        }
        if !st.label.is_empty() {
            print!("  {} --", id);
            let fmt = symset_fmt(&st.label);
            for c in fmt.bytes() {
                if b"\\\"#&{}()xo=- ".contains(&c) {
                    print!("#{};", c);
                } else {
                    print!("{}", c as char);
                }
            }
            if let Some(t) = st.target {
                println!("--> {}", t);
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
        Dfa { states: Vec::new(), initial: 0 }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
}

fn new_dstate(bitset_size: usize) -> DState {
    DState {
        transitions: [0; 256],
        accepting: false,
        terminating: false,
        bitset: vec![0u8; bitset_size],
    }
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf = Vec::new();
    let dfa_size = dfa.states.len() as i32;
    leb128_put(&mut buf, dfa_size);

    // Build order: initial state first, then rest
    let order = dfa_state_order(dfa);

    for &sid in &order {
        let ds = &dfa.states[sid];
        buf.push((ds.accepting as u8) << 1 | ds.terminating as u8);
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            while chr < 255 && ds.transitions[chr] == ds.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8);
            // map state index to order index
            let target_order = order.iter().position(|&x| x == ds.transitions[chr]).unwrap();
            leb128_put(&mut buf, target_order as i32);
            chr += 1;
        }
    }
    buf
}

fn dfa_state_order(dfa: &Dfa) -> Vec<usize> {
    // initial state first, then all others in order
    let mut order = vec![dfa.initial];
    for i in 0..dfa.states.len() {
        if i != dfa.initial {
            order.push(i);
        }
    }
    order
}

pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p = 0usize;
    let dfa_size = leb128_get(buf, &mut p)? as usize;

    let mut dfa = Dfa::new();
    for _ in 0..dfa_size {
        dfa.states.push(new_dstate(0));
    }

    for id in 0..dfa_size {
        let byte = *buf.get(p).ok_or("unexpected end")?;
        p += 1;
        dfa.states[id].accepting = (byte >> 1) & 1 != 0;
        dfa.states[id].terminating = byte & 1 != 0;
        let mut chr = 0usize;
        while chr < 256 {
            let len = *buf.get(p).ok_or("unexpected end")? as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            for _ in 0..=len {
                dfa.states[id].transitions[chr] = target;
                chr += 1;
            }
        }
    }

    dfa.initial = 0;
    Ok((dfa, p))
}

pub fn dfa_dump(dfa: &Dfa) {
    println!("graph LR");
    println!("  I( ) --> {}", dfa.initial);
    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            println!("  {} --> F( )", id1);
        }
        for id2 in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u16 {
                if ds1.transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            print!("  {} --", id1);
            let fmt = symset_fmt(&transitions);
            for c in fmt.bytes() {
                if b"\\\"#&{}()xo=- ".contains(&c) {
                    print!("#{};", c);
                } else {
                    print!("{}", c as char);
                }
            }
            println!("--> {}", id2);
        }
    }
}

fn leb128_put(buf: &mut Vec<u8>, mut n: i32) {
    let mut n = n as u32;
    loop {
        if n >> 7 != 0 {
            buf.push((n as u8 & 0x7f) | 0x80);
            n >>= 7;
        } else {
            buf.push(n as u8);
            break;
        }
    }
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: u32 = 0;
    let mut c = 0u32;
    loop {
        let byte = *buf.get(*p).ok_or("unexpected end in leb128")?;
        *p += 1;
        n |= ((byte & 0x7f) as u32) << (c * 7);
        c += 1;
        if byte & 0x80 == 0 {
            break;
        }
    }
    Ok(n as i32)
}

// ---- Parsing ----

struct ParseContext<'a> {
    chars: &'a [u8],
    pos: usize,
}
impl<'a> ParseContext<'a> {
    fn new(s: &'a str) -> Self {
        ParseContext { chars: s.as_bytes(), pos: 0 }
    }
    fn peek(&self) -> Option<u8> {
        if self.pos < self.chars.len() { Some(self.chars[self.pos]) } else { None }
    }
    fn next(&mut self) -> Option<u8> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }
    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
    fn expect_char(&mut self) -> Result<u8, String> {
        self.next().ok_or_else(|| "unexpected end of input".to_string())
    }
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    if ctx.peek().map_or(true, |c| !c.is_ascii_digit()) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !c.is_ascii_digit() { break; }
        ctx.next();
        let digit = (c - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            return Err("natural number overflow".to_string());
        }
        natural = natural * 10 + digit;
    }
    Ok(natural)
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte = 0u8;
    for _ in 0..2 {
        byte <<= 4;
        let c = ctx.next().ok_or("expected hex digit")?;
        if c.is_ascii_digit() {
            byte |= c - b'0';
        } else if c.is_ascii_hexdigit() {
            byte |= c.to_ascii_lowercase() - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or("expected escape")?;
    if is_metachar(c) {
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
        _ => {
            ctx.pos -= 1;
            Err("unknown escape".to_string())
        }
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        return parse_escape(ctx);
    }
    let c = ctx.peek().ok_or("expected symbol")?;
    if is_metachar(c) {
        return Err("unexpected metacharacter".to_string());
    }
    if !is_print(c) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.next();
    Ok(c)
}

fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0..=255u8 { if c.is_ascii_digit() { s.insert(c); } }
    s
}
fn not_digits_set() -> SymSet {
    let mut s = digits_set(); s.invert(); s
}
fn spaces_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0..=255u8 { if (c as char).is_ascii_whitespace() || c == 0x0b { s.insert(c); } }
    s
}
fn not_spaces_set() -> SymSet {
    let mut s = spaces_set(); s.invert(); s
}
fn wordchar_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0..=255u8 { if c == b'_' || c.is_ascii_alphanumeric() { s.insert(c); } }
    s
}
fn not_wordchar_set() -> SymSet {
    let mut s = wordchar_set(); s.invert(); s
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let saved = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        match ctx.peek() {
            Some(b'd') => { ctx.next(); return Ok(digits_set()); }
            Some(b'D') => { ctx.next(); return Ok(not_digits_set()); }
            Some(b's') => { ctx.next(); return Ok(spaces_set()); }
            Some(b'S') => { ctx.next(); return Ok(not_spaces_set()); }
            Some(b'w') => { ctx.next(); return Ok(wordchar_set()); }
            Some(b'W') => { ctx.next(); return Ok(not_wordchar_set()); }
            _ => { ctx.pos = saved; }
        }
    }
    if ctx.peek() == Some(b'.') {
        ctx.next();
        let mut s = SymSet::full();
        // . matches everything except \n
        s.bits[b'\n' as usize / 8] &= !(1 << (b'\n' as usize % 8));
        return Ok(s);
    }
    ctx.pos = saved;
    Err("expected shorthand class".to_string())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.next();
        complement = true;
    }

    let saved = ctx.pos;

    // try shorthand
    match parse_shorthand(ctx) {
        Ok(mut s) => {
            if complement { s.invert(); }
            return Ok(s);
        }
        Err(_) => { ctx.pos = saved; }
    }

    // try [...]
    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut s = SymSet::empty();
        while ctx.peek() != Some(b']') {
            if ctx.is_eof() {
                return Err("expected ']'".to_string());
            }
            let sub = parse_symset(ctx)?;
            s.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.next();
        if complement { s.invert(); }
        return Ok(s);
    }

    // try <...>
    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut s = SymSet::full();
        while ctx.peek() != Some(b'>') {
            if ctx.is_eof() {
                return Err("expected '>'".to_string());
            }
            let sub = parse_symset(ctx)?;
            s.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.next();
        if complement { s.invert(); }
        return Ok(s);
    }

    // try symbol or range
    let saved2 = ctx.pos;
    match parse_symbol(ctx) {
        Ok(begin) => {
            let mut end = begin;
            if ctx.peek() == Some(b'-') {
                ctx.next();
                end = parse_symbol(ctx)?;
            }
            let end_plus = end.wrapping_add(1);
            let mut s = SymSet::empty();
            let mut c = begin;
            loop {
                s.insert(c);
                c = c.wrapping_add(1);
                if c == end_plus { break; }
            }
            if complement { s.invert(); }
            return Ok(s);
        }
        Err(e) => {
            ctx.pos = saved2;
            return Err(e);
        }
    }
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}
fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(ref mut v) = opt {
        *v += offset;
    }
}

// ---- NFA parsing (regex -> NFA) ----

fn parse_atom(ctx: &mut ParseContext) -> Result<Nfa, String> {
    if ctx.peek() == Some(b'(') {
        ctx.next();
        let sub = parse_regex(ctx)?;
        if ctx.peek() != Some(b')') {
            return Err("expected ')'".to_string());
        }
        ctx.next();
        return Ok(sub);
    }

    // character/symset atom
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

    if ctx.peek() == Some(b'*') {
        ctx.next();
        let _ = nfa_uncomplement(&mut atom);
        atom.states[atom.final_].epsilon1 = Some(atom.initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }

    if ctx.peek() == Some(b'+') {
        ctx.next();
        let _ = nfa_uncomplement(&mut atom);
        atom.states[atom.final_].epsilon1 = Some(atom.initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        return Ok(atom);
    }

    if ctx.peek() == Some(b'?') {
        ctx.next();
        let _ = nfa_uncomplement(&mut atom);
        if atom.states[atom.initial].epsilon1.is_some() {
            nfa_pad_initial(&mut atom);
        }
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }

    let saved = ctx.pos;
    if ctx.peek() == Some(b'{') {
        ctx.next();
        let _ = nfa_uncomplement(&mut atom);

        let min_result = parse_natural(ctx);
        let min;
        match min_result {
            Ok(v) => min = v,
            Err(ref e) if e.contains("overflow") => return Err(e.clone()),
            Err(_) => min = 0,
        }

        let mut max = min;
        let mut max_unbounded = false;
        if ctx.peek() == Some(b',') {
            ctx.next();
            let max_result = parse_natural(ctx);
            match max_result {
                Ok(v) => max = v,
                Err(ref e) if e.contains("overflow") => return Err(e.clone()),
                Err(_) => { max_unbounded = true; }
            }
        }

        if ctx.peek() != Some(b'}') {
            return Err("expected '}'".to_string());
        }
        ctx.next();

        if min > max && !max_unbounded {
            ctx.pos = saved;
            return Err("misbounded quantifier".to_string());
        }

        let mut atoms = Nfa::new_single();
        atoms.complemented = false;

        let limit = if max_unbounded { min + 1 } else { max };
        let mut i: u32 = 0;
        loop {
            if i > limit { break; }
            let mut clone = nfa_clone(&atom);
            if i >= min {
                if max_unbounded {
                    let _ = nfa_uncomplement(&mut clone);
                    clone.states[clone.final_].epsilon1 = Some(clone.initial);
                    nfa_pad_initial(&mut clone);
                    nfa_pad_final(&mut clone);
                }
                clone.states[clone.initial].epsilon1 = Some(clone.final_);
            }
            nfa_concat(&mut atoms, clone);
            if i == u32::MAX { break; }
            i += 1;
        }

        return Ok(atoms);
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
    term.complemented = false;

    while !matches!(ctx.peek(), Some(b')') | Some(b'|') | Some(b'&') | None) {
        let mut factor = parse_factor(ctx)?;
        let _ = nfa_uncomplement(&mut factor);
        nfa_concat(&mut term, factor);
    }

    if complement {
        term.complemented = true;
    }

    Ok(term)
}

fn parse_regex(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut re = parse_term(ctx)?;

    while ctx.peek() == Some(b'|') || ctx.peek() == Some(b'&') {
        let intersect = ctx.peek() == Some(b'&');
        ctx.next();
        let mut alt = parse_term(ctx)?;

        // De Morgan for intersection
        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        let _ = nfa_uncomplement(&mut re);
        let _ = nfa_uncomplement(&mut alt);

        // merge: pad initial of re, pad final of alt
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // offset alt states and merge
        let offset = re.states.len();
        for st in &mut alt.states {
            st.target = st.target.map(|t| t + offset);
            st.epsilon0 = st.epsilon0.map(|t| t + offset);
            st.epsilon1 = st.epsilon1.map(|t| t + offset);
        }
        let alt_initial = alt.initial + offset;
        let alt_final = alt.final_ + offset;

        re.states[re.initial].epsilon1 = Some(alt_initial);
        re.states[re.final_].epsilon0 = Some(alt_final);

        re.states.extend(alt.states);
        re.final_ = alt_final;

        re.complemented ^= intersect;
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
    nfa.complemented = false;
    for &b in s.as_bytes() {
        let old_final = nfa.final_;
        let new_id = alloc_state(&mut nfa);
        nfa.states[old_final].target = Some(new_id);
        nfa.states[old_final].label.insert(b);
        nfa.final_ = new_id;
    }
    nfa
}

pub fn ltre_partial(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    nfa_pad_initial(nfa);
    nfa_pad_final(nfa);
    nfa.states[nfa.initial].target = Some(nfa.initial);
    nfa.states[nfa.final_].target = Some(nfa.final_);
    nfa.states[nfa.initial].label = SymSet::full();
    nfa.states[nfa.final_].label = SymSet::full();
    Ok(())
}

pub fn ltre_ignorecase(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    for st in nfa.states.iter_mut() {
        let orig = st.label;
        for chr in 0..=255u8 {
            if orig.contains(chr) {
                if chr.is_ascii_alphabetic() {
                    st.label.insert(chr.to_ascii_lowercase());
                    st.label.insert(chr.to_ascii_uppercase());
                }
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

// ---- Compilation (NFA -> DFA) ----

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    bs[idx / 8] & (1 << (idx % 8)) != 0
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

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    if bitset_test(bitset, st_id) { return; }
    bitset_set(bitset, st_id);
    if let Some(e0) = nfa.states[st_id].epsilon0 {
        epsilon_closure_into(nfa, e0, bitset);
    }
    if let Some(e1) = nfa.states[st_id].epsilon1 {
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
    let bs_size = (nfa_size + 7) / 8;
    let mut result = vec![0u8; bs_size];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) && nfa.states[id].label.contains(chr) {
            if let Some(target) = nfa.states[id].target {
                epsilon_closure_into(nfa, target, &mut result);
            }
        }
    }
    result
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    let mut dfa = Dfa::new();

    // Create initial DFA state from epsilon closure of NFA initial
    let init_bs = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let accepting = bitset_test(&init_bs, nfa.final_) ^ nfa.complemented;
    let mut init_state = new_dstate(bs_size);
    init_state.bitset = init_bs;
    init_state.accepting = accepting;
    dfa.states.push(init_state);
    dfa.initial = 0;

    // Process states - build DFA by powerset construction
    let mut i = 0;
    while i < dfa.states.len() {
        for chr in 0..256u16 {
            let bs_union = step_powerset(&nfa, &dfa.states[i].bitset, chr as u8);

            // Find existing state with same bitset
            let existing = dfa.states.iter().position(|s| s.bitset == bs_union);
            let target = match existing {
                Some(idx) => idx,
                None => {
                    let acc = bitset_test(&bs_union, nfa.final_) ^ nfa.complemented;
                    let mut ns = new_dstate(bs_size);
                    ns.bitset = bs_union;
                    ns.accepting = acc;
                    dfa.states.push(ns);
                    dfa.states.len() - 1
                }
            };
            dfa.states[i].transitions[chr as usize] = target;
        }
        i += 1;
    }

    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    // Find a "dead" state (non-accepting, all transitions to self)
    for (i, s) in states.iter().enumerate() {
        if !s.accepting && s.transitions.iter().all(|&t| t == i) {
            return i;
        }
    }
    let id = states.len();
    let mut ds = new_dstate(0);
    for t in ds.transitions.iter_mut() { *t = id; }
    states.push(ds);
    id
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let dfa_size = dfa.states.len();
    if dfa_size == 0 { return; }

    let bs_size = (dfa_size + 7) / 8;
    // dis[i][j] = true means states i and j are distinguishable
    let mut dis = vec![vec![0u8; bs_size]; dfa_size];

    // Initially mark pairs with different accepting as distinguishable
    for i in 0..dfa_size {
        for j in (i+1)..dfa_size {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                bitset_set(&mut dis[i], j);
                bitset_set(&mut dis[j], i);
            }
        }
    }

    // Iterate until no changes
    let mut changed = true;
    while changed {
        changed = false;
        for id1 in 0..dfa_size {
            for id2 in (id1+1)..dfa_size {
                if bitset_test(&dis[id1], id2) { continue; }
                let mut found = false;
                for chr in 0..256 {
                    let t1 = dfa.states[id1].transitions[chr];
                    let t2 = dfa.states[id2].transitions[chr];
                    if t1 != t2 && bitset_test(&dis[t1], t2) {
                        found = true;
                        break;
                    }
                }
                if found {
                    bitset_set(&mut dis[id1], id2);
                    bitset_set(&mut dis[id2], id1);
                    changed = true;
                }
            }
        }
    }

    // Merge indistinguishable states
    // Build mapping: for each state, find the representative (lowest index indistinguishable)
    let mut repr = vec![0usize; dfa_size];
    for i in 0..dfa_size {
        repr[i] = i;
    }
    // Process in order similar to C: for each ds1, scan forward and merge
    for ds1 in 0..dfa_size {
        if repr[ds1] != ds1 { continue; } // already merged
        for ds2 in (ds1+1)..dfa_size {
            if repr[ds2] != ds2 { continue; }
            if !bitset_test(&dis[ds1], ds2) {
                repr[ds2] = ds1;
            }
        }
    }

    // Remap transitions
    for i in 0..dfa_size {
        for chr in 0..256 {
            let t = dfa.states[i].transitions[chr];
            dfa.states[i].transitions[chr] = repr[t];
        }
    }

    // Remap initial
    dfa.initial = repr[dfa.initial];

    // Build new state list with only representatives
    let mut new_states = Vec::new();
    let mut old_to_new = vec![0usize; dfa_size];
    for i in 0..dfa_size {
        if repr[i] == i {
            old_to_new[i] = new_states.len();
            new_states.push(dfa.states[i].clone());
        } else {
            old_to_new[i] = old_to_new[repr[i]]; // will be set when repr is processed
        }
    }

    // Fix transitions in new states
    for s in new_states.iter_mut() {
        for chr in 0..256 {
            s.transitions[chr] = old_to_new[s.transitions[chr]];
        }
    }

    dfa.initial = old_to_new[dfa.initial];
    dfa.states = new_states;

    // Flag terminating states
    for i in 0..dfa.states.len() {
        dfa.states[i].terminating = dfa.states[i].transitions.iter().all(|&t| t == i);
    }
}

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut state = dfa.initial;
    for &b in input {
        if dfa.states[state].terminating { break; }
        state = dfa.states[state].transitions[b as usize];
    }
    dfa.states[state].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    if dfap.is_none() {
        let mut dfa = Dfa::new();
        let init_bs = epsilon_closure_vec(nfa, nfa.initial, nfa_size);
        let accepting = bitset_test(&init_bs, nfa.final_) ^ nfa.complemented;
        let mut init_state = new_dstate(bs_size);
        init_state.bitset = init_bs;
        init_state.accepting = accepting;
        dfa.states.push(init_state);
        dfa.initial = 0;
        *dfap = Some(dfa);
    }

    let dfa = dfap.as_mut().unwrap();
    let mut state = dfa.initial;

    for &b in input {
        let chr = b as usize;
        // Check if transition exists (0 could be valid, so we use a sentinel approach)
        // Actually we need to check if we've computed this transition yet
        // Use a flag: if the state's bitset is non-empty but transition target's bitset is empty
        // and target != any existing state... Actually let's just eagerly compute.
        // For lazy: check if transition has been computed
        let target = dfa.states[state].transitions[chr];
        // We need to know if this transition was computed. Let's use a different approach:
        // store computed flags. Actually, let's just compute on demand.
        // The C code checks `if (!dstate->transitions[*input])` (NULL pointer).
        // We'll use usize::MAX as sentinel for "not computed".
        if target == usize::MAX {
            let bs_union = step_powerset(nfa, &dfa.states[state].bitset, b);
            let existing = dfa.states.iter().position(|s| s.bitset == bs_union);
            let new_target = match existing {
                Some(idx) => idx,
                None => {
                    let acc = bitset_test(&bs_union, nfa.final_) ^ nfa.complemented;
                    let mut ns = new_dstate(bs_size);
                    // Initialize all transitions to usize::MAX (not computed)
                    for t in ns.transitions.iter_mut() { *t = usize::MAX; }
                    ns.bitset = bs_union;
                    ns.accepting = acc;
                    dfa.states.push(ns);
                    dfa.states.len() - 1
                }
            };
            dfa.states[state].transitions[chr] = new_target;
            state = new_target;
        } else {
            state = target;
        }
    }

    dfa.states[state].accepting
}

// ---- Uncompile (DFA -> NFA) ----

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();

    let mut nfa = Nfa {
        states: Vec::new(),
        initial: 0,
        final_: 0,
        complemented: false,
    };

    // Allocate initial state
    nfa.states.push(NState::new()); // index 0 = nfa initial

    // Allocate one NFA state per DFA state
    let mut nstates = Vec::new(); // maps DFA state index -> NFA state index
    for _ in 0..dfa_size {
        let id = nfa.states.len();
        nfa.states.push(NState::new());
        nstates.push(id);
    }

    // Allocate final state
    let final_id = nfa.states.len();
    nfa.states.push(NState::new());
    nfa.final_ = final_id;

    // epsilon1 from nfa.initial to nstates[dfa.initial]
    nfa.states[0].epsilon1 = Some(nstates[dfa.initial]);

    // accepting states get epsilon1 to final
    for (i, ds) in dfa.states.iter().enumerate() {
        if ds.accepting {
            nfa.states[nstates[i]].epsilon1 = Some(final_id);
        }
    }

    // For each DFA state, build a binary tree of NFA states for labeled transitions
    let order = dfa_state_order_for_uncompile(dfa);

    for &ds1_idx in &order {
        let mut free: Option<usize> = None;

        for &ds2_idx in &order {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u16 {
                if dfa.states[ds1_idx].transitions[chr as usize] == ds2_idx {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }

            let src;
            if free.is_none() {
                free = Some(nstates[ds1_idx]);
                src = nstates[ds1_idx];
            } else {
                let new_id = nfa.states.len();
                nfa.states.push(NState::new());
                src = new_id;

                let f = free.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_id);
                } else {
                    nfa.states[f].epsilon0 = Some(new_id);
                    free = Some(new_id);
                }
            }

            nfa.states[src].target = Some(nstates[ds2_idx]);
            nfa.states[src].label = transitions;
        }
    }

    nfa
}

fn dfa_state_order_for_uncompile(dfa: &Dfa) -> Vec<usize> {
    let mut order = vec![dfa.initial];
    for i in 0..dfa.states.len() {
        if i != dfa.initial {
            order.push(i);
        }
    }
    order
}

// ---- Decompile (DFA -> regex string) ----

pub fn ltre_decompile(dfa: &Dfa) -> String {
    #[derive(Clone)]
    struct Arrow {
        label: Option<String>, // None = empty /[]/, Some("") = epsilon /()/ 
        prec: u8,
    }
    const ALT: u8 = 0;
    const CONCAT: u8 = 1;
    const QUANT: u8 = 2;
    const SYMSET: u8 = 3;

    let dfa_size = dfa.states.len();
    let aux = dfa_size; // auxiliary state index
    let n = dfa_size + 1;

    // arrows[i][j]
    let mut arrows: Vec<Vec<Arrow>> = vec![vec![Arrow { label: None, prec: 0 }; n]; n];

    // epsilon from aux to initial
    arrows[aux][dfa.initial].label = Some(String::new());
    arrows[aux][dfa.initial].prec = SYMSET;

    for (id1, ds1) in dfa.states.iter().enumerate() {
        // accepting states get epsilon to aux
        if ds1.accepting {
            arrows[id1][aux].label = Some(String::new());
            arrows[id1][aux].prec = SYMSET;
        }

        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u16 {
                if ds1.transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            let fmt = symset_fmt(&transitions);
            arrows[id1][id2].label = Some(fmt.clone());
            arrows[id1][id2].prec = SYMSET;
        }
    }

    // State elimination
    loop {
        let mut best_fit = None;
        let mut min_degree = i32::MAX;

        for id1 in 0..dfa_size {
            let mut degree = 0i32;
            for id2 in 0..dfa_size {
                if arrows[id1][id2].label.is_some() { degree += 1; }
                if arrows[id2][id1].label.is_some() { degree += 1; }
            }
            if degree == 0 { continue; }
            if degree < min_degree {
                min_degree = degree;
                best_fit = Some(id1);
            }
        }

        let best_fit = match best_fit {
            Some(bf) => bf,
            None => break,
        };

        for id1 in 0..n {
            if id1 == best_fit { continue; }
            for id2 in 0..n {
                if id2 == best_fit { continue; }

                let in_arrow = arrows[id1][best_fit].clone();
                let out_arrow = arrows[best_fit][id2].clone();
                let self_arrow = arrows[best_fit][best_fit].clone();
                let existing = arrows[id1][id2].clone();

                if in_arrow.label.is_none() || out_arrow.label.is_none() { continue; }

                let in_label = in_arrow.label.as_ref().unwrap();
                let out_label = out_arrow.label.as_ref().unwrap();
                let self_label = self_arrow.label.as_ref();

                // Compute first and second after handling self-loop
                let (mut first_label, mut first_prec, mut second_label, mut second_prec) =
                    (in_label.clone(), in_arrow.prec, out_label.clone(), out_arrow.prec);

                if self_label.is_some() && !self_label.unwrap().is_empty() {
                    let sl = self_label.unwrap();
                    let sp = self_arrow.prec;

                    let mut handled = false;

                    'try_in_suffix: {
                        if in_arrow.prec >= CONCAT && sp >= CONCAT {
                            let diff = in_label.len() as isize - sl.len() as isize;
                            if diff >= 0 {
                                let diff = diff as usize;
                                if &in_label[diff..] == sl {
                                    let mut nevermind = false;
                                    if diff >= 1 {
                                        let prev = in_label.as_bytes()[diff - 1];
                                        if b"^-\\".contains(&prev) && (diff == 1 || in_label.as_bytes()[diff - 2] != b'\\') {
                                            nevermind = true;
                                        }
                                    }
                                    if !nevermind && diff >= 2 && &in_label[diff-2..diff] == "\\x" && (diff == 2 || in_label.as_bytes()[diff - 3] != b'\\') {
                                        nevermind = true;
                                    }
                                    if !nevermind && diff >= 3 && &in_label[diff-3..diff-1] == "\\x" && (diff == 3 || in_label.as_bytes()[diff - 4] != b'\\') {
                                        nevermind = true;
                                    }

                                    if !nevermind {
                                        let mut p = String::new();
                                        if diff != 0 && in_arrow.prec < CONCAT { p.push('('); }
                                        p.push_str(&in_label[..diff]);
                                        if diff != 0 && in_arrow.prec < CONCAT { p.push(')'); }
                                        if sp <= QUANT { p.push('('); }
                                        p.push_str(sl);
                                        if sp <= QUANT { p.push(')'); }
                                        p.push('+');

                                        first_label = p;
                                        first_prec = CONCAT;
                                        second_label = out_label.clone();
                                        second_prec = out_arrow.prec;
                                        handled = true;
                                    }
                                }
                            }
                        }
                    }

                    if !handled && out_arrow.prec >= CONCAT && sp >= CONCAT {
                        let diff = out_label.len() as isize - sl.len() as isize;
                        if diff >= 0 && out_label.starts_with(sl) {
                            let diff = diff as usize;
                            let mut p = String::new();
                            if sp <= QUANT { p.push('('); }
                            p.push_str(sl);
                            if sp <= QUANT { p.push(')'); }
                            p.push('+');
                            if diff != 0 && out_arrow.prec < CONCAT { p.push('('); }
                            p.push_str(&out_label[sl.len()..]);
                            if diff != 0 && out_arrow.prec < CONCAT { p.push(')'); }

                            first_label = in_label.clone();
                            first_prec = in_arrow.prec;
                            second_label = p;
                            second_prec = CONCAT;
                            handled = true;
                        }
                    }

                    if !handled {
                        let mut p = String::new();
                        if sp <= QUANT { p.push('('); }
                        p.push_str(sl);
                        if sp <= QUANT { p.push(')'); }
                        p.push('*');
                        if out_arrow.prec < CONCAT { p.push('('); }
                        p.push_str(out_label);
                        if out_arrow.prec < CONCAT { p.push(')'); }

                        first_label = in_label.clone();
                        first_prec = in_arrow.prec;
                        second_label = p;
                        second_prec = CONCAT;
                    }
                }

                // Concatenate first and second to create bypass
                let (bypass_label, bypass_prec);
                if first_label.is_empty() {
                    bypass_label = second_label.clone();
                    bypass_prec = second_prec;
                } else if second_label.is_empty() {
                    bypass_label = first_label.clone();
                    bypass_prec = first_prec;
                } else {
                    let mut p = String::new();
                    if first_prec < CONCAT { p.push('('); }
                    p.push_str(&first_label);
                    if first_prec < CONCAT { p.push(')'); }
                    if second_prec < CONCAT { p.push('('); }
                    p.push_str(&second_label);
                    if second_prec < CONCAT { p.push(')'); }
                    bypass_label = p;
                    bypass_prec = CONCAT;
                }

                // Merge bypass with existing
                let (merged_label, merged_prec);
                let existing_label = existing.label.as_ref();

                if existing_label.is_none() {
                    merged_label = Some(bypass_label);
                    merged_prec = bypass_prec;
                } else if existing_label.unwrap().is_empty() {
                    // ()|(bypass) = (bypass)?
                    let mut p = String::new();
                    if bypass_prec <= QUANT { p.push('('); }
                    p.push_str(&bypass_label);
                    if bypass_prec <= QUANT { p.push(')'); }
                    p.push('?');
                    merged_label = Some(p);
                    merged_prec = QUANT;
                } else {
                    // (existing)|(bypass)
                    let mut p = String::new();
                    p.push_str(existing_label.unwrap());
                    p.push('|');
                    p.push_str(&bypass_label);
                    merged_label = Some(p);
                    merged_prec = ALT;
                }

                arrows[id1][id2] = Arrow { label: merged_label, prec: merged_prec };
            }
        }

        // Eliminate best_fit
        for id in 0..n {
            arrows[id][best_fit] = Arrow { label: None, prec: 0 };
            arrows[best_fit][id] = Arrow { label: None, prec: 0 };
        }
    }

    match &arrows[aux][aux].label {
        Some(s) => s.clone(),
        None => "[]".to_string(),
    }
}
