const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn is_print(c: u8) -> bool {
    // C isprint: printable including space (0x20..=0x7e)
    c >= 0x20 && c <= 0x7e
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
    // C isspace: space, tab, LF, VT, FF, CR
    c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r'
}

fn to_lower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' { c + 32 } else { c }
}

fn to_upper(c: u8) -> u8 {
    if c >= b'a' && c <= b'z' { c - 32 } else { c }
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

pub fn symset_fmt(set: &SymSet) -> String {
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;
    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    fn emit(target: &mut Vec<u8>, c: u8) {
        let is_meta = is_metachar(c);
        if !is_print(c) && !is_meta {
            target.extend_from_slice(format!("\\x{:02x}", c).as_bytes());
        } else {
            if is_meta {
                target.push(b'\\');
            }
            target.push(c);
        }
    }

    let mut chr: usize = 0;
    while chr < 256 {
        let c = chr as u8;
        let in_set = set.contains(c);
        if in_set { nsym += 1; } else { nnsym += 1; }
        if in_set { emit(&mut buf, c); } else { emit(&mut nbuf, c); }

        // run extension
        let start = chr;
        while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
            chr += 1;
        }
        if chr - start >= 2 {
            let end_in_set = set.contains(chr as u8);
            if end_in_set { buf.push(b'-'); nsym -= 1; }
            else { nbuf.push(b'-'); nnsym -= 1; }
        }
        if chr - start >= 1 {
            // emit end-of-run character
            let c2 = chr as u8;
            let in_set2 = set.contains(c2);
            if in_set2 { nsym += 1; } else { nnsym += 1; }
            if in_set2 { emit(&mut buf, c2); } else { emit(&mut nbuf, c2); }
        }
        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        let s = &buf[1..buf.len() - 1];
        return String::from_utf8_lossy(s).into_owned();
    } else if nnsym == 1 {
        // Original: nbuf[1] = '^', then strip trailing ']' and return from idx 1
        let mut nb = nbuf.clone();
        nb[1] = b'^';
        let trimmed = &nb[1..nb.len() - 1];
        return String::from_utf8_lossy(trimmed).into_owned();
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
        // Single state at index 0; initial == final == 0
        let mut states = Vec::new();
        states.push(NState::new());
        Nfa { states, initial: 0, final_: 0, complemented: false }
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

pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    // Match C semantics:
    //   if nfa1.initial == nfa1.final: replace nfa1 with nfa2
    //   else if nfa2.initial != nfa2.final: merge contents of nfa2.initial into
    //     nfa1.final and set nfa1.final = remapped(nfa2.final)
    if nfa1.initial == nfa1.final_ {
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        // Nothing to do
        return;
    }
    let final_idx_old = nfa1.final_;
    let nfa2_initial = nfa2.initial;
    // Build a mapping: nfa2 index -> nfa1 index
    let mut mapping = vec![0usize; nfa2.states.len()];
    mapping[nfa2_initial] = final_idx_old;
    for i in 0..nfa2.states.len() {
        if i == nfa2_initial { continue; }
        mapping[i] = nfa1.states.len();
        nfa1.states.push(NState::new()); // placeholder
    }
    // Now properly populate them with remapped references
    for i in 0..nfa2.states.len() {
        let new_idx = mapping[i];
        let mut s = nfa2.states[i].clone();
        if let Some(v) = s.target { s.target = Some(mapping[v]); }
        if let Some(v) = s.epsilon0 { s.epsilon0 = Some(mapping[v]); }
        if let Some(v) = s.epsilon1 { s.epsilon1 = Some(mapping[v]); }
        nfa1.states[new_idx] = s;
    }
    nfa1.final_ = mapping[nfa2.final_];
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let mut new_state = NState::new();
    new_state.epsilon0 = Some(nfa.initial);
    let new_idx = nfa.states.len();
    nfa.states.push(new_state);
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
    let cloned = nfa.clone();
    let dfa = ltre_compile(cloned);
    let uncomplemented = ltre_uncompile(&dfa);
    *nfa = uncomplemented;
    Ok(())
}

pub fn nfa_dump(_nfa: &Nfa) {
    // Stub: implementation not required by tests
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
            .field("bitset_len", &self.bitset.len())
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

fn make_dstate(bitset_size: usize) -> DState {
    DState {
        transitions: [0usize; 256],
        accepting: false,
        terminating: false,
        bitset: vec![0u8; bitset_size],
    }
}

fn leb128_put(buf: &mut Vec<u8>, mut n: i32) {
    let mut v = n as u32;
    while (v >> 7) != 0 {
        buf.push(((v & 0x7f) as u8) | 0x80);
        v >>= 7;
    }
    buf.push(v as u8);
    let _ = n; // silence
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: u32 = 0;
    let mut c = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128: out of bounds".to_string());
        }
        let byte = buf[*p];
        n |= ((byte & 0x7f) as u32) << (c * 7);
        c += 1;
        *p += 1;
        if byte & 0x80 == 0 {
            break;
        }
    }
    Ok(n as i32)
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    leb128_put(&mut buf, dfa.states.len() as i32);
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
    let mut states: Vec<DState> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        states.push(make_dstate(0));
    }
    for id in 0..dfa_size {
        if p >= buf.len() {
            return Err("deserialize: out of bounds".to_string());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;
        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("deserialize: out of bounds".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            // assign target to chr..=chr+len, then advance
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

pub fn dfa_dump(_dfa: &Dfa) {
    // Stub: not exercised by tests
}

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    if bitset_test(bitset, st_id) {
        return;
    }
    bitset_set(bitset, st_id);
    let s = &nfa.states[st_id];
    let e0 = s.epsilon0;
    let e1 = s.epsilon1;
    if let Some(idx) = e0 {
        epsilon_closure_into(nfa, idx, bitset);
    }
    if let Some(idx) = e1 {
        epsilon_closure_into(nfa, idx, bitset);
    }
}

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let bitset_size = (nfa_size + 7) / 8;
    let mut bitset = vec![0u8; bitset_size];
    epsilon_closure_into(nfa, start, &mut bitset);
    bitset
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;
    let mut out = vec![0u8; bitset_size];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) && nfa.states[id].label.contains(chr) {
            if let Some(target) = nfa.states[id].target {
                epsilon_closure_into(nfa, target, &mut out);
            }
        }
    }
    out
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    if idx / 8 >= bs.len() { return false; }
    (bs[idx / 8] & (1u8 << (idx % 8))) != 0
}
fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx / 8] |= 1u8 << (idx % 8);
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    // Used in older designs; not strictly needed but keep functional
    for (i, s) in states.iter().enumerate() {
        if !s.accepting && s.transitions.iter().all(|&t| t == i) {
            return i;
        }
    }
    let i = states.len();
    let mut d = make_dstate(0);
    for t in d.transitions.iter_mut() { *t = i; }
    states.push(d);
    i
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;

    // Powerset construction: start from epsilon-closure of initial
    let mut states: Vec<DState> = Vec::new();
    let init_bitset = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);

    // helper to find an existing state with the same bitset
    fn find_or_create(states: &mut Vec<DState>, bitset: Vec<u8>, nfa: &Nfa) -> usize {
        for (i, s) in states.iter().enumerate() {
            if s.bitset == bitset {
                return i;
            }
        }
        let mut d = make_dstate(0);
        d.bitset = bitset.clone();
        d.accepting = bitset_test(&bitset, nfa.final_);
        if nfa.complemented {
            d.accepting = !d.accepting;
        }
        let i = states.len();
        states.push(d);
        i
    }

    let _ = bitset_size;
    let initial_idx = find_or_create(&mut states, init_bitset, &nfa);

    // Process states one by one; new states are appended
    let mut idx = 0;
    while idx < states.len() {
        for chr in 0..=255u32 {
            let chr8 = chr as u8;
            let cur_bitset = states[idx].bitset.clone();
            let new_bitset = step_powerset(&nfa, &cur_bitset, chr8);
            let target_idx = find_or_create(&mut states, new_bitset, &nfa);
            states[idx].transitions[chr as usize] = target_idx;
        }
        idx += 1;
    }

    let mut dfa = Dfa { states, initial: initial_idx };
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 { return; }
    // Build distinguishability matrix
    let mut dis = vec![vec![false; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                dis[i][j] = true;
                dis[j][i] = true;
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            for j in (i + 1)..n {
                if dis[i][j] { continue; }
                for chr in 0..256usize {
                    let t1 = dfa.states[i].transitions[chr];
                    let t2 = dfa.states[j].transitions[chr];
                    if t1 != t2 && dis[t1][t2] {
                        dis[i][j] = true;
                        dis[j][i] = true;
                        changed = true;
                        break;
                    }
                }
            }
        }
    }
    // Group equivalent states. Compute class id for each state.
    let mut class_of: Vec<usize> = vec![usize::MAX; n];
    let mut representatives: Vec<usize> = Vec::new();
    for i in 0..n {
        if class_of[i] != usize::MAX { continue; }
        let class_id = representatives.len();
        representatives.push(i);
        class_of[i] = class_id;
        for j in (i + 1)..n {
            if class_of[j] == usize::MAX && !dis[i][j] {
                class_of[j] = class_id;
            }
        }
    }
    // Build new minimized DFA
    let mut new_states: Vec<DState> = Vec::with_capacity(representatives.len());
    for &rep in &representatives {
        let mut d = make_dstate(0);
        d.accepting = dfa.states[rep].accepting;
        for chr in 0..256 {
            d.transitions[chr] = class_of[dfa.states[rep].transitions[chr]];
        }
        new_states.push(d);
    }
    // Compute terminating: a state is terminating iff all its transitions go to itself
    for (i, s) in new_states.iter_mut().enumerate() {
        s.terminating = s.transitions.iter().all(|&t| t == i);
    }
    let new_initial = class_of[dfa.initial];
    // The initial state must be at index 0 in our representation, since the C
    // version uses linked list head as initial. But our Dfa has explicit `initial`
    // field, so that's fine.
    dfa.states = new_states;
    dfa.initial = new_initial;
}

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut idx = dfa.initial;
    for &b in input {
        if dfa.states[idx].terminating { break; }
        idx = dfa.states[idx].transitions[b as usize];
    }
    dfa.states[idx].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    // Simpler: compile if not present
    if dfap.is_none() {
        *dfap = Some(ltre_compile(nfa.clone()));
    }
    ltre_matches(dfap.as_ref().unwrap(), input)
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    // Mirror C: create initial NFA state, final state, and one NFA state per DFA state.
    // Use binary trees of epsilon transitions to dispatch the multi-target labeled
    // transitions of each DFA state.
    let dfa_size = dfa.states.len();
    let mut nfa = Nfa { states: Vec::new(), initial: 0, final_: 0, complemented: false };
    // initial
    nfa.states.push(NState::new());
    let nfa_initial = 0usize;
    // final
    nfa.states.push(NState::new());
    let nfa_final = 1usize;
    // one NFA state per DFA state
    let mut nstates: Vec<usize> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        nstates.push(nfa.states.len());
        nfa.states.push(NState::new());
    }
    // Initial -> nstates[dfa.initial] via epsilon1
    nfa.states[nfa_initial].epsilon1 = Some(nstates[dfa.initial]);
    // Accepting DFA states: nstates[id].epsilon1 = nfa_final
    for id in 0..dfa_size {
        if dfa.states[id].accepting {
            nfa.states[nstates[id]].epsilon1 = Some(nfa_final);
        }
    }

    // For each DFA state, build the binary tree of epsilon transitions to handle
    // multiple labeled transitions
    for id1 in 0..dfa_size {
        // Collect (id2, transitions_set) for each ds2 with non-empty transitions
        let mut entries: Vec<(usize, SymSet)> = Vec::new();
        for id2 in 0..dfa_size {
            let mut trans = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[id1].transitions[chr] == id2 {
                    trans.insert(chr as u8);
                    empty = false;
                }
            }
            if !empty {
                entries.push((id2, trans));
            }
        }
        // Build binary tree
        let mut free_idx: Option<usize> = None;
        for (id2, trans) in entries {
            let src = if free_idx.is_none() {
                let s = nstates[id1];
                free_idx = Some(s);
                s
            } else {
                let new_state_idx = nfa.states.len();
                nfa.states.push(NState::new());
                let f = free_idx.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_state_idx);
                } else {
                    nfa.states[f].epsilon0 = Some(new_state_idx);
                    free_idx = Some(new_state_idx);
                }
                new_state_idx
            };
            nfa.states[src].target = Some(nstates[id2]);
            nfa.states[src].label = trans;
        }
    }

    nfa.initial = nfa_initial;
    nfa.final_ = nfa_final;
    nfa
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    // Use the state-elimination approach. We work with `arrows[i][j]` storing
    // a label string (None means no transition; Some("") means epsilon).
    #[derive(Clone, Copy, PartialEq, PartialOrd)]
    enum Prec { Alt, Concat, Quant, Symset }

    #[derive(Clone)]
    struct Arrow {
        label: Option<String>,
        prec: Prec,
    }

    let dfa_size = dfa.states.len();
    let aux = dfa_size;
    let n = dfa_size + 1;
    let mut arrows: Vec<Vec<Arrow>> = vec![vec![Arrow { label: None, prec: Prec::Symset }; n]; n];

    // epsilon from aux to dfa.initial
    arrows[aux][dfa.initial] = Arrow { label: Some(String::new()), prec: Prec::Symset };

    for id1 in 0..dfa_size {
        if dfa.states[id1].accepting {
            arrows[id1][aux] = Arrow { label: Some(String::new()), prec: Prec::Symset };
        }
        for id2 in 0..dfa_size {
            let mut trans = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[id1].transitions[chr] == id2 {
                    trans.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            let s = symset_fmt(&trans);
            arrows[id1][id2] = Arrow { label: Some(s), prec: Prec::Symset };
        }
    }

    // State elimination
    loop {
        // Find best fit: minimum non-zero in/out degree (excluding aux)
        let mut best_fit: Option<usize> = None;
        let mut min_degree = i32::MAX;
        for id1 in 0..dfa_size {
            let mut degree = 0;
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
            Some(b) => b,
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

                if in_arrow.label.is_none() || out_arrow.label.is_none() {
                    continue;
                }

                let in_label = in_arrow.label.unwrap();
                let out_label = out_arrow.label.unwrap();
                let self_label_opt = self_arrow.label.clone();

                // Compute first/second
                let (first_label, first_prec, second_label, second_prec) = if self_label_opt.is_none() || self_label_opt.as_ref().unwrap().is_empty() {
                    (in_label.clone(), in_arrow.prec, out_label.clone(), out_arrow.prec)
                } else {
                    let self_label = self_label_opt.unwrap();
                    // try in-suffix first
                    let mut handled = false;
                    let mut fl = String::new(); let mut fp = Prec::Concat;
                    let mut sl = String::new(); let mut sp = Prec::Concat;
                    if in_arrow.prec >= Prec::Concat && self_arrow.prec >= Prec::Concat
                        && in_label.len() >= self_label.len()
                        && in_label.ends_with(&self_label) {
                        let diff = in_label.len() - self_label.len();
                        let in_bytes = in_label.as_bytes();
                        // hacky checks
                        let mut nevermind = false;
                        if diff >= 1 && (in_bytes[diff - 1] == b'^' || in_bytes[diff - 1] == b'-' || in_bytes[diff - 1] == b'\\')
                            && (diff == 1 || in_bytes[diff - 2] != b'\\') {
                            nevermind = true;
                        }
                        if !nevermind && diff >= 2 && &in_bytes[diff - 2..diff] == b"\\x"
                            && (diff == 2 || in_bytes[diff - 3] != b'\\') {
                            nevermind = true;
                        }
                        if !nevermind && diff >= 3 && &in_bytes[diff - 3..diff - 1] == b"\\x"
                            && (diff == 3 || in_bytes[diff - 4] != b'\\') {
                            nevermind = true;
                        }
                        if !nevermind {
                            // in_pre + self+ + out
                            let in_pre = &in_label[..diff];
                            let mut p = String::new();
                            if diff != 0 && in_arrow.prec < Prec::Concat { p.push('('); }
                            p.push_str(in_pre);
                            if diff != 0 && in_arrow.prec < Prec::Concat { p.push(')'); }
                            if self_arrow.prec <= Prec::Quant { p.push('('); }
                            p.push_str(&self_label);
                            if self_arrow.prec <= Prec::Quant { p.push(')'); }
                            p.push('+');
                            fl = p; fp = Prec::Concat;
                            sl = out_label.clone(); sp = out_arrow.prec;
                            handled = true;
                        }
                    }
                    if !handled {
                        if out_arrow.prec >= Prec::Concat && self_arrow.prec >= Prec::Concat
                            && out_label.len() >= self_label.len()
                            && out_label.starts_with(&self_label) {
                            let diff = out_label.len() - self_label.len();
                            // self+ + out_post
                            let out_post = &out_label[self_label.len()..];
                            let mut p = String::new();
                            if self_arrow.prec <= Prec::Quant { p.push('('); }
                            p.push_str(&self_label);
                            if self_arrow.prec <= Prec::Quant { p.push(')'); }
                            p.push('+');
                            if diff != 0 && out_arrow.prec < Prec::Concat { p.push('('); }
                            p.push_str(out_post);
                            if diff != 0 && out_arrow.prec < Prec::Concat { p.push(')'); }
                            sl = p; sp = Prec::Concat;
                            fl = in_label.clone(); fp = in_arrow.prec;
                        } else {
                            // (in)(self)*(out)
                            let mut p = String::new();
                            if self_arrow.prec <= Prec::Quant { p.push('('); }
                            p.push_str(&self_label);
                            if self_arrow.prec <= Prec::Quant { p.push(')'); }
                            p.push('*');
                            if out_arrow.prec < Prec::Concat { p.push('('); }
                            p.push_str(&out_label);
                            if out_arrow.prec < Prec::Concat { p.push(')'); }
                            sl = p; sp = Prec::Concat;
                            fl = in_label.clone(); fp = in_arrow.prec;
                        }
                    }
                    (fl, fp, sl, sp)
                };

                // Concatenate first and second to make bypass
                let (bypass_label, bypass_prec) = if first_label.is_empty() {
                    (Some(second_label.clone()), second_prec)
                } else if second_label.is_empty() {
                    (Some(first_label.clone()), first_prec)
                } else {
                    let mut p = String::new();
                    if first_prec < Prec::Concat { p.push('('); }
                    p.push_str(&first_label);
                    if first_prec < Prec::Concat { p.push(')'); }
                    if second_prec < Prec::Concat { p.push('('); }
                    p.push_str(&second_label);
                    if second_prec < Prec::Concat { p.push(')'); }
                    (Some(p), Prec::Concat)
                };

                // Merge with existing
                let merged: Arrow = match (existing.label.clone(), bypass_label.clone()) {
                    (existing_lab, None) => Arrow { label: existing_lab, prec: existing.prec },
                    (None, Some(blab)) => Arrow { label: Some(blab), prec: bypass_prec },
                    (Some(elab), Some(blab)) => {
                        if elab.is_empty() {
                            // ()|(bypass) => (bypass)?
                            let mut p = String::new();
                            if bypass_prec <= Prec::Quant { p.push('('); }
                            p.push_str(&blab);
                            if bypass_prec <= Prec::Quant { p.push(')'); }
                            p.push('?');
                            Arrow { label: Some(p), prec: Prec::Quant }
                        } else {
                            let mut p = elab.clone();
                            p.push('|');
                            p.push_str(&blab);
                            Arrow { label: Some(p), prec: Prec::Alt }
                        }
                    }
                };

                arrows[id1][id2] = merged;
            }
        }
        // Eliminate best_fit by zeroing all incident arrows
        for id in 0..n {
            arrows[id][best_fit] = Arrow { label: None, prec: Prec::Symset };
            arrows[best_fit][id] = Arrow { label: None, prec: Prec::Symset };
        }
    }

    arrows[aux][aux].label.clone().unwrap_or_else(|| "[]".to_string())
}

// Parsing

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
        match self.next() {
            Some(c) => Ok(c),
            None => Err("unexpected EOF".to_string()),
        }
    }
}

fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in b'0'..=b'9' { s.insert(c); }
    s
}
fn not_digits_set() -> SymSet {
    let mut s = digits_set();
    s.invert();
    s
}
fn spaces_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0u32..256u32 {
        if is_space(c as u8) { s.insert(c as u8); }
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
    for c in 0u32..256u32 {
        let cc = c as u8;
        if cc == b'_' || is_alnum(cc) { s.insert(cc); }
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

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let c = ctx.peek().ok_or_else(|| "expected natural number".to_string())?;
    if !is_digit(c) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !is_digit(c) { break; }
        let digit = (c - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            // Consume remaining digits to advance pos? In C, the loop continues
            // consuming digits; we should match that semantics for error reporting.
            // We return UINT_MAX-ish via Err; signal overflow distinctly.
            return Err("natural number overflow".to_string());
        }
        natural = natural * 10 + digit;
        ctx.next();
    }
    Ok(natural)
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
            return Err("expected hex digit".to_string());
        }
        ctx.next();
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or_else(|| "unexpected EOF in escape".to_string())?;
    if is_metachar(c) {
        ctx.next();
        return Ok(c);
    }
    let c = ctx.next().unwrap();
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
    let c = ctx.peek().ok_or_else(|| "expected symbol".to_string())?;
    if is_metachar(c) {
        return Err("unexpected metacharacter".to_string());
    }
    if !is_print(c) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.next();
    Ok(c)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    if ctx.peek() == Some(b'\\') {
        let saved = ctx.pos;
        ctx.next();
        if let Some(c) = ctx.next() {
            match c {
                b'd' => return Ok(digits_set()),
                b'D' => return Ok(not_digits_set()),
                b's' => return Ok(spaces_set()),
                b'S' => return Ok(not_spaces_set()),
                b'w' => return Ok(wordchar_set()),
                b'W' => return Ok(not_wordchar_set()),
                _ => {}
            }
        }
        ctx.pos = saved;
    }
    if ctx.peek() == Some(b'.') {
        ctx.next();
        let mut s = SymSet::full();
        // exclude '\n'
        let mut excl = SymSet::empty();
        excl.insert(b'\n');
        excl.invert();
        s.intersect_with(&excl);
        return Ok(s);
    }
    Err("expected shorthand class".to_string())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.next();
        complement = true;
    }
    let last_pos = ctx.pos;

    // Try shorthand
    match parse_shorthand(ctx) {
        Ok(mut s) => {
            if complement { s.invert(); }
            return Ok(s);
        }
        Err(_) => { ctx.pos = last_pos; }
    }

    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut symset = SymSet::empty();
        while ctx.peek().map_or(false, |c| c != b']') {
            let sub = parse_symset(ctx)?;
            symset.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.next();
        if complement { symset.invert(); }
        return Ok(symset);
    }

    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut symset = SymSet::full();
        while ctx.peek().map_or(false, |c| c != b'>') {
            let sub = parse_symset(ctx)?;
            symset.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.next();
        if complement { symset.invert(); }
        return Ok(symset);
    }

    // Symbol or symbol-range
    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.next();
        end = parse_symbol(ctx)?;
    }
    let mut symset = SymSet::empty();
    let begin_u = begin as u32;
    let end_u = end as u32;
    // Open upper bound: end++; then iterate begin..end (mod 256)
    let mut chr = begin_u;
    let stop = (end_u + 1) & 0xff;
    loop {
        symset.insert(chr as u8);
        chr = (chr + 1) & 0xff;
        if chr == stop { break; }
    }
    if complement { symset.invert(); }
    Ok(symset)
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
    // Build a 2-state NFA for it
    let mut nfa = Nfa { states: Vec::new(), initial: 0, final_: 1, complemented: false };
    nfa.states.push(NState::new());
    nfa.states.push(NState::new());
    nfa.states[0].label = symset;
    nfa.states[0].target = Some(1);
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;
    match ctx.peek() {
        Some(b'*') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            // atom.final.epsilon1 = atom.initial
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            // initial.epsilon1 = final
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
            let last_pos = ctx.pos;
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            // parse min
            let mut min: u32 = 0;
            let mut min_specified = true;
            match parse_natural(ctx) {
                Ok(v) => min = v,
                Err(e) => {
                    if e == "natural number overflow" {
                        return Err(e);
                    }
                    min = 0;
                    min_specified = false;
                    let _ = min_specified;
                }
            }
            let mut max = min;
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
                return Err("expected '}'".to_string());
            }
            ctx.next();
            if min > max && !max_unbounded {
                ctx.pos = last_pos;
                return Err("misbounded quantifier".to_string());
            }
            // Build atoms: a fresh single-state NFA, then concat clones
            let mut atoms = Nfa::new_single();
            let mut i: u32 = 0;
            loop {
                let cond = if max_unbounded { i <= min } else { i < max };
                if !cond { break; }
                let mut clone = nfa_clone(&atom);
                if i >= min {
                    if max_unbounded {
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
        if c == b')' || c == b'|' || c == b'&' { break; }
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
        if c != b'|' && c != b'&' { break; }
        let intersect = c == b'&';
        ctx.next();
        let mut alt = parse_term(ctx)?;
        // De Morgan: a&b == ~(~a|~b). Toggle complemented flags.
        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;
        // Build alternation:
        //   pad_initial(re), pad_final(alt)
        //   re.initial.epsilon1 = alt.initial
        //   re.final.epsilon0 = alt.final
        //   re.final = alt.final
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);
        // We need to combine alt's states into re's states
        // Use nfa_concat-like logic but customize.
        let alt_initial_old = alt.initial;
        let alt_final_old = alt.final_;
        // Mapping: alt index -> re index
        let mut mapping = vec![0usize; alt.states.len()];
        for i in 0..alt.states.len() {
            mapping[i] = re.states.len() + i;
        }
        // Push placeholders
        for _ in 0..alt.states.len() {
            re.states.push(NState::new());
        }
        // Copy states with remapped references
        for i in 0..alt.states.len() {
            let new_idx = mapping[i];
            let mut s = alt.states[i].clone();
            if let Some(v) = s.target { s.target = Some(mapping[v]); }
            if let Some(v) = s.epsilon0 { s.epsilon0 = Some(mapping[v]); }
            if let Some(v) = s.epsilon1 { s.epsilon1 = Some(mapping[v]); }
            re.states[new_idx] = s;
        }
        // Now re.initial gets epsilon1 -> alt.initial (mapped)
        let re_initial = re.initial;
        let re_final = re.final_;
        re.states[re_initial].epsilon1 = Some(mapping[alt_initial_old]);
        re.states[re_final].epsilon0 = Some(mapping[alt_final_old]);
        re.final_ = mapping[alt_final_old];

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
    let bytes = s.as_bytes();
    let mut nfa = Nfa { states: Vec::new(), initial: 0, final_: 0, complemented: false };
    nfa.states.push(NState::new());
    for &b in bytes {
        let prev_final = nfa.final_;
        let new_idx = nfa.states.len();
        nfa.states.push(NState::new());
        nfa.states[prev_final].target = Some(new_idx);
        nfa.states[prev_final].label.insert(b);
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
    for s in nfa.states.iter_mut() {
        let mut new_label = s.label;
        for chr in 0u32..256u32 {
            let c = chr as u8;
            if s.label.contains(c) {
                new_label.insert(to_lower(c));
                new_label.insert(to_upper(c));
            }
        }
        s.label = new_label;
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}
