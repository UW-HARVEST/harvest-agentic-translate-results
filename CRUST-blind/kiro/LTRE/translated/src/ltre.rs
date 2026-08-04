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
        self.bits[c as usize / 8] & (1 << (c as usize % 8)) != 0
    }
    pub fn insert(&mut self, c: u8) {
        self.bits[c as usize / 8] |= 1 << (c as usize % 8);
    }
    pub fn invert(&mut self) {
        for b in self.bits.iter_mut() { *b = !*b; }
    }
    pub fn union_with(&mut self, other: &SymSet) {
        for i in 0..32 { self.bits[i] |= other.bits[i]; }
    }
    pub fn intersect_with(&mut self, other: &SymSet) {
        for i in 0..32 { self.bits[i] &= other.bits[i]; }
    }
    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }
}

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
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
        let mut first = true;
        loop {
            if first { first = false; } else { /* came from goto-like */ }
            let c = chr as u8;
            let in_set = set.contains(c);
            if in_set { nsym += 1; } else { nnsym += 1; }
            let p = if in_set { &mut buf } else { &mut nbuf };
            let mc = is_metachar(c);
            if !is_printable(c) && !mc {
                p.push_str(&format!("\\x{:02x}", c));
            } else {
                if mc { p.push('\\'); }
                p.push(c as char);
            }
            let start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - start >= 2 {
                let p2 = if in_set { &mut buf } else { &mut nbuf };
                p2.push('-');
                if in_set { nsym -= 1; } else { nnsym -= 1; }
            }
            if chr - start >= 1 {
                // "goto append_chr" equivalent: continue inner loop
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
        // remove first char of nbuf content after '^', replace with '^'
        let inner = &nbuf[2..nbuf.len()-1]; // skip ^[ and ]
        return format!("^{}", inner);
    }

    if buf.len() < nbuf.len() { buf } else { nbuf }
}

fn is_printable(c: u8) -> bool {
    c >= 0x20 && c <= 0x7e
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
        NState { label: SymSet::empty(), target: None, epsilon0: None, epsilon1: None }
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
        Nfa { states: vec![NState::new()], initial: 0, final_: 0, complemented: false }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
}

fn alloc_state(nfa: &mut Nfa) -> usize {
    let id = nfa.states.len();
    nfa.states.push(NState::new());
    id
}

pub fn nfa_free(_nfa: Nfa) {}
pub fn dfa_free(_dfa: Dfa) {}

pub fn nfa_clone(orig: &Nfa) -> Nfa {
    orig.clone()
}

pub fn nfa_concat(nfa1: &mut Nfa, mut nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ {
        // nfa1 is a single-state NFA, replace it
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        // nfa2 is a single-state NFA, nothing to concat
        return;
    }
    // Merge nfa2.initial into nfa1.final_
    let offset = nfa1.states.len();
    // Copy nfa2 initial state's data into nfa1.final_
    let ini2 = nfa2.initial;
    let fin1 = nfa1.final_;
    nfa1.states[fin1].label = nfa2.states[ini2].label;
    nfa1.states[fin1].target = nfa2.states[ini2].target.map(|t| remap(t, ini2, fin1, offset));
    nfa1.states[fin1].epsilon0 = nfa2.states[ini2].epsilon0.map(|t| remap(t, ini2, fin1, offset));
    nfa1.states[fin1].epsilon1 = nfa2.states[ini2].epsilon1.map(|t| remap(t, ini2, fin1, offset));
    // Add all nfa2 states except initial
    for (i, mut st) in nfa2.states.into_iter().enumerate() {
        if i == ini2 { continue; }
        st.target = st.target.map(|t| remap(t, ini2, fin1, offset));
        st.epsilon0 = st.epsilon0.map(|t| remap(t, ini2, fin1, offset));
        st.epsilon1 = st.epsilon1.map(|t| remap(t, ini2, fin1, offset));
        nfa1.states.push(st);
    }
    nfa1.final_ = remap(nfa2.final_, ini2, fin1, offset);
}

fn remap(id: usize, old_ini: usize, new_ini: usize, offset: usize) -> usize {
    if id == old_ini {
        new_ini
    } else if id < old_ini {
        id + offset
    } else {
        id + offset - 1
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
    if !nfa.complemented { return Ok(()); }
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
        if let Some(e0) = st.epsilon0 { println!("  {} --> {}", id, e0); }
        if let Some(e1) = st.epsilon1 { println!("  {} --> {}", id, e1); }
        if st.label.is_empty() { continue; }
        print!("  {} --", id);
        let fmt = symset_fmt(&st.label);
        for c in fmt.chars() {
            if "\\\"#&{}()xo=- ".contains(c) {
                print!("#{};", c as u32);
            } else {
                print!("{}", c);
            }
        }
        if let Some(t) = st.target {
            println!("--> {}", t);
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
        transitions: [usize::MAX; 256],
        accepting: false,
        terminating: false,
        bitset: vec![0u8; bitset_size],
    }
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf = Vec::new();
    let dfa_size = dfa.states.len() as i32;
    leb128_put(&mut buf, dfa_size);
    for st in &dfa.states {
        buf.push((st.accepting as u8) << 1 | st.terminating as u8);
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            while chr < 255 && st.transitions[chr] == st.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8);
            leb128_put(&mut buf, st.transitions[chr] as i32);
            chr += 1;
        }
    }
    buf
}

pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p = 0usize;
    let dfa_size = leb128_get(buf, &mut p)? as usize;
    let mut dfa = Dfa { states: Vec::with_capacity(dfa_size), initial: 0 };
    for _ in 0..dfa_size {
        let mut ds = new_dstate(0);
        if p >= buf.len() { return Err("unexpected end of buffer".into()); }
        let flags = buf[p]; p += 1;
        ds.accepting = (flags >> 1) & 1 != 0;
        ds.terminating = flags & 1 != 0;
        let mut chr = 0usize;
        while chr < 256 {
            if p >= buf.len() { return Err("unexpected end of buffer".into()); }
            let len = buf[p] as usize; p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            for _ in 0..=len {
                if chr >= 256 { return Err("bad run length".into()); }
                ds.transitions[chr] = target;
                chr += 1;
            }
        }
        dfa.states.push(ds);
    }
    Ok((dfa, p))
}

pub fn dfa_dump(dfa: &Dfa) {
    println!("graph LR");
    println!("  I( ) --> {}", dfa.initial);
    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting { println!("  {} --> F( )", id1); }
        for (id2, _) in dfa.states.iter().enumerate() {
            let mut ss = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if ds1.transitions[chr] == id2 {
                    ss.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            print!("  {} --", id1);
            let fmt = symset_fmt(&ss);
            for c in fmt.chars() {
                if "\\\"#&{}()xo=- ".contains(c) {
                    print!("#{};", c as u32);
                } else {
                    print!("{}", c);
                }
            }
            println!("--> {}", id2);
        }
    }
}

fn leb128_put(buf: &mut Vec<u8>, mut n: i32) {
    loop {
        if (n >> 7) != 0 {
            buf.push((n as u8 & 0x7f) | 0x80);
            n >>= 7;
        } else {
            buf.push(n as u8);
            break;
        }
    }
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: i32 = 0;
    let mut c = 0;
    loop {
        if *p >= buf.len() { return Err("unexpected end of leb128".into()); }
        let byte = buf[*p];
        n |= ((byte & 0x7f) as i32) << (c * 7);
        c += 1;
        *p += 1;
        if byte & 0x80 == 0 { break; }
    }
    Ok(n)
}

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
        } else { None }
    }
    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
    fn expect_char(&mut self) -> Result<u8, String> {
        self.next().ok_or_else(|| "unexpected end of input".to_string())
    }
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte = 0u8;
    for _ in 0..2 {
        byte <<= 4;
        let c = ctx.peek().ok_or("expected hex digit")?;
        if c.is_ascii_digit() {
            byte |= c - b'0';
        } else if c.is_ascii_hexdigit() {
            byte |= (c as char).to_ascii_lowercase() as u8 - b'a' + 10;
        } else {
            return Err("expected hex digit".into());
        }
        ctx.next();
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or("expected escape")?;
    if is_metachar(c) { ctx.next(); return Ok(c); }
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
        _ => { ctx.pos -= 1; Err("unknown escape".into()) }
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or("expected symbol")?;
    if c == b'\\' {
        ctx.next();
        return parse_escape(ctx);
    }
    if METACHARS.contains(&c) { return Err("unexpected metacharacter".into()); }
    if !is_printable(c) { return Err("unexpected nonprintable character".into()); }
    ctx.next();
    Ok(c)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    if ctx.peek() == Some(b'\\') {
        let saved = ctx.pos;
        ctx.next();
        let c = ctx.peek();
        match c {
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
        let mut ss = SymSet::full();
        // . matches everything except \n
        ss.bits[b'\n' as usize / 8] &= !(1 << (b'\n' as usize % 8));
        return Ok(ss);
    }
    Err("expected shorthand class".into())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.next();
        complement = true;
    }

    let saved = ctx.pos;
    match parse_shorthand(ctx) {
        Ok(mut ss) => {
            if complement { ss.invert(); }
            return Ok(ss);
        }
        Err(_) => { ctx.pos = saved; }
    }

    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut ss = SymSet::empty();
        while ctx.peek() != Some(b']') {
            if ctx.is_eof() { return Err("expected ']'".into()); }
            let sub = parse_symset(ctx)?;
            ss.union_with(&sub);
        }
        ctx.next(); // consume ']'
        if complement { ss.invert(); }
        return Ok(ss);
    }

    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut ss = SymSet::full();
        while ctx.peek() != Some(b'>') {
            if ctx.is_eof() { return Err("expected '>'".into()); }
            let sub = parse_symset(ctx)?;
            ss.intersect_with(&sub);
        }
        ctx.next(); // consume '>'
        if complement { ss.invert(); }
        return Ok(ss);
    }

    ctx.pos = saved;
    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.next();
        end = parse_symbol(ctx)?;
    }
    let end_open = end.wrapping_add(1);
    let mut ss = SymSet::empty();
    let mut c = begin;
    loop {
        ss.insert(c);
        c = c.wrapping_add(1);
        if c == end_open { break; }
    }
    if complement { ss.invert(); }
    Ok(ss)
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

    let mut nfa = Nfa { states: vec![NState::new(), NState::new()], initial: 0, final_: 1, complemented: false };
    let ss = parse_symset(ctx)?;
    nfa.states[0].label = ss;
    nfa.states[0].target = Some(1);
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    if ctx.peek() == Some(b'*') {
        ctx.next();
        nfa_uncomplement(&mut atom)?;
        atom.states[atom.final_].epsilon1 = Some(atom.initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }
    if ctx.peek() == Some(b'+') {
        ctx.next();
        nfa_uncomplement(&mut atom)?;
        atom.states[atom.final_].epsilon1 = Some(atom.initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        return Ok(atom);
    }
    if ctx.peek() == Some(b'?') {
        ctx.next();
        nfa_uncomplement(&mut atom)?;
        if atom.states[atom.initial].epsilon1.is_some() {
            nfa_pad_initial(&mut atom);
        }
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }

    let saved = ctx.pos;
    if ctx.peek() == Some(b'{') {
        ctx.next();
        nfa_uncomplement(&mut atom)?;

        let min_res = parse_natural(ctx);
        let min = match min_res {
            Ok(v) => v,
            Err(ref e) if e.contains("overflow") => { return Err(e.clone()); }
            Err(_) => 0,
        };

        let mut max = min;
        let mut max_unbounded = false;
        if ctx.peek() == Some(b',') {
            ctx.next();
            match parse_natural(ctx) {
                Ok(v) => max = v,
                Err(ref e) if e.contains("overflow") => { return Err(e.clone()); }
                Err(_) => { max_unbounded = true; }
            }
        }

        if ctx.peek() != Some(b'}') {
            return Err("expected '}'".into());
        }
        ctx.next();

        if min > max && !max_unbounded {
            ctx.pos = saved;
            return Err("misbounded quantifier".into());
        }

        let mut atoms = Nfa::new_single();
        atoms.complemented = false;

        let mut i: u32 = 0;
        loop {
            let should_continue = if max_unbounded { i <= min } else { i < max };
            if !should_continue { break; }
            let mut clone = nfa_clone(&atom);
            if i >= min {
                if max_unbounded {
                    nfa_uncomplement(&mut clone)?;
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

    while !matches!(ctx.peek(), Some(b')') | Some(b'|') | Some(b'&') | None) {
        let mut factor = parse_factor(ctx)?;
        nfa_uncomplement(&mut factor)?;
        nfa_concat(&mut term, factor);
    }

    if complement { term.complemented = true; }
    Ok(term)
}

fn parse_regex(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut re = parse_term(ctx)?;

    while ctx.peek() == Some(b'|') || ctx.peek() == Some(b'&') {
        let intersect = ctx.peek() == Some(b'&');
        ctx.next();
        let mut alt = parse_term(ctx)?;

        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Connect re.initial epsilon1 -> alt.initial
        let alt_offset = re.states.len();
        re.states[re.initial].epsilon1 = Some(alt.initial + alt_offset);
        re.states[re.final_].epsilon0 = Some(alt.final_ + alt_offset);

        // Merge alt states into re
        let old_re_final = re.final_;
        for st in alt.states.iter() {
            let mut ns = st.clone();
            ns.target = ns.target.map(|t| t + alt_offset);
            ns.epsilon0 = ns.epsilon0.map(|t| t + alt_offset);
            ns.epsilon1 = ns.epsilon1.map(|t| t + alt_offset);
            re.states.push(ns);
        }
        // re.final_ -> alt.final (which is now at alt.final_ + alt_offset)
        // But we already set epsilon0 above. Now set re.final_ to alt.final_
        re.final_ = alt.final_ + alt_offset;

        // Fix: re's old final's next should link to alt's initial
        // In the C code: re.final->next = alt.initial; re.final = alt.final
        // We already handled the epsilon0 connection from old_re_final to alt.final_ + alt_offset
        let _ = old_re_final;

        re.complemented ^= intersect;
    }

    Ok(re)
}

pub fn ltre_parse(regex: &str) -> Result<Nfa, String> {
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
        let old = st.label;
        for chr in 0..=255u8 {
            if old.contains(chr) {
                st.label.insert((chr as char).to_ascii_lowercase() as u8);
                st.label.insert((chr as char).to_ascii_uppercase() as u8);
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

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
    let bs_size = (nfa_size + 7) / 8;
    let mut bs = vec![0u8; bs_size];
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
    let mut init_ds = new_dstate(bs_size);
    init_ds.bitset = init_bs.clone();
    init_ds.accepting = bitset_test(&init_bs, nfa.final_) ^ nfa.complemented;
    dfa.states.push(init_ds);
    dfa.initial = 0;

    let mut i = 0;
    while i < dfa.states.len() {
        for chr in 0..=255u8 {
            let bs = step_powerset(&nfa, &dfa.states[i].bitset, chr);
            // Find existing state with same bitset
            let existing = dfa.states.iter().position(|s| s.bitset == bs);
            let target = if let Some(idx) = existing {
                idx
            } else {
                let mut ds = new_dstate(bs_size);
                ds.bitset = bs.clone();
                ds.accepting = bitset_test(&bs, nfa.final_) ^ nfa.complemented;
                let idx = dfa.states.len();
                dfa.states.push(ds);
                idx
            };
            dfa.states[i].transitions[chr as usize] = target;
        }
        i += 1;
    }

    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
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

    // Distinguishability matrix
    let bs_size = (dfa_size + 7) / 8;
    let mut dis = vec![vec![0u8; bs_size]; dfa_size];

    // Initially distinguish states with different accepting values
    for i in 0..dfa_size {
        for j in (i+1)..dfa_size {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                bitset_set(&mut dis[i], j);
                bitset_set(&mut dis[j], i);
            }
        }
    }

    // Iterate until no more changes
    let mut changed = true;
    while changed {
        changed = false;
        for id1 in 0..dfa_size {
            for id2 in (id1+1)..dfa_size {
                if !bitset_test(&dis[id1], id2) {
                    for chr in 0..256usize {
                        let t1 = dfa.states[id1].transitions[chr];
                        let t2 = dfa.states[id2].transitions[chr];
                        if t1 != t2 && bitset_test(&dis[t1], t2) {
                            bitset_set(&mut dis[id1], id2);
                            bitset_set(&mut dis[id2], id1);
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // Merge indistinguishable states
    // Build mapping: old_id -> canonical_id
    let mut mapping: Vec<usize> = (0..dfa_size).collect();
    for id1 in 0..dfa_size {
        for id2 in (id1+1)..dfa_size {
            if !bitset_test(&dis[id1], id2) && mapping[id2] == id2 {
                mapping[id2] = mapping[id1];
            }
        }
    }

    // Remap transitions
    for st in dfa.states.iter_mut() {
        for t in st.transitions.iter_mut() {
            *t = mapping[*t];
        }
    }

    // Determine which states to keep
    let mut kept: Vec<bool> = vec![false; dfa_size];
    for i in 0..dfa_size {
        if mapping[i] == i { kept[i] = true; }
    }

    // Build new state list and remap
    let mut new_id: Vec<usize> = vec![0; dfa_size];
    let mut new_states: Vec<DState> = Vec::new();
    for i in 0..dfa_size {
        if kept[i] {
            new_id[i] = new_states.len();
            new_states.push(dfa.states[i].clone());
        }
    }
    for i in 0..dfa_size {
        if !kept[i] {
            new_id[i] = new_id[mapping[i]];
        }
    }

    // Remap transitions in new states
    for st in new_states.iter_mut() {
        for t in st.transitions.iter_mut() {
            *t = new_id[*t];
        }
    }

    // Flag terminating states
    for i in 0..new_states.len() {
        new_states[i].terminating = (0..256).all(|chr| new_states[i].transitions[chr] == i);
    }

    dfa.initial = new_id[dfa.initial];
    dfa.states = new_states;
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
    if dfap.is_none() {
        *dfap = Some(Dfa::new());
    }
    let dfa = dfap.as_mut().unwrap();
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    // Ensure initial state exists
    if dfa.states.is_empty() {
        let init_bs = epsilon_closure_vec(nfa, nfa.initial, nfa_size);
        let mut ds = new_dstate(bs_size);
        ds.bitset = init_bs.clone();
        ds.accepting = bitset_test(&init_bs, nfa.final_) ^ nfa.complemented;
        dfa.states.push(ds);
        dfa.initial = 0;
    }

    let mut state = dfa.initial;
    for &b in input {
        let chr = b as usize;
        // Check if transition exists (0 could be valid, so we use a sentinel approach)
        // Actually, we need to lazily create. Use a flag or check bitset.
        // In the C code, transitions are NULL initially. We'll use usize::MAX as sentinel.
        if dfa.states[state].transitions[chr] == usize::MAX {
            // Create new state
            let bs = step_powerset(nfa, &dfa.states[state].bitset, b);
            let existing = dfa.states.iter().position(|s| s.bitset == bs);
            let target = if let Some(idx) = existing {
                idx
            } else {
                let mut ds = new_dstate(bs_size);
                ds.bitset = bs.clone();
                ds.accepting = bitset_test(&bs, nfa.final_) ^ nfa.complemented;
                let idx = dfa.states.len();
                dfa.states.push(ds);
                idx
            };
            dfa.states[state].transitions[chr] = target;
        }
        state = dfa.states[state].transitions[chr];
    }
    dfa.states[state].accepting
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    // Create NFA: initial state + one state per DFA state + final state
    let mut nfa_states: Vec<NState> = Vec::new();

    // State 0: NFA initial
    nfa_states.push(NState::new());
    // States 1..=dfa_size: one per DFA state
    for _ in 0..dfa_size {
        nfa_states.push(NState::new());
    }
    // State dfa_size+1: NFA final
    nfa_states.push(NState::new());

    let nfa_initial = 0;
    let nfa_final = dfa_size + 1;

    // initial epsilon1 -> DFA initial state's NFA state
    nfa_states[nfa_initial].epsilon1 = Some(dfa.initial + 1);

    // accepting DFA states get epsilon1 -> nfa_final
    for (id, ds) in dfa.states.iter().enumerate() {
        if ds.accepting {
            nfa_states[id + 1].epsilon1 = Some(nfa_final);
        }
    }

    // For each DFA state, build a binary tree of labeled transitions
    for (id1, ds1) in dfa.states.iter().enumerate() {
        let mut free_node: Option<usize> = None;

        for id2 in 0..dfa_size {
            let mut ss = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if ds1.transitions[chr] == id2 {
                    ss.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }

            let src;
            if free_node.is_none() {
                // First iteration: use the nstates[id1] directly
                src = id1 + 1;
                free_node = Some(src);
            } else {
                let new_id = nfa_states.len();
                nfa_states.push(NState::new());
                src = new_id;

                let f = free_node.unwrap();
                if nfa_states[f].epsilon1.is_none() || (f == id1 + 1 && dfa.states[id1].accepting && nfa_states[f].epsilon1 == Some(nfa_final)) {
                    // If epsilon1 is used for accepting, we need to handle differently
                    if nfa_states[f].epsilon1.is_none() {
                        nfa_states[f].epsilon1 = Some(new_id);
                    } else {
                        // epsilon1 already used (for accepting), use epsilon0
                        nfa_states[f].epsilon0 = Some(new_id);
                        free_node = Some(new_id);
                    }
                } else if nfa_states[f].epsilon1.is_some() {
                    nfa_states[f].epsilon0 = Some(new_id);
                    free_node = Some(new_id);
                } else {
                    nfa_states[f].epsilon1 = Some(new_id);
                }
            }

            nfa_states[src].target = Some(id2 + 1);
            nfa_states[src].label = ss;
        }
    }

    Nfa {
        states: nfa_states,
        initial: nfa_initial,
        final_: nfa_final,
        complemented: false,
    }
}

#[derive(Clone)]
struct DecompArrow {
    label: Option<String>,
    prec: u8,
}
const DC_ALT: u8 = 0;
const DC_CONCAT: u8 = 1;
const DC_QUANT: u8 = 2;
const DC_SYMSET: u8 = 3;

fn compute_first_second(
    in_arrow: &DecompArrow, out_arrow: &DecompArrow, self_arrow: &DecompArrow,
) -> (String, u8, String, u8) {
    let in_label = in_arrow.label.as_ref().unwrap();
    let out_label = out_arrow.label.as_ref().unwrap();

    if self_arrow.label.is_none() || self_arrow.label.as_ref().unwrap().is_empty() {
        return (in_label.clone(), in_arrow.prec, out_label.clone(), out_arrow.prec);
    }

    let self_label = self_arrow.label.as_ref().unwrap();
    let self_prec = self_arrow.prec;

    // Try (in_pre)(self)+(out)
    if in_arrow.prec >= DC_CONCAT && self_prec >= DC_CONCAT {
        let diff = in_label.len() as isize - self_label.len() as isize;
        if diff >= 0 && &in_label[diff as usize..] == self_label {
            let d = diff as usize;
            let mut nevermind = false;
            if d >= 1 {
                let prev = in_label.as_bytes()[d - 1];
                if b"^-\\".contains(&prev) && (d == 1 || in_label.as_bytes()[d - 2] != b'\\') {
                    nevermind = true;
                }
            }
            if !nevermind && d >= 2 && &in_label[d-2..d] == "\\x"
                && (d == 2 || in_label.as_bytes()[d - 3] != b'\\') {
                nevermind = true;
            }
            if !nevermind && d >= 3 && &in_label[d-3..d-1] == "\\x"
                && (d == 3 || in_label.as_bytes()[d - 4] != b'\\') {
                nevermind = true;
            }
            if !nevermind {
                let mut p = String::new();
                if d != 0 && in_arrow.prec < DC_CONCAT { p.push('('); }
                p.push_str(&in_label[..d]);
                if d != 0 && in_arrow.prec < DC_CONCAT { p.push(')'); }
                if self_prec <= DC_QUANT { p.push('('); }
                p.push_str(self_label);
                if self_prec <= DC_QUANT { p.push(')'); }
                p.push('+');
                return (p, DC_CONCAT, out_label.clone(), out_arrow.prec);
            }
        }
    }

    // Try (in)(self)+(out_post)
    if out_arrow.prec >= DC_CONCAT && self_prec >= DC_CONCAT {
        let diff = out_label.len() as isize - self_label.len() as isize;
        if diff >= 0 && &out_label[..self_label.len()] == self_label {
            let d = diff as usize;
            let mut p = String::new();
            if self_prec <= DC_QUANT { p.push('('); }
            p.push_str(self_label);
            if self_prec <= DC_QUANT { p.push(')'); }
            p.push('+');
            if d != 0 && out_arrow.prec < DC_CONCAT { p.push('('); }
            p.push_str(&out_label[self_label.len()..]);
            if d != 0 && out_arrow.prec < DC_CONCAT { p.push(')'); }
            return (in_label.clone(), in_arrow.prec, p, DC_CONCAT);
        }
    }

    // (in)(self)*(out)
    let mut p = String::new();
    if self_prec <= DC_QUANT { p.push('('); }
    p.push_str(self_label);
    if self_prec <= DC_QUANT { p.push(')'); }
    p.push('*');
    if out_arrow.prec < DC_CONCAT { p.push('('); }
    p.push_str(out_label);
    if out_arrow.prec < DC_CONCAT { p.push(')'); }
    (in_label.clone(), in_arrow.prec, p, DC_CONCAT)
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();

    let n = dfa_size + 1; // +1 for auxiliary state
    let aux = dfa_size;
    let mut arrows: Vec<Vec<DecompArrow>> = vec![vec![DecompArrow { label: None, prec: 0 }; n]; n];

    // Epsilon from aux to initial
    arrows[aux][dfa.initial].label = Some(String::new());
    arrows[aux][dfa.initial].prec = DC_SYMSET;

    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            arrows[id1][aux].label = Some(String::new());
            arrows[id1][aux].prec = DC_SYMSET;
        }
        for id2 in 0..dfa_size {
            let mut ss = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if ds1.transitions[chr] == id2 {
                    ss.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            let fmt = symset_fmt(&ss);
            arrows[id1][id2].label = Some(fmt);
            arrows[id1][id2].prec = DC_SYMSET;
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
            // Also count aux connections
            if arrows[id1][aux].label.is_some() { degree += 1; }
            if arrows[aux][id1].label.is_some() { degree += 1; }
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

        for id1 in 0..=aux {
            if id1 == best_fit { continue; }
            for id2 in 0..=aux {
                if id2 == best_fit { continue; }
                let in_arrow = arrows[id1][best_fit].clone();
                let out_arrow = arrows[best_fit][id2].clone();
                let self_arrow = arrows[best_fit][best_fit].clone();
                let existing = arrows[id1][id2].clone();

                if in_arrow.label.is_none() || out_arrow.label.is_none() { continue; }

                let in_label = in_arrow.label.as_ref().unwrap();
                let out_label = out_arrow.label.as_ref().unwrap();

                let (first_label, first_prec, second_label, second_prec) =
                    compute_first_second(&in_arrow, &out_arrow, &self_arrow);

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
                    if first_prec < DC_CONCAT { p.push('('); }
                    p.push_str(&first_label);
                    if first_prec < DC_CONCAT { p.push(')'); }
                    if second_prec < DC_CONCAT { p.push('('); }
                    p.push_str(&second_label);
                    if second_prec < DC_CONCAT { p.push(')'); }
                    bypass_label = p;
                    bypass_prec = DC_CONCAT;
                }

                // Merge with existing
                let (merged_label, merged_prec): (Option<String>, u8);
                if existing.label.is_none() {
                    merged_label = Some(bypass_label);
                    merged_prec = bypass_prec;
                } else {
                    let ex_label = existing.label.as_ref().unwrap();
                    if ex_label.is_empty() {
                        // ()|(bypass) == (bypass)?
                        let mut p = String::new();
                        if bypass_prec <= DC_QUANT { p.push('('); }
                        p.push_str(&bypass_label);
                        if bypass_prec <= DC_QUANT { p.push(')'); }
                        p.push('?');
                        merged_label = Some(p);
                        merged_prec = DC_QUANT;
                    } else {
                        // (existing)|(bypass)
                        let mut p = String::new();
                        p.push_str(ex_label);
                        p.push('|');
                        p.push_str(&bypass_label);
                        merged_label = Some(p);
                        merged_prec = DC_ALT;
                    }
                }

                arrows[id1][id2] = DecompArrow { label: merged_label, prec: merged_prec };
            }
        }

        // Eliminate best_fit
        for id in 0..=aux {
            arrows[id][best_fit].label = None;
            arrows[best_fit][id].label = None;
        }
    }

    // Result is self-loop on auxiliary state
    match &arrows[aux][aux].label {
        Some(s) => s.clone(),
        None => "[]".to_string(),
    }
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}

fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

fn digits_set() -> SymSet {
    let mut ss = SymSet::empty();
    for c in b'0'..=b'9' { ss.insert(c); }
    ss
}

fn not_digits_set() -> SymSet {
    let mut ss = digits_set();
    ss.invert();
    ss
}

fn spaces_set() -> SymSet {
    let mut ss = SymSet::empty();
    for &c in &[b' ', b'\t', b'\n', b'\r', 0x0bu8, 0x0cu8] { ss.insert(c); }
    ss
}

fn not_spaces_set() -> SymSet {
    let mut ss = spaces_set();
    ss.invert();
    ss
}

fn wordchar_set() -> SymSet {
    let mut ss = SymSet::empty();
    ss.insert(b'_');
    for c in b'a'..=b'z' { ss.insert(c); }
    for c in b'A'..=b'Z' { ss.insert(c); }
    for c in b'0'..=b'9' { ss.insert(c); }
    ss
}

fn not_wordchar_set() -> SymSet {
    let mut ss = wordchar_set();
    ss.invert();
    ss
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    if ctx.peek().map_or(true, |c| !c.is_ascii_digit()) {
        return Err("expected natural number".into());
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !c.is_ascii_digit() { break; }
        let digit = (c - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            // consume remaining digits
            while ctx.peek().map_or(false, |c| c.is_ascii_digit()) { ctx.next(); }
            return Err("natural number overflow".into());
        }
        natural = natural * 10 + digit;
        ctx.next();
    }
    Ok(natural)
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(ref mut v) = opt {
        *v += offset;
    }
}
