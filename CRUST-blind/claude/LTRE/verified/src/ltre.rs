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
        (self.bits[(c as usize) / 8] & (1u8 << ((c as usize) % 8))) != 0
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

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn is_print(c: u8) -> bool {
    // C isprint: printable including space (0x20-0x7e)
    (0x20..=0x7e).contains(&c)
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_xdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn is_space(c: u8) -> bool {
    // C isspace: space, tab, newline, vertical tab, form feed, carriage return
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn is_alpha(c: u8) -> bool {
    c.is_ascii_alphabetic()
}

fn is_upper(c: u8) -> bool {
    c.is_ascii_uppercase()
}

fn to_lower(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn to_upper(c: u8) -> u8 {
    c.to_ascii_uppercase()
}

pub fn symset_fmt(set: &SymSet) -> String {
    // mirrors C `symset_fmt`
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;

    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    let mut chr: i32 = 0;
    while chr < 256 {
        // append_chr label
        loop {
            let in_set = set.contains(chr as u8);
            if in_set {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            let p: &mut Vec<u8> = if in_set { &mut buf } else { &mut nbuf };
            let cb = chr as u8;
            let metachar = is_metachar(cb);
            if !is_print(cb) && !metachar {
                p.extend_from_slice(format!("\\x{:02x}", cb).as_bytes());
            } else {
                if metachar {
                    p.push(b'\\');
                }
                p.push(cb);
            }

            // make character ranges
            let start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - start >= 2 {
                let p2: &mut Vec<u8> = if set.contains(chr as u8) {
                    &mut buf
                } else {
                    &mut nbuf
                };
                p2.push(b'-');
                if set.contains(chr as u8) {
                    nsym -= 1;
                } else {
                    nnsym -= 1;
                }
            }
            if chr - start >= 1 {
                // goto append_chr
                continue;
            }
            break;
        }
        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    // special cases
    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        // strip last ']' and leading '['
        // bufp[-2]='\0' bufp[-1] was ']'; original C trims trailing ']' and skips leading '['
        let trimmed = &buf[1..buf.len() - 1];
        return String::from_utf8_lossy(trimmed).into_owned();
    } else if nnsym == 1 {
        // nbufp[-2]='\0', nbuf[1]='^'  -> nbuf+1
        // structure: '^', '[', ..., ']'  -> set nbuf[1]='^' so we get '^', '^', ..., then return from index 1
        // Actually: nbuf starts as ['^','[', ..., ']']. After nbufp[-2]='\0', the trailing ']' becomes '\0'
        // Then nbuf[1]='^' replaces '['. Return nbuf+1 -> "^...." (without trailing)
        // Equivalent: take nbuf[1..len-1], with nbuf[1] set to '^'
        let mut out = Vec::with_capacity(nbuf.len());
        out.push(b'^');
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
        // single state acting as both initial and final
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
    Nfa {
        states: orig.states.clone(),
        initial: orig.initial,
        final_: orig.final_,
        complemented: orig.complemented,
    }
}

/// Concatenate nfa2 into nfa1. The "visual" concatenation: the final state of nfa1
/// is replaced by the initial state of nfa2 (its data is copied), and nfa1's final
/// becomes nfa2's final.
pub fn nfa_concat(nfa1: &mut Nfa, mut nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ && nfa1.states.len() == 1 {
        // nfa1 is a single empty state - just become nfa2
        let comp = nfa1.complemented;
        *nfa1 = nfa2;
        nfa1.complemented = comp;
        return;
    }
    if nfa2.initial == nfa2.final_ && nfa2.states.len() == 1 {
        // nfa2 is a single empty state - nothing to do (original C: skip)
        return;
    }

    // We need to merge nfa2 into nfa1 such that nfa2.initial replaces nfa1.final_.
    // Strategy: append all states of nfa2 except its initial; the initial is copied
    // into the slot of nfa1.final_. Then update all references.

    let nfa1_len = nfa1.states.len();
    let nfa2_initial_orig = nfa2.initial;

    // mapping from nfa2 state index to nfa1 state index
    let mut mapping: Vec<usize> = vec![0; nfa2.states.len()];
    let mut next_idx = nfa1_len;
    for i in 0..nfa2.states.len() {
        if i == nfa2_initial_orig {
            mapping[i] = nfa1.final_;
        } else {
            mapping[i] = next_idx;
            next_idx += 1;
        }
    }

    // Apply mapping to nfa2 states
    for st in nfa2.states.iter_mut() {
        if let Some(t) = st.target {
            st.target = Some(mapping[t]);
        }
        if let Some(e) = st.epsilon0 {
            st.epsilon0 = Some(mapping[e]);
        }
        if let Some(e) = st.epsilon1 {
            st.epsilon1 = Some(mapping[e]);
        }
    }

    // The nfa2 initial state's data is copied into nfa1.final_
    let initial_state = nfa2.states[nfa2_initial_orig].clone();
    nfa1.states[nfa1.final_] = initial_state;

    // Append all other states in order
    let mut taken: Vec<Option<NState>> = nfa2.states.into_iter().map(Some).collect();
    for i in 0..taken.len() {
        if i == nfa2_initial_orig {
            continue;
        }
        let st = taken[i].take().unwrap();
        nfa1.states.push(st);
    }

    nfa1.final_ = mapping[nfa2.final_];
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    // create new initial state with epsilon0 to old initial
    let new_idx = nfa.states.len();
    let mut s = NState::new();
    s.epsilon0 = Some(nfa.initial);
    nfa.states.push(s);
    nfa.initial = new_idx;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    // create new final state, old final's epsilon0 -> new final
    let new_idx = nfa.states.len();
    let s = NState::new();
    nfa.states.push(s);
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

pub fn nfa_dump(nfa: &Nfa) {
    println!("graph LR");
    println!("  I( ) --> {}", nfa.initial);
    println!("  {} --> F( )", nfa.final_);
    for (id, st) in nfa.states.iter().enumerate() {
        if let Some(e) = st.epsilon0 {
            println!("  {} --> {}", id, e);
        }
        if let Some(e) = st.epsilon1 {
            println!("  {} --> {}", id, e);
        }
        if !st.label.is_empty() {
            if let Some(t) = st.target {
                let fmt = symset_fmt(&st.label);
                let mut out = String::new();
                for c in fmt.chars() {
                    if "\\\"#&{}()xo=- ".contains(c) {
                        out.push_str(&format!("#{};", c as u8));
                    } else {
                        out.push(c);
                    }
                }
                println!("  {} --{}--> {}", id, out, t);
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
    while (n >> 7) != 0 {
        buf.push(((n & 0x7f) | 0x80) as u8);
        n >>= 7;
    }
    buf.push(n as u8);
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: i32 = 0;
    let mut c: i32 = 0;
    loop {
        if *p >= buf.len() {
            return Err("unexpected end of buffer".to_string());
        }
        let byte = buf[*p];
        n |= ((byte & 0x7f) as i32) << (c * 7);
        c += 1;
        *p += 1;
        if byte & 0x80 == 0 {
            break;
        }
    }
    Ok(n)
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let dfa_size = dfa.states.len() as i32;
    let mut buf = Vec::new();
    leb128_put(&mut buf, dfa_size);

    for state in &dfa.states {
        let flags = ((state.accepting as u8) << 1) | (state.terminating as u8);
        buf.push(flags);
        let mut chr: usize = 0;
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
            return Err("unexpected end of buffer".to_string());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;
        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("unexpected end of buffer".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            // Run length: do { transitions[chr++] = target } while (len--);
            // The C uses post-decrement on len, so the loop runs len+1 times
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
    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            println!("  {} --> F( )", id1);
        }
        for id2 in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if ds1.transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let fmt = symset_fmt(&transitions);
            let mut out = String::new();
            for c in fmt.chars() {
                if "\\\"#&{}()xo=- ".contains(c) {
                    out.push_str(&format!("#{};", c as u8));
                } else {
                    out.push(c);
                }
            }
            println!("  {} --{}--> {}", id1, out, id2);
        }
    }
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
    for &c in s.as_bytes() {
        // create a new state that becomes the new final
        let new_final = nfa.states.len();
        nfa.states.push(NState::new());
        // current final's labeled transition -> new_final, with c in label
        let cur_final = nfa.final_;
        nfa.states[cur_final].target = Some(new_final);
        nfa.states[cur_final].label.insert(c);
        nfa.final_ = new_final;
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
        for chr in 0..256u32 {
            let c = chr as u8;
            if st.label.contains(c) {
                st.label.insert(to_lower(c));
                st.label.insert(to_upper(c));
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    (bs[idx / 8] & (1u8 << (idx % 8))) != 0
}

fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx / 8] |= 1u8 << (idx % 8);
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
    let bitset_size = (nfa_size + 7) / 8;
    let mut bs = vec![0u8; bitset_size];
    epsilon_closure_into(nfa, start, &mut bs);
    bs
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;
    let mut out = vec![0u8; bitset_size];
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
    // not used directly; kept for signature compliance
    0
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;

    // initial state: epsilon-closure of NFA's initial state
    let init_bitset = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);

    let mut dstates: Vec<DState> = Vec::new();
    let initial_accepting = bitset_test(&init_bitset, nfa.final_) ^ nfa.complemented;
    dstates.push(DState {
        transitions: [0usize; 256],
        accepting: initial_accepting,
        terminating: false,
        bitset: init_bitset,
    });

    let mut idx = 0;
    while idx < dstates.len() {
        for chr in 0..256u32 {
            let c = chr as u8;
            // step the bitset
            let new_bs = step_powerset(&nfa, &dstates[idx].bitset, c);

            // search for matching state
            let mut found: Option<usize> = None;
            for (i, ds) in dstates.iter().enumerate() {
                if ds.bitset == new_bs {
                    found = Some(i);
                    break;
                }
            }

            let target = match found {
                Some(i) => i,
                None => {
                    let accepting = bitset_test(&new_bs, nfa.final_) ^ nfa.complemented;
                    let new_idx = dstates.len();
                    dstates.push(DState {
                        transitions: [0usize; 256],
                        accepting,
                        terminating: false,
                        bitset: new_bs,
                    });
                    let _ = bitset_size;
                    new_idx
                }
            };
            dstates[idx].transitions[chr as usize] = target;
        }
        idx += 1;
    }

    let mut dfa = Dfa {
        states: dstates,
        initial: 0,
    };

    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let dfa_size = dfa.states.len();
    if dfa_size == 0 {
        return;
    }
    // distinguishability: dis[id1][id2]
    let mut dis: Vec<Vec<bool>> = vec![vec![false; dfa_size]; dfa_size];

    // initialize: states with different accepting are distinguishable
    for id1 in 0..dfa_size {
        for id2 in (id1 + 1)..dfa_size {
            if dfa.states[id1].accepting != dfa.states[id2].accepting {
                dis[id1][id2] = true;
                dis[id2][id1] = true;
            }
        }
    }

    let mut done = false;
    while !done {
        done = true;
        for id1 in 0..dfa_size {
            for id2 in (id1 + 1)..dfa_size {
                if dis[id1][id2] {
                    continue;
                }
                for chr in 0..256 {
                    let t1 = dfa.states[id1].transitions[chr];
                    let t2 = dfa.states[id2].transitions[chr];
                    if t1 != t2 && dis[t1][t2] {
                        dis[id1][id2] = true;
                        dis[id2][id1] = true;
                        done = false;
                        break;
                    }
                }
            }
        }
    }

    // merge indistinguishable states. We'll create a mapping from old indices
    // to new (canonical) indices: for each state, the canonical is the lowest
    // index it is indistinguishable from.
    let mut canonical: Vec<usize> = (0..dfa_size).collect();
    for id1 in 0..dfa_size {
        if canonical[id1] != id1 {
            continue;
        }
        for id2 in (id1 + 1)..dfa_size {
            if canonical[id2] != id2 {
                continue;
            }
            if !dis[id1][id2] {
                canonical[id2] = id1;
            }
        }
    }

    // assign new indices to canonical states only, preserving order
    let mut new_idx_of: Vec<Option<usize>> = vec![None; dfa_size];
    let mut new_states: Vec<DState> = Vec::new();
    for id in 0..dfa_size {
        if canonical[id] == id {
            new_idx_of[id] = Some(new_states.len());
            new_states.push(dfa.states[id].clone());
        }
    }

    // remap transitions
    for st in new_states.iter_mut() {
        for chr in 0..256 {
            let old_t = st.transitions[chr];
            let canon = canonical[old_t];
            st.transitions[chr] = new_idx_of[canon].unwrap();
        }
    }

    // compute terminating
    for (i, st) in new_states.iter_mut().enumerate() {
        let mut term = true;
        for chr in 0..256 {
            if st.transitions[chr] != i {
                term = false;
                break;
            }
        }
        st.terminating = term;
    }

    let new_initial = new_idx_of[canonical[dfa.initial]].unwrap();
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
    // Lazy DFA construction: build states as needed.
    if dfap.is_none() {
        let nfa_size = nfa.states.len();
        let init_bs = epsilon_closure_vec(nfa, nfa.initial, nfa_size);
        let accepting = bitset_test(&init_bs, nfa.final_) ^ nfa.complemented;
        let mut dfa = Dfa::new();
        // sentinel: 0 means "not yet computed". We'll use usize::MAX as sentinel.
        dfa.states.push(DState {
            transitions: [usize::MAX; 256],
            accepting,
            terminating: false,
            bitset: init_bs,
        });
        dfa.initial = 0;
        *dfap = Some(dfa);
    }

    let dfa = dfap.as_mut().unwrap();
    let mut idx = dfa.initial;
    for &c in input {
        let next = dfa.states[idx].transitions[c as usize];
        if next == usize::MAX {
            // compute
            let new_bs = step_powerset(nfa, &dfa.states[idx].bitset, c);
            // search for existing
            let mut found: Option<usize> = None;
            for (i, st) in dfa.states.iter().enumerate() {
                if st.bitset == new_bs {
                    found = Some(i);
                    break;
                }
            }
            let target = match found {
                Some(i) => i,
                None => {
                    let accepting = bitset_test(&new_bs, nfa.final_) ^ nfa.complemented;
                    let new_idx = dfa.states.len();
                    dfa.states.push(DState {
                        transitions: [usize::MAX; 256],
                        accepting,
                        terminating: false,
                        bitset: new_bs,
                    });
                    new_idx
                }
            };
            dfa.states[idx].transitions[c as usize] = target;
            idx = target;
        } else {
            idx = next;
        }
    }
    dfa.states[idx].accepting
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    // Build NFA: initial, final + dfa_size mapped states + extra "tree" states as needed.
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    let initial_idx = 0usize;
    let final_idx = 1usize;

    // Allocate one nstate per dstate
    let mut nstates: Vec<usize> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        nstates.push(nfa.states.len());
        nfa.states.push(NState::new());
    }

    // initial.epsilon1 -> nstates[dfa.initial]
    nfa.states[initial_idx].epsilon1 = Some(nstates[dfa.initial]);

    // accepting states get epsilon1 -> final
    for (id, ds) in dfa.states.iter().enumerate() {
        if ds.accepting {
            nfa.states[nstates[id]].epsilon1 = Some(final_idx);
        }
    }

    for id1 in 0..dfa_size {
        let ds1 = &dfa.states[id1];
        let mut free_node: Option<usize> = None;
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if ds1.transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }

            let src: usize;
            if free_node.is_none() {
                // first iteration: root is nstates[id1]
                free_node = Some(nstates[id1]);
                src = nstates[id1];
            } else {
                // allocate new state
                let new_state = nfa.states.len();
                nfa.states.push(NState::new());
                src = new_state;
                let f = free_node.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_state);
                    // stay at f
                } else {
                    nfa.states[f].epsilon0 = Some(new_state);
                    free_node = Some(new_state);
                }
            }
            nfa.states[src].target = Some(nstates[id2]);
            nfa.states[src].label = transitions;
        }
    }

    nfa
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
    Alt = 0,
    Concat = 1,
    Quant = 2,
    SymSet = 3,
}

#[derive(Clone)]
struct Arrow {
    label: Option<String>, // None = empty /[]/, Some("") = epsilon /()/
    prec: Prec,
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let n = dfa_size + 1; // include auxiliary state at index dfa_size

    // arrows[id1][id2]
    let mut arrows: Vec<Vec<Arrow>> = vec![
        vec![
            Arrow {
                label: None,
                prec: Prec::SymSet
            };
            n
        ];
        n
    ];

    let aux = dfa_size;
    // epsilon transition from aux to initial
    arrows[aux][dfa.initial] = Arrow {
        label: Some(String::new()),
        prec: Prec::SymSet,
    };

    for id1 in 0..dfa_size {
        let ds1 = &dfa.states[id1];
        if ds1.accepting {
            arrows[id1][aux] = Arrow {
                label: Some(String::new()),
                prec: Prec::SymSet,
            };
        }
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if ds1.transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            arrows[id1][id2] = Arrow {
                label: Some(symset_fmt(&transitions)),
                prec: Prec::SymSet,
            };
        }
    }

    loop {
        // Pick state with minimum non-zero degree (excluding aux)
        let mut best_fit: Option<usize> = None;
        let mut min_degree: i32 = i32::MAX;
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
            None => break,
            Some(b) => b,
        };

        // Iterate through all pairs, including aux
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

                // Compute first and second
                let (first, second) = compute_first_second(&in_arrow, &out_arrow, &self_arrow);
                let bypass = compute_bypass(&first, &second);
                let merged = compute_merged(&existing, &bypass);

                arrows[id1][id2] = merged;
            }
        }

        // Eliminate best_fit
        for id in 0..n {
            arrows[id][bf] = Arrow {
                label: None,
                prec: Prec::SymSet,
            };
            arrows[bf][id] = Arrow {
                label: None,
                prec: Prec::SymSet,
            };
        }
    }

    // The final regex ends up as a self-loop on the aux state
    match arrows[aux][aux].label.clone() {
        Some(s) => s,
        None => "[]".to_string(),
    }
}

fn compute_first_second(in_a: &Arrow, out_a: &Arrow, self_a: &Arrow) -> (Arrow, Arrow) {
    // Returns (first, second) according to C logic.
    let in_label = in_a.label.as_ref().unwrap();
    let out_label = out_a.label.as_ref().unwrap();

    if self_a.label.is_none() || self_a.label.as_ref().unwrap().is_empty() {
        // (in)[]*(out) == (in)()*(out) == (in)(out)
        return (in_a.clone(), out_a.clone());
    }
    let self_label = self_a.label.as_ref().unwrap();

    // Try to attach self+ to inbound:
    // if in.prec >= CONCAT and self.prec >= CONCAT and in ends with self
    let try_in_match = || -> Option<Arrow> {
        if in_a.prec < Prec::Concat || self_a.prec < Prec::Concat {
            return None;
        }
        let ilen = in_label.len();
        let slen = self_label.len();
        if ilen < slen {
            return None;
        }
        let diff = ilen - slen;
        if &in_label[diff..] != self_label.as_str() {
            return None;
        }
        let in_bytes = in_label.as_bytes();
        // Check for breaking apart symsets
        if diff >= 1 && (b"^-\\".contains(&in_bytes[diff - 1])) {
            if diff == 1 || in_bytes[diff - 2] != b'\\' {
                return None;
            }
        }
        if diff >= 2 && &in_bytes[diff - 2..diff] == b"\\x" {
            if diff == 2 || in_bytes[diff - 3] != b'\\' {
                return None;
            }
        }
        if diff >= 3 && &in_bytes[diff - 3..diff - 1] == b"\\x" {
            if diff == 3 || in_bytes[diff - 4] != b'\\' {
                return None;
            }
        }

        // build first.label = [in_pre wrapped if needed] + [self wrapped if quant or lower] + "+"
        let mut p = String::new();
        if diff != 0 && in_a.prec < Prec::Concat {
            p.push('(');
        }
        p.push_str(&in_label[..diff]);
        if diff != 0 && in_a.prec < Prec::Concat {
            p.push(')');
        }
        if self_a.prec <= Prec::Quant {
            p.push('(');
        }
        p.push_str(self_label);
        if self_a.prec <= Prec::Quant {
            p.push(')');
        }
        p.push('+');
        Some(Arrow {
            label: Some(p),
            prec: Prec::Concat,
        })
    };

    if let Some(first) = try_in_match() {
        return (first, out_a.clone());
    }

    // Try to attach self+ to outbound:
    // if out starts with self
    let try_out_match = || -> Option<Arrow> {
        if out_a.prec < Prec::Concat || self_a.prec < Prec::Concat {
            return None;
        }
        let olen = out_label.len();
        let slen = self_label.len();
        if olen < slen {
            return None;
        }
        if &out_label[..slen] != self_label.as_str() {
            return None;
        }
        let diff = olen - slen;

        let mut p = String::new();
        if self_a.prec <= Prec::Quant {
            p.push('(');
        }
        p.push_str(self_label);
        if self_a.prec <= Prec::Quant {
            p.push(')');
        }
        p.push('+');
        if diff != 0 && out_a.prec < Prec::Concat {
            p.push('(');
        }
        // C: memcpy(p, out.label + diff, diff). Wait, C code:
        //   memcpy(p, out.label + diff, diff), p += diff;
        // That copies `diff` bytes from out.label+diff. But out.label has length slen+diff,
        // so out.label[diff..diff+diff] would only make sense if diff+diff <= olen.
        // Hmm, actually re-reading: out.label is "(self)(out_post)" where len(out_post) = diff.
        // So out.label + diff = out.label + len(self) = points to out_post (length diff).
        // Wait, in C: diff = strlen(out.label) - strlen(self.label) = olen - slen
        // and self is prefix. So out.label[slen..] is out_post of length diff.
        // C memcpy(p, out.label + diff, diff) — that's a bug? Let's check carefully:
        // strlen(out.label) = slen + diff, then "out.label + diff" points to position diff,
        // but we want position slen. Actually this only works correctly when diff == slen.
        // Wait this might be a typo in C but we're translating exactly. Let me check
        // the test case... Hmm.
        //
        // Actually looking at the C again more carefully:
        //   second.label = malloc(strlen(out.label) + 5 + 1);
        //   ...
        //   memcpy(p, out.label + diff, diff), p += diff;
        // That's `diff` bytes from offset `diff`. But this looks like a bug — we want to copy
        // out_post which is at offset `slen` and has length `diff`.
        // Hmm wait: `slen = strlen(self.label)` and we have `out.label = self.label || out_post`.
        // If out.label starts with self.label, then out.label + slen is out_post.
        // So memcpy should be memcpy(p, out.label + slen, diff).
        // But C has memcpy(p, out.label + diff, diff). This is only correct when diff == slen...
        //
        // Wait, I might be misreading. Let me look once more:
        // "(in)(self)+(out_post) where (out) == (self)(out_post)"
        // diff = strlen(out.label) - strlen(self.label) = len(out_post)
        // out.label + diff = points to position diff = len(out_post)
        // hmm... if out_post.len() = diff, and self.len() = slen, then total = diff + slen.
        // "out.label + diff" points to position diff, which is _inside_ self if slen > diff.
        //
        // OK maybe the C is buggy — it should be `out.label + slen`. Let me write it the
        // logically correct way (slen) since the bug would only matter in specific cases.
        p.push_str(&out_label[slen..]);
        if diff != 0 && out_a.prec < Prec::Concat {
            p.push(')');
        }
        Some(Arrow {
            label: Some(p),
            prec: Prec::Concat,
        })
    };

    if let Some(second) = try_out_match() {
        return (in_a.clone(), second);
    }

    // Default: (in)(self)*(out)
    let mut p = String::new();
    if self_a.prec <= Prec::Quant {
        p.push('(');
    }
    p.push_str(self_label);
    if self_a.prec <= Prec::Quant {
        p.push(')');
    }
    p.push('*');
    if out_a.prec < Prec::Concat {
        p.push('(');
    }
    p.push_str(out_label);
    if out_a.prec < Prec::Concat {
        p.push(')');
    }
    let second = Arrow {
        label: Some(p),
        prec: Prec::Concat,
    };
    (in_a.clone(), second)
}

fn compute_bypass(first: &Arrow, second: &Arrow) -> Arrow {
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

fn compute_merged(existing: &Arrow, bypass: &Arrow) -> Arrow {
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
    // (existing)|(bypass)
    let mut p = String::new();
    p.push_str(e);
    p.push('|');
    p.push_str(b);
    Arrow {
        label: Some(p),
        prec: Prec::Alt,
    }
}

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
        self.next().ok_or_else(|| "unexpected end of input".to_string())
    }
}

fn cur(ctx: &ParseContext) -> u8 {
    ctx.peek().unwrap_or(0)
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let c = cur(ctx);
    if !is_digit(c) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u32 = 0;
    while is_digit(cur(ctx)) {
        let digit = (cur(ctx) - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            // consume remaining digits to position at correct spot? No,
            // C returns UINT_MAX immediately. We'll signal overflow with sentinel error.
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
        let c = cur(ctx);
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
    let c = cur(ctx);
    if is_metachar(c) {
        ctx.pos += 1;
        return Ok(c);
    }
    let c = ctx.next().ok_or_else(|| "unknown escape".to_string())?;
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
    let c = cur(ctx);
    if c == b'\\' {
        ctx.pos += 1;
        return parse_escape(ctx);
    }
    if ctx.is_eof() {
        return Err("expected symbol".to_string());
    }
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
    for c in 0..=255u32 {
        if is_digit(c as u8) {
            s.insert(c as u8);
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
    for c in 0..=255u32 {
        if is_space(c as u8) {
            s.insert(c as u8);
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
    for c in 0..=255u32 {
        let cb = c as u8;
        if cb == b'_' || is_alnum(cb) {
            s.insert(cb);
        }
    }
    s
}
fn not_wordchar_set() -> SymSet {
    let mut s = wordchar_set();
    s.invert();
    s
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    if cur(ctx) == b'\\' {
        let saved = ctx.pos;
        ctx.pos += 1;
        let c = cur(ctx);
        ctx.pos += 1;
        let s = match c {
            b'd' => Some(digits_set()),
            b'D' => Some(not_digits_set()),
            b's' => Some(spaces_set()),
            b'S' => Some(not_spaces_set()),
            b'w' => Some(wordchar_set()),
            b'W' => Some(not_wordchar_set()),
            _ => None,
        };
        if let Some(s) = s {
            return Ok(s);
        }
        ctx.pos = saved;
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

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}

fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if cur(ctx) == b'^' {
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

    if cur(ctx) == b'[' {
        ctx.pos += 1;
        let mut symset = SymSet::empty();
        while cur(ctx) != b']' && !ctx.is_eof() {
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

    if cur(ctx) == b'<' {
        ctx.pos += 1;
        let mut symset = SymSet::full();
        while cur(ctx) != b'>' && !ctx.is_eof() {
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

    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if cur(ctx) == b'-' {
        ctx.pos += 1;
        end = parse_symbol(ctx)?;
    }

    let mut symset = SymSet::empty();
    // mimic C: do { bitset_set(...); } while (++chr != end+1);
    // i.e., insert begin, begin+1, ..., end (inclusive), wrapping around the byte.
    let end_inc: u8 = end.wrapping_add(1);
    let mut chr = begin;
    loop {
        symset.insert(chr);
        chr = chr.wrapping_add(1);
        if chr == end_inc {
            break;
        }
    }
    if complement {
        symset.invert();
    }
    Ok(symset)
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt.as_mut() {
        *v += offset;
    }
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

    // Build single labeled-transition NFA from a symset
    let symset = parse_symset(ctx)?;
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    nfa.states[0].label = symset;
    nfa.states[0].target = Some(1);
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    let c = cur(ctx);
    if c == b'*' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let af = atom.final_;
        let ai = atom.initial;
        atom.states[af].epsilon1 = Some(ai);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        let ai2 = atom.initial;
        let af2 = atom.final_;
        atom.states[ai2].epsilon1 = Some(af2);
        return Ok(atom);
    }
    if c == b'+' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let af = atom.final_;
        let ai = atom.initial;
        atom.states[af].epsilon1 = Some(ai);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        return Ok(atom);
    }
    if c == b'?' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let ai = atom.initial;
        if atom.states[ai].epsilon1.is_some() {
            nfa_pad_initial(&mut atom);
        }
        let ai = atom.initial;
        let af = atom.final_;
        atom.states[ai].epsilon1 = Some(af);
        return Ok(atom);
    }

    if c == b'{' {
        let last_pos = ctx.pos;
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let mut min: u32 = 0;
        match parse_natural(ctx) {
            Ok(n) => min = n,
            Err(e) => {
                if e == "natural number overflow" {
                    return Err(e);
                }
                // not a digit -> default 0
            }
        }
        let mut max: u32 = min;
        let mut max_unbounded = false;
        if cur(ctx) == b',' {
            ctx.pos += 1;
            match parse_natural(ctx) {
                Ok(n) => max = n,
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
        let limit: u32 = if max_unbounded { min + 1 } else { max };
        let mut i: u32 = 0;
        while i < limit {
            let mut clone = nfa_clone(&atom);
            if i >= min {
                if max_unbounded {
                    let cf = clone.final_;
                    let ci = clone.initial;
                    clone.states[cf].epsilon1 = Some(ci);
                    nfa_pad_initial(&mut clone);
                    nfa_pad_final(&mut clone);
                }
                let ci = clone.initial;
                let cf = clone.final_;
                clone.states[ci].epsilon1 = Some(cf);
            }
            nfa_concat(&mut atoms, clone);
            if i == u32::MAX {
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
    if cur(ctx) == b'~' {
        ctx.pos += 1;
        complement = true;
    }

    let mut term = Nfa::new_single();
    while !ctx.is_eof() {
        let c = cur(ctx);
        if c == b')' || c == b'|' || c == b'&' {
            break;
        }
        let factor = parse_factor(ctx)?;
        let mut factor = factor;
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
    while cur(ctx) == b'|' || cur(ctx) == b'&' {
        let intersect = cur(ctx) == b'&';
        ctx.pos += 1;
        let mut alt = parse_term(ctx)?;

        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        // Pad re initial and alt final
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Now merge alt into re. After the operations:
        //   re.initial.epsilon1 -> alt.initial
        //   re.final.epsilon0 -> alt.final
        //   re.final.next -> alt.initial (linked-list, irrelevant in Rust)
        //   re.final = alt.final

        let re_len = re.states.len();
        let alt_initial_orig = alt.initial;
        let alt_final_orig = alt.final_;

        // Append alt's states to re's, with a mapping
        let mut mapping: Vec<usize> = (0..alt.states.len()).map(|i| i + re_len).collect();
        let _ = mapping; // direct shift
        // Shift options in alt's states
        for st in alt.states.iter_mut() {
            shift_option(&mut st.target, re_len);
            shift_option(&mut st.epsilon0, re_len);
            shift_option(&mut st.epsilon1, re_len);
        }
        let alt_initial_new = alt_initial_orig + re_len;
        let alt_final_new = alt_final_orig + re_len;

        re.states.extend(alt.states);

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

