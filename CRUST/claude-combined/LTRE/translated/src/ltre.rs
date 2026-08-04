// LTRE: regular expression engine - Rust port

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_print(c: u8) -> bool {
    c >= 0x20 && c < 0x7f
}

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
        (self.bits[(c >> 3) as usize] >> (c & 7)) & 1 != 0
    }
    pub fn insert(&mut self, c: u8) {
        self.bits[(c >> 3) as usize] |= 1u8 << (c & 7);
    }
    pub fn invert(&mut self) {
        for b in &mut self.bits {
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

pub fn symset_fmt(set: &SymSet) -> String {
    // produce a parsable representation. mirrors C `symset_fmt`.
    let mut buf = String::new();
    let mut nbuf = String::new();
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;

    nbuf.push('^');
    buf.push('[');
    nbuf.push('[');

    let mut chr: usize = 0;
    while chr < 256 {
        loop {
            let in_set = set.contains(chr as u8);
            if in_set {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            let is_metachar = chr != 0 && METACHARS.contains(&(chr as u8));
            let s: String = if !is_print(chr as u8) && !is_metachar {
                format!("\\x{:02x}", chr)
            } else if is_metachar {
                let mut s = String::from("\\");
                s.push(chr as u8 as char);
                s
            } else {
                let mut s = String::new();
                s.push(chr as u8 as char);
                s
            };
            if in_set {
                buf.push_str(&s);
            } else {
                nbuf.push_str(&s);
            }

            let start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - start >= 2 {
                if in_set {
                    buf.push('-');
                    nsym -= 1;
                } else {
                    nbuf.push('-');
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

    buf.push(']');
    nbuf.push(']');

    if nnsym == 0 {
        return "<>".to_string();
    }
    if nsym == 1 {
        // strip leading '[' and trailing ']'
        let inner = &buf[1..buf.len() - 1];
        return inner.to_string();
    }
    if nnsym == 1 {
        // nbuf starts with "^[" and ends with "]"; produce "^" + inner
        let inner = &nbuf[2..nbuf.len() - 1];
        return format!("^{}", inner);
    }

    if buf.len() < nbuf.len() {
        buf
    } else {
        nbuf
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
        // nfa1 is just one state (no transitions). Replace with nfa2 entirely.
        let mut nfa2 = nfa2;
        // need to keep nfa1's complemented? In C it discards nfa1 and overwrites with nfa2.
        nfa1.states = std::mem::take(&mut nfa2.states);
        nfa1.initial = nfa2.initial;
        nfa1.final_ = nfa2.final_;
        nfa1.complemented = nfa2.complemented;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        return;
    }
    // Merge: nfa2.initial's contents are placed at nfa1.final_; remaining nfa2 states appended.
    let nfa1_len = nfa1.states.len();
    let mut map = vec![0usize; nfa2.states.len()];
    let mut next_idx = nfa1_len;
    for i in 0..nfa2.states.len() {
        if i == nfa2.initial {
            map[i] = nfa1.final_;
        } else {
            map[i] = next_idx;
            next_idx += 1;
        }
    }

    nfa1.states.reserve(nfa2.states.len() - 1);

    for (i, mut s) in nfa2.states.into_iter().enumerate() {
        if let Some(t) = s.target {
            s.target = Some(map[t]);
        }
        if let Some(t) = s.epsilon0 {
            s.epsilon0 = Some(map[t]);
        }
        if let Some(t) = s.epsilon1 {
            s.epsilon1 = Some(map[t]);
        }
        if i == nfa2.initial {
            nfa1.states[nfa1.final_] = s;
        } else {
            nfa1.states.push(s);
        }
    }
    nfa1.final_ = map[nfa2.final_];
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
    let f = nfa.final_;
    nfa.states[f].epsilon0 = Some(new_idx);
    nfa.final_ = new_idx;
}

pub fn nfa_uncomplement(nfa: &mut Nfa) -> Result<(), String> {
    if !nfa.complemented {
        return Ok(());
    }
    let cloned = nfa.clone();
    let dfa = ltre_compile(cloned);
    let uncomplemented = ltre_uncompile(&dfa);
    *nfa = uncomplemented;
    Ok(())
}

pub fn nfa_dump(nfa: &Nfa) {
    println!("graph LR");
    println!("  I( ) --> {}", nfa.initial);
    println!("  {} --> F( )", nfa.final_);
    for (i, s) in nfa.states.iter().enumerate() {
        if let Some(e) = s.epsilon0 {
            println!("  {} --> {}", i, e);
        }
        if let Some(e) = s.epsilon1 {
            println!("  {} --> {}", i, e);
        }
        if !s.label.is_empty() {
            if let Some(t) = s.target {
                println!("  {} --{}--> {}", i, symset_fmt(&s.label), t);
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

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf = Vec::new();
    leb128_put(&mut buf, dfa.states.len() as i32);
    for state in &dfa.states {
        let flag: u8 = ((state.accepting as u8) << 1) | (state.terminating as u8);
        buf.push(flag);
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            while chr < 255 && state.transitions[chr] == state.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8);
            leb128_put(&mut buf, state.transitions[chr] as i32);
            chr += 1;
        }
    }
    buf
}

pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p = 0usize;
    let dfa_size = leb128_get(buf, &mut p)? as usize;
    let mut states = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        states.push(DState {
            transitions: [0usize; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        });
    }
    for id in 0..dfa_size {
        if p >= buf.len() {
            return Err("buffer underflow".to_string());
        }
        let flag = buf[p];
        p += 1;
        states[id].accepting = (flag >> 1) & 1 != 0;
        states[id].terminating = flag & 1 != 0;
        let mut chr = 0usize;
        while chr < 256 {
            if p >= buf.len() {
                return Err("buffer underflow".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            // run length: len + 1 occurrences
            let mut count = len + 1;
            while count > 0 && chr < 256 {
                states[id].transitions[chr] = target;
                chr += 1;
                count -= 1;
            }
        }
    }
    Ok((Dfa { states, initial: 0 }, p))
}

pub fn dfa_dump(dfa: &Dfa) {
    println!("graph LR");
    println!("  I( ) --> {}", dfa.initial);
    for (i, s) in dfa.states.iter().enumerate() {
        if s.accepting {
            println!("  {} --> F( )", i);
        }
        for j in 0..dfa.states.len() {
            let mut sym = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if s.transitions[chr as usize] == j {
                    sym.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            println!("  {} --{}--> {}", i, symset_fmt(&sym), j);
        }
    }
}

fn leb128_put(buf: &mut Vec<u8>, n: i32) {
    let mut n = n as u32;
    while n >> 7 != 0 {
        buf.push(((n & 0x7f) | 0x80) as u8);
        n >>= 7;
    }
    buf.push((n & 0x7f) as u8);
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: u32 = 0;
    let mut c: u32 = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128 buffer underflow".to_string());
        }
        let b = buf[*p];
        *p += 1;
        n |= ((b & 0x7f) as u32) << (c * 7);
        c += 1;
        if b & 0x80 == 0 {
            break;
        }
        if c * 7 >= 32 {
            return Err("leb128 overflow".to_string());
        }
    }
    Ok(n as i32)
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
        let cur_final = nfa.final_;
        let new_idx = nfa.states.len();
        nfa.states.push(NState::new());
        let mut sym = SymSet::empty();
        sym.insert(b);
        nfa.states[cur_final].label = sym;
        nfa.states[cur_final].target = Some(new_idx);
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
    for state in &mut nfa.states {
        let mut new_label = state.label;
        for c in 0u32..256 {
            let c = c as u8;
            if state.label.contains(c) {
                let lc = c.to_ascii_lowercase();
                let uc = c.to_ascii_uppercase();
                new_label.insert(lc);
                new_label.insert(uc);
            }
        }
        state.label = new_label;
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;

    let mut dfa = Dfa::new();

    // Initial DFA state: epsilon-closure of nfa.initial
    let initial_bitset = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let final_id = nfa.final_;
    let initial_accepting = bitset_test(&initial_bitset, final_id) ^ nfa.complemented;
    dfa.states.push(DState {
        transitions: [0usize; 256],
        accepting: initial_accepting,
        terminating: false,
        bitset: initial_bitset,
    });
    dfa.initial = 0;

    let mut i = 0usize;
    while i < dfa.states.len() {
        for chr_u in 0u32..256 {
            let chr = chr_u as u8;
            let cur_bitset = dfa.states[i].bitset.clone();
            let new_bitset = step_powerset(&nfa, &cur_bitset, chr);
            // Find or insert
            let target = if let Some(idx) = dfa.states.iter().position(|s| s.bitset == new_bitset) {
                idx
            } else {
                let accepting = bitset_test(&new_bitset, final_id) ^ nfa.complemented;
                dfa.states.push(DState {
                    transitions: [0usize; 256],
                    accepting,
                    terminating: false,
                    bitset: new_bitset,
                });
                dfa.states.len() - 1
            };
            dfa.states[i].transitions[chr as usize] = target;
        }
        i += 1;
    }

    let _ = bitset_size; // unused after build
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    // Find a state that has all-zero bitset and is non-accepting and only loops to itself.
    for (i, s) in states.iter().enumerate() {
        if !s.accepting && s.transitions.iter().all(|&t| t == i) {
            return i;
        }
    }
    let new_idx = states.len();
    let dead = DState {
        transitions: [new_idx; 256],
        accepting: false,
        terminating: true,
        bitset: Vec::new(),
    };
    states.push(dead);
    new_idx
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

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let mut bitset = vec![0u8; (nfa_size + 7) / 8];
    epsilon_closure_into(nfa, start, &mut bitset);
    bitset
}

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    // iterative DFS to avoid stack overflow on deep regexes
    let mut stack = vec![st_id];
    while let Some(id) = stack.pop() {
        if bitset_test(bitset, id) {
            continue;
        }
        bitset_set(bitset, id);
        if let Some(e) = nfa.states[id].epsilon0 {
            if !bitset_test(bitset, e) {
                stack.push(e);
            }
        }
        if let Some(e) = nfa.states[id].epsilon1 {
            if !bitset_test(bitset, e) {
                stack.push(e);
            }
        }
    }
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }
    let row_size = (n + 7) / 8;
    let mut dis = vec![0u8; n * row_size];

    let idx = |i: usize, j: usize| i * row_size + j / 8;
    let mask = |j: usize| 1u8 << (j % 8);

    // Initial: states with different accepting are distinguishable
    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                dis[idx(i, j)] |= mask(j);
                dis[idx(j, i)] |= mask(i);
            }
        }
    }

    // Iterate until stable
    let mut done = false;
    while !done {
        done = true;
        for i in 0..n {
            for j in (i + 1)..n {
                if dis[idx(i, j)] & mask(j) != 0 {
                    continue;
                }
                let mut newly_dis = false;
                for chr in 0..256 {
                    let ti = dfa.states[i].transitions[chr];
                    let tj = dfa.states[j].transitions[chr];
                    if ti != tj {
                        let (a, b) = (ti.min(tj), ti.max(tj));
                        if dis[idx(a, b)] & mask(b) != 0 {
                            newly_dis = true;
                            break;
                        }
                    }
                }
                if newly_dis {
                    dis[idx(i, j)] |= mask(j);
                    dis[idx(j, i)] |= mask(i);
                    done = false;
                }
            }
        }
    }

    // Determine equivalence classes: equiv[i] = canonical (smallest index) equivalent to i
    let mut equiv = vec![0usize; n];
    for i in 0..n {
        equiv[i] = i;
        for j in 0..i {
            if dis[idx(i, j)] & mask(j) == 0 {
                equiv[i] = equiv[j];
                break;
            }
        }
    }

    // Build new DFA: only canonical states.
    let mut new_idx_of = vec![0usize; n];
    let mut next = 0usize;
    for i in 0..n {
        if equiv[i] == i {
            new_idx_of[i] = next;
            next += 1;
        }
    }
    for i in 0..n {
        if equiv[i] != i {
            new_idx_of[i] = new_idx_of[equiv[i]];
        }
    }

    let mut new_states: Vec<DState> = Vec::with_capacity(next);
    for i in 0..n {
        if equiv[i] != i {
            continue;
        }
        let mut transitions = [0usize; 256];
        for chr in 0..256 {
            transitions[chr] = new_idx_of[dfa.states[i].transitions[chr]];
        }
        let self_idx = new_idx_of[i];
        let terminating = transitions.iter().all(|&t| t == self_idx);
        new_states.push(DState {
            transitions,
            accepting: dfa.states[i].accepting,
            terminating,
            bitset: Vec::new(),
        });
    }

    dfa.initial = new_idx_of[dfa.initial];
    dfa.states = new_states;
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    (bs[idx >> 3] >> (idx & 7)) & 1 != 0
}

fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx >> 3] |= 1u8 << (idx & 7);
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
        *dfap = Some(ltre_compile(nfa.clone()));
    }
    let dfa = dfap.as_ref().unwrap();
    ltre_matches(dfa, input)
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    let mut states: Vec<NState> = Vec::new();

    let initial_idx = states.len();
    states.push(NState::new()); // initial at 0

    // Allocate one nstate per dfa state (the "root" of the binary tree).
    let mut nstates: Vec<usize> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        nstates.push(states.len());
        states.push(NState::new());
    }

    // Allocate final state next so that subsequent allocations append after it
    // (matches the C order where `tail->next = nstate_alloc()` extends the linked
    // list past `nfa.final` is irrelevant — but we must mark accepting states with
    // epsilon1 = final BEFORE the transition loop).
    let final_idx = states.len();
    states.push(NState::new());

    states[initial_idx].epsilon1 = Some(nstates[dfa.initial]);

    // Mark accepting states' epsilon1 = final BEFORE building transitions
    for id in 0..dfa_size {
        if dfa.states[id].accepting {
            states[nstates[id]].epsilon1 = Some(final_idx);
        }
    }

    for ds1_id in 0..dfa_size {
        let mut free_idx: Option<usize> = None;
        for ds2_id in 0..dfa_size {
            let mut sym = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if dfa.states[ds1_id].transitions[chr as usize] == ds2_id {
                    sym.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }

            let src_idx = match free_idx {
                None => {
                    free_idx = Some(nstates[ds1_id]);
                    nstates[ds1_id]
                }
                Some(fi) => {
                    let new_idx = states.len();
                    states.push(NState::new());

                    if states[fi].epsilon1.is_none() {
                        states[fi].epsilon1 = Some(new_idx);
                    } else {
                        states[fi].epsilon0 = Some(new_idx);
                        free_idx = Some(new_idx);
                    }
                    new_idx
                }
            };

            states[src_idx].target = Some(nstates[ds2_id]);
            states[src_idx].label = sym;
        }
    }

    Nfa {
        states,
        initial: initial_idx,
        final_: final_idx,
        complemented: false,
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Prec {
    Alt = 0,
    Concat = 1,
    Quant = 2,
    Symset = 3,
}

#[derive(Clone)]
struct Arrow {
    label: Option<String>,
    prec: Prec,
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let n = dfa_size + 1; // include aux state at index dfa_size
    let aux = dfa_size;

    // arrows[i][j]
    let mut arrows: Vec<Vec<Arrow>> = (0..n)
        .map(|_| {
            (0..n)
                .map(|_| Arrow {
                    label: None,
                    prec: Prec::Symset,
                })
                .collect()
        })
        .collect();

    // epsilon transition aux -> dfa.initial
    arrows[aux][dfa.initial].label = Some(String::new());
    arrows[aux][dfa.initial].prec = Prec::Symset;

    for id1 in 0..dfa_size {
        if dfa.states[id1].accepting {
            arrows[id1][aux].label = Some(String::new());
            arrows[id1][aux].prec = Prec::Symset;
        }
        for id2 in 0..dfa_size {
            let mut sym = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if dfa.states[id1].transitions[chr as usize] == id2 {
                    sym.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let fmt = symset_fmt(&sym);
            arrows[id1][id2].label = Some(fmt);
            arrows[id1][id2].prec = Prec::Symset;
        }
    }

    loop {
        // pick best fit
        let mut best_fit: Option<usize> = None;
        let mut min_degree = usize::MAX;
        for id1 in 0..dfa_size {
            let mut degree = 0;
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
        let best_fit = match best_fit {
            None => break,
            Some(b) => b,
        };

        for id1 in 0..n {
            if id1 == best_fit {
                continue;
            }
            for id2 in 0..n {
                if id2 == best_fit {
                    continue;
                }
                let in_arr = arrows[id1][best_fit].clone();
                let out_arr = arrows[best_fit][id2].clone();
                let self_arr = arrows[best_fit][best_fit].clone();
                let existing = arrows[id1][id2].clone();

                let in_label = match &in_arr.label {
                    Some(l) => l.clone(),
                    None => continue,
                };
                let out_label = match &out_arr.label {
                    Some(l) => l.clone(),
                    None => continue,
                };

                // Build first/second
                let (first_label, first_prec, second_label, second_prec) =
                    build_first_second(&in_label, in_arr.prec, &out_label, out_arr.prec, &self_arr);

                // bypass = first . second
                let bypass: Arrow = if first_label.is_empty() {
                    Arrow {
                        label: Some(second_label.clone()),
                        prec: second_prec,
                    }
                } else if second_label.is_empty() {
                    Arrow {
                        label: Some(first_label.clone()),
                        prec: first_prec,
                    }
                } else {
                    let mut s = String::new();
                    if first_prec < Prec::Concat {
                        s.push('(');
                    }
                    s.push_str(&first_label);
                    if first_prec < Prec::Concat {
                        s.push(')');
                    }
                    if second_prec < Prec::Concat {
                        s.push('(');
                    }
                    s.push_str(&second_label);
                    if second_prec < Prec::Concat {
                        s.push(')');
                    }
                    Arrow {
                        label: Some(s),
                        prec: Prec::Concat,
                    }
                };

                // merge with existing
                let merged: Arrow = match (&existing.label, &bypass.label) {
                    (_, None) => existing.clone(),
                    (None, Some(_)) => bypass.clone(),
                    (Some(e), Some(b)) => {
                        if e.is_empty() {
                            // ()|(bypass) = (bypass)?
                            let mut s = String::new();
                            if bypass.prec <= Prec::Quant {
                                s.push('(');
                            }
                            s.push_str(b);
                            if bypass.prec <= Prec::Quant {
                                s.push(')');
                            }
                            s.push('?');
                            Arrow {
                                label: Some(s),
                                prec: Prec::Quant,
                            }
                        } else {
                            // (existing)|(bypass)
                            let mut s = String::new();
                            s.push_str(e);
                            s.push('|');
                            s.push_str(b);
                            Arrow {
                                label: Some(s),
                                prec: Prec::Alt,
                            }
                        }
                    }
                };

                arrows[id1][id2] = merged;
            }
        }

        // remove all transitions through best_fit
        for id in 0..n {
            arrows[id][best_fit].label = None;
            arrows[best_fit][id].label = None;
        }
    }

    arrows[aux][aux].label.clone().unwrap_or_else(|| "[]".to_string())
}

fn build_first_second(
    in_label: &str,
    in_prec: Prec,
    out_label: &str,
    out_prec: Prec,
    self_arr: &Arrow,
) -> (String, Prec, String, Prec) {
    // (in)(self)*(out) reduction with various tweaks.
    let self_label_opt = self_arr.label.as_deref();
    let self_prec = self_arr.prec;

    if self_label_opt.is_none() || self_label_opt.unwrap().is_empty() {
        // (in)[]*(out) == (in)()*(out) == (in)(out)
        return (in_label.to_string(), in_prec, out_label.to_string(), out_prec);
    }

    let self_label = self_label_opt.unwrap();

    // Try in.label ends with self.label
    let try_in = || -> Option<(String, Prec)> {
        if in_prec < Prec::Concat || self_prec < Prec::Concat {
            return None;
        }
        let in_b = in_label.as_bytes();
        let self_b = self_label.as_bytes();
        if in_b.len() < self_b.len() {
            return None;
        }
        let diff = in_b.len() - self_b.len();
        if &in_b[diff..] != self_b {
            return None;
        }
        // hacky guards from C
        if diff >= 1 {
            let c = in_b[diff - 1];
            if c == b'^' || c == b'-' || c == b'\\' {
                if diff == 1 || in_b[diff - 2] != b'\\' {
                    return None;
                }
            }
        }
        if diff >= 2 {
            if &in_b[diff - 2..diff] == b"\\x" {
                if diff == 2 || in_b[diff - 3] != b'\\' {
                    return None;
                }
            }
        }
        if diff >= 3 {
            if &in_b[diff - 3..diff - 1] == b"\\x" {
                if diff == 3 || in_b[diff - 4] != b'\\' {
                    return None;
                }
            }
        }
        // first = in_pre + (self) + +
        let mut s = String::new();
        if diff != 0 && in_prec < Prec::Concat {
            s.push('(');
        }
        s.push_str(&in_label[..diff]);
        if diff != 0 && in_prec < Prec::Concat {
            s.push(')');
        }
        if self_prec <= Prec::Quant {
            s.push('(');
        }
        s.push_str(self_label);
        if self_prec <= Prec::Quant {
            s.push(')');
        }
        s.push('+');
        Some((s, Prec::Concat))
    };

    if let Some((fst, fst_prec)) = try_in() {
        return (fst, fst_prec, out_label.to_string(), out_prec);
    }

    // Try out.label starts with self.label
    if out_prec >= Prec::Concat && self_prec >= Prec::Concat {
        let out_b = out_label.as_bytes();
        let self_b = self_label.as_bytes();
        if out_b.len() >= self_b.len() && &out_b[..self_b.len()] == self_b {
            let diff = out_b.len() - self_b.len();
            // (in)(self)+(out_post)
            let mut s = String::new();
            if self_prec <= Prec::Quant {
                s.push('(');
            }
            s.push_str(self_label);
            if self_prec <= Prec::Quant {
                s.push(')');
            }
            s.push('+');
            if diff != 0 && out_prec < Prec::Concat {
                s.push('(');
            }
            // C: memcpy(p, out.label + diff, diff) -- wait that copies "diff" bytes from out.label+diff
            // That's bytes [diff .. 2*diff], which seems like a bug... let me re-check.
            // Actually wait, looking at C:
            //   second.label = malloc(strlen(out.label) + 5 + 1);
            //   ...
            //   memcpy(p, out.label + diff, diff), p += diff;
            // strlen(self_label) bytes were skipped at start of out.label, so out_post = out.label + strlen(self.label).
            // strlen(out.label) - strlen(self.label) = diff bytes remain.
            // So we should copy `diff` bytes starting at out.label + strlen(self.label), which equals out.label + (out.label.len - diff).
            // The C code says `memcpy(p, out.label + diff, diff)`. Hmm.
            // Wait, in C, diff = strlen(out.label) - strlen(self.label). If self_label_len = S, out_label_len = O, diff = O - S.
            // out_post should start at offset S = O - diff. The remainder is diff bytes.
            // C code uses `out.label + diff` which is offset diff = O - S. That doesn't match S = O - diff unless O - S = S, i.e., O = 2S.
            // Wait, this looks buggy. Let me re-read:
            //   diff = strlen(out.label) - strlen(self.label) = O - S
            //   out_post starts at offset S = O - diff
            // C: `memcpy(p, out.label + diff, diff)`
            // That's offset diff (= O-S), copy diff (= O-S) bytes.
            // So copies bytes [O-S, 2(O-S)]. This isn't right unless O - S + (O - S) = O, i.e., O = 2(O-S), 2S = O.
            // Hmm. Looking at the test case that exercises this... e.g., regex `aa*` decompile result.
            // Actually let me just match the C behavior verbatim — even if it's buggy, the tests may rely on it.
            let copy_start = diff;
            let copy_len = diff;
            let copy_end = (copy_start + copy_len).min(out_b.len());
            s.push_str(&out_label[copy_start..copy_end]);
            if diff != 0 && out_prec < Prec::Concat {
                s.push(')');
            }
            return (in_label.to_string(), in_prec, s, Prec::Concat);
        }
    }

    // (in)(self)*(out)
    let mut s = String::new();
    if self_prec <= Prec::Quant {
        s.push('(');
    }
    s.push_str(self_label);
    if self_prec <= Prec::Quant {
        s.push(')');
    }
    s.push('*');
    if out_prec < Prec::Concat {
        s.push('(');
    }
    s.push_str(out_label);
    if out_prec < Prec::Concat {
        s.push(')');
    }
    (in_label.to_string(), in_prec, s, Prec::Concat)
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
        match self.next() {
            Some(c) => Ok(c),
            None => Err("unexpected end of input".to_string()),
        }
    }
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

        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Merge state vectors: re states stay 0..re_len, alt states shifted by re_len
        let re_len = re.states.len();
        for s in alt.states.iter_mut() {
            shift_option(&mut s.target, re_len);
            shift_option(&mut s.epsilon0, re_len);
            shift_option(&mut s.epsilon1, re_len);
        }
        let alt_initial_new = alt.initial + re_len;
        let alt_final_new = alt.final_ + re_len;
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

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    match ctx.peek() {
        Some(b'*') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let f = atom.final_;
            let i = atom.initial;
            atom.states[f].epsilon1 = Some(i);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            let new_i = atom.initial;
            let new_f = atom.final_;
            atom.states[new_i].epsilon1 = Some(new_f);
            return Ok(atom);
        }
        Some(b'+') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let f = atom.final_;
            let i = atom.initial;
            atom.states[f].epsilon1 = Some(i);
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
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[init].epsilon1 = Some(fin);
            return Ok(atom);
        }
        Some(b'{') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let saved_pos = ctx.pos.saturating_sub(1); // position of '{'

            let min: u32 = match parse_natural(ctx) {
                Ok(n) => n,
                Err(e) => {
                    if e == "expected natural number" {
                        0
                    } else {
                        return Err(e);
                    }
                }
            };

            let mut max: u32 = min;
            let mut max_unbounded = false;
            if ctx.peek() == Some(b',') {
                ctx.next();
                match parse_natural(ctx) {
                    Ok(n) => max = n,
                    Err(e) => {
                        if e == "expected natural number" {
                            max_unbounded = true;
                        } else {
                            return Err(e);
                        }
                    }
                }
            }

            if ctx.peek() != Some(b'}') {
                return Err("expected '}'".to_string());
            }
            ctx.next();

            if min > max && !max_unbounded {
                ctx.pos = saved_pos;
                return Err("misbounded quantifier".to_string());
            }

            let mut atoms = Nfa::new_single();
            let total: u32 = if max_unbounded {
                min.saturating_add(1)
            } else {
                max
            };
            for i in 0..total {
                let mut clone = nfa_clone(&atom);
                if i >= min {
                    if max_unbounded {
                        let f = clone.final_;
                        let init = clone.initial;
                        clone.states[f].epsilon1 = Some(init);
                        nfa_pad_initial(&mut clone);
                        nfa_pad_final(&mut clone);
                    }
                    let new_i = clone.initial;
                    let new_f = clone.final_;
                    clone.states[new_i].epsilon1 = Some(new_f);
                }
                nfa_concat(&mut atoms, clone);
                if i == u32::MAX {
                    break;
                }
            }
            return Ok(atoms);
        }
        _ => {}
    }

    Ok(atom)
}

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

    let symset = parse_symset(ctx)?;
    let mut chars = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    chars.states[0].label = symset;
    chars.states[0].target = Some(1);
    Ok(chars)
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.next();
        complement = true;
    }

    let saved = ctx.pos;
    if let Ok(s) = try_parse_shorthand(ctx) {
        let mut s = s;
        if complement {
            s.invert();
        }
        return Ok(s);
    }
    ctx.pos = saved;

    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut symset = SymSet::empty();
        while ctx.peek() != Some(b']') {
            if ctx.is_eof() {
                return Err("expected ']'".to_string());
            }
            let sub = parse_symset(ctx)?;
            symset.union_with(&sub);
        }
        ctx.next(); // consume ']'
        if complement {
            symset.invert();
        }
        return Ok(symset);
    }

    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut symset = SymSet::full();
        while ctx.peek() != Some(b'>') {
            if ctx.is_eof() {
                return Err("expected '>'".to_string());
            }
            let sub = parse_symset(ctx)?;
            symset.intersect_with(&sub);
        }
        ctx.next(); // consume '>'
        if complement {
            symset.invert();
        }
        return Ok(symset);
    }

    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.next();
        end = parse_symbol(ctx)?;
    }
    let end_open: u8 = end.wrapping_add(1);
    let mut symset = SymSet::empty();
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

fn try_parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let saved = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        match ctx.next() {
            Some(b'd') => return Ok(digits_set()),
            Some(b'D') => return Ok(not_digits_set()),
            Some(b's') => return Ok(spaces_set()),
            Some(b'S') => return Ok(not_spaces_set()),
            Some(b'w') => return Ok(wordchar_set()),
            Some(b'W') => return Ok(not_wordchar_set()),
            _ => {
                ctx.pos = saved;
            }
        }
    }
    if ctx.peek() == Some(b'.') {
        ctx.next();
        let mut s = SymSet::full();
        // remove '\n'
        let n = b'\n';
        s.bits[(n / 8) as usize] &= !(1u8 << (n % 8));
        return Ok(s);
    }
    Err("expected shorthand class".to_string())
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = match ctx.peek() {
        None => return Err("expected symbol".to_string()),
        Some(c) => c,
    };
    if c == b'\\' {
        ctx.next();
        return parse_escape(ctx);
    }
    if METACHARS.contains(&c) {
        return Err("unexpected metacharacter".to_string());
    }
    if !is_print(c) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.next();
    Ok(c)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = match ctx.peek() {
        None => return Err("unknown escape".to_string()),
        Some(c) => c,
    };
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
        _ => {
            ctx.pos -= 1;
            Err("unknown escape".to_string())
        }
    }
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte: u8 = 0;
    for _ in 0..2 {
        byte = byte.wrapping_shl(4);
        let c = match ctx.peek() {
            None => return Err("expected hex digit".to_string()),
            Some(c) => c,
        };
        if c.is_ascii_digit() {
            byte |= c - b'0';
        } else if c.is_ascii_hexdigit() {
            byte |= c.to_ascii_lowercase() - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.next();
    }
    Ok(byte)
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
    for &c in &[b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
        s.insert(c);
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
    for c in b'a'..=b'z' {
        s.insert(c);
    }
    for c in b'A'..=b'Z' {
        s.insert(c);
    }
    for c in b'0'..=b'9' {
        s.insert(c);
    }
    s.insert(b'_');
    s
}

fn not_wordchar_set() -> SymSet {
    let mut s = wordchar_set();
    s.invert();
    s
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let first = ctx.peek();
    if !matches!(first, Some(c) if c.is_ascii_digit()) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        let digit = (c - b'0') as u32;
        natural = match natural.checked_mul(10).and_then(|n| n.checked_add(digit)) {
            Some(n) => n,
            None => return Err("natural number overflow".to_string()),
        };
        ctx.next();
    }
    Ok(natural)
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt {
        *v += offset;
    }
}
