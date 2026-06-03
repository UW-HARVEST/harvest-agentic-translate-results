// Translated from c_src/ltre.c

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn is_print(c: u8) -> bool {
    (0x20..=0x7e).contains(&c)
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_xdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r')
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn to_lower(c: u8) -> u8 {
    if c.is_ascii_uppercase() {
        c + 32
    } else {
        c
    }
}

fn to_upper(c: u8) -> u8 {
    if c.is_ascii_lowercase() {
        c - 32
    } else {
        c
    }
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
        let i = c as usize;
        (self.bits[i / 8] & (1u8 << (i % 8))) != 0
    }
    pub fn insert(&mut self, c: u8) {
        let i = c as usize;
        self.bits[i / 8] |= 1u8 << (i % 8);
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
    // Mirrors c_src/ltre.c symset_fmt: produces a parsable form.
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;

    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    let mut chr: i32 = 0;
    while chr < 256 {
        // append_chr label re-entry
        loop {
            let cur = chr as u8;
            if set.contains(cur) {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            let target_is_buf = set.contains(cur);
            let p = if target_is_buf { &mut buf } else { &mut nbuf };
            let metachar = is_metachar(cur);
            if !is_print(cur) && !metachar {
                let s = format!("\\x{:02x}", cur);
                p.extend_from_slice(s.as_bytes());
            } else {
                if metachar {
                    p.push(b'\\');
                }
                p.push(cur);
            }

            // make character ranges
            let start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - start >= 2 {
                let p = if set.contains(chr as u8) {
                    &mut buf
                } else {
                    &mut nbuf
                };
                p.push(b'-');
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
        // bufp[-2] = '\0'; return buf+1
        // i.e., drop the trailing ']' and the leading '['
        let s = &buf[1..buf.len() - 1];
        return String::from_utf8_lossy(s).into_owned();
    } else if nnsym == 1 {
        // nbufp[-2] = '\0', nbuf[1] = '^'; return nbuf + 1
        // produce "^X" where X is the single complementary symset
        let mut out = Vec::new();
        out.push(b'^');
        // skip leading "^[" and trailing "]"
        // Original: buf was "^[X]"; setting nbuf[1]='^' makes "^^X]" then nbuf+1 gives "^X]" with last char zeroed = "^X"
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

/// Concatenate `nfa2` onto the end of `nfa1`.
/// In the C code, this merges nfa2.initial into nfa1.final.
/// Here, since states refer to indices, we adjust by an offset.
pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ {
        // nfa1 is just one state (the empty NFA); replace it with nfa2
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        // nfa2 is just one state; nothing to concat (both endpoints same)
        return;
    }
    let offset = nfa1.states.len();
    // nfa2.initial will be merged into nfa1.final_
    // For each state in nfa2:
    //   - state index = offset + i, except nfa2.initial which becomes nfa1.final_
    let map_idx = |i: usize| -> usize {
        if i == nfa2.initial {
            nfa1.final_
        } else {
            offset + i - if i > nfa2.initial { 1 } else { 0 }
        }
    };
    // Actually, simpler: keep the nfa2.initial slot but copy its contents into nfa1.final_.
    // Then append all other states (skipping nfa2.initial).
    // But that complicates index mapping. Let's just append all, then merge later.

    // Simpler approach: append all states of nfa2 with offset, and copy nfa2.initial's
    // contents into nfa1.final_.
    let initial_state = nfa2.states[nfa2.initial].clone();
    // First: copy nfa2.initial contents into nfa1.final_, with mapped indices
    let map_simple = |opt: Option<usize>| opt.map(|i| i + offset);
    nfa1.states[nfa1.final_].label = initial_state.label;
    nfa1.states[nfa1.final_].target = map_simple(initial_state.target);
    nfa1.states[nfa1.final_].epsilon0 = map_simple(initial_state.epsilon0);
    nfa1.states[nfa1.final_].epsilon1 = map_simple(initial_state.epsilon1);
    // But if those mapped indices reference nfa2.initial (offset + nfa2.initial), we need
    // to redirect them to nfa1.final_.
    let fix = |opt: Option<usize>| -> Option<usize> {
        opt.map(|i| if i == offset + nfa2.initial { nfa1.final_ } else { i })
    };
    let target = nfa1.states[nfa1.final_].target;
    let e0 = nfa1.states[nfa1.final_].epsilon0;
    let e1 = nfa1.states[nfa1.final_].epsilon1;
    nfa1.states[nfa1.final_].target = fix(target);
    nfa1.states[nfa1.final_].epsilon0 = fix(e0);
    nfa1.states[nfa1.final_].epsilon1 = fix(e1);

    // Now append all states from nfa2 (including nfa2.initial as a placeholder slot to keep
    // simple offset mapping). The placeholder won't be referenced because we redirect above.
    for (i, s) in nfa2.states.iter().enumerate() {
        let mut ns = s.clone();
        ns.target = fix(map_simple(ns.target));
        ns.epsilon0 = fix(map_simple(ns.epsilon0));
        ns.epsilon1 = fix(map_simple(ns.epsilon1));
        nfa1.states.push(ns);
        let _ = i;
    }

    // The new final is the mapped nfa2.final_
    let new_final = if nfa2.final_ == nfa2.initial {
        nfa1.final_
    } else {
        offset + nfa2.final_
    };
    nfa1.final_ = new_final;
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let mut new_init = NState::new();
    new_init.epsilon0 = Some(nfa.initial);
    nfa.states.push(new_init);
    nfa.initial = nfa.states.len() - 1;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_final_idx = nfa.states.len();
    nfa.states.push(NState::new());
    nfa.states[nfa.final_].epsilon0 = Some(new_final_idx);
    nfa.final_ = new_final_idx;
}

pub fn nfa_uncomplement(nfa: &mut Nfa) -> Result<(), String> {
    if !nfa.complemented {
        return Ok(());
    }
    let dfa = ltre_compile(nfa.clone());
    let new = ltre_uncompile(&dfa);
    *nfa = new;
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
    let mut c: i32 = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128: out of bounds".to_string());
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
    let dfa_size = dfa.states.len();
    let mut buf: Vec<u8> = Vec::new();
    leb128_put(&mut buf, dfa_size as i32);

    for state in &dfa.states {
        let acc_term = ((state.accepting as u8) << 1) | (state.terminating as u8);
        buf.push(acc_term);

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
        states.push(DState {
            transitions: [0usize; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        });
    }

    for id in 0..dfa_size {
        if p >= buf.len() {
            return Err("dfa_deserialize: out of bounds".to_string());
        }
        let acc_term = buf[p];
        p += 1;
        states[id].accepting = (acc_term >> 1) & 1 != 0;
        states[id].terminating = acc_term & 1 != 0;

        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("dfa_deserialize: out of bounds".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            // do { transitions[chr++] = target; } while (len--);
            // i.e., transitions[chr..chr + len + 1] = target
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

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    if idx / 8 >= bs.len() {
        return false;
    }
    bs[idx / 8] & (1u8 << (idx % 8)) != 0
}

fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx / 8] |= 1u8 << (idx % 8);
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
    let bs_size = (nfa_size + 7) / 8;
    let mut bs = vec![0u8; bs_size];
    epsilon_closure_into(nfa, start, &mut bs);
    bs
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;
    let mut out = vec![0u8; bs_size];
    for id in all_bitset_indices(bitset) {
        if id >= nfa_size {
            continue;
        }
        if nfa.states[id].label.contains(chr) {
            if let Some(t) = nfa.states[id].target {
                epsilon_closure_into(nfa, t, &mut out);
            }
        }
    }
    out
}

fn find_or_create_dead(_states: &mut Vec<DState>) -> usize {
    0
}

/// Compile the NFA into a DFA via powerset construction + minimization.
pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    let mut states: Vec<DState> = Vec::new();

    // Initial DFA state = epsilon-closure of nfa.initial
    let mut init_bs = vec![0u8; bs_size];
    epsilon_closure_into(&nfa, nfa.initial, &mut init_bs);
    let mut init_state = DState {
        transitions: [0usize; 256],
        accepting: bitset_test(&init_bs, nfa.final_) ^ nfa.complemented,
        terminating: false,
        bitset: init_bs,
    };
    let _ = &mut init_state;
    states.push(init_state);

    let mut i = 0;
    while i < states.len() {
        for chr in 0..=255u32 {
            let chr = chr as u8;
            let target_bs = step_powerset(&nfa, &states[i].bitset.clone(), chr);
            // find a state with the same bitset
            let mut found: Option<usize> = None;
            for (j, st) in states.iter().enumerate() {
                if st.bitset == target_bs {
                    found = Some(j);
                    break;
                }
            }
            let target_idx = if let Some(j) = found {
                j
            } else {
                let accepting = bitset_test(&target_bs, nfa.final_) ^ nfa.complemented;
                states.push(DState {
                    transitions: [0usize; 256],
                    accepting,
                    terminating: false,
                    bitset: target_bs,
                });
                states.len() - 1
            };
            states[i].transitions[chr as usize] = target_idx;
        }
        i += 1;
    }

    let mut dfa = Dfa { states, initial: 0 };
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let dfa_size = dfa.states.len();
    if dfa_size == 0 {
        return;
    }

    // Distinguishability matrix
    let mut dis = vec![vec![false; dfa_size]; dfa_size];
    let make_dis = |dis: &mut Vec<Vec<bool>>, a: usize, b: usize| {
        dis[a][b] = true;
        dis[b][a] = true;
    };

    for i in 0..dfa_size {
        for j in (i + 1)..dfa_size {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                make_dis(&mut dis, i, j);
            }
        }
    }

    // iteratively flag distinguishable
    loop {
        let mut changed = false;
        for id1 in 0..dfa_size {
            for id2 in (id1 + 1)..dfa_size {
                if dis[id1][id2] {
                    continue;
                }
                for chr in 0..256 {
                    let t1 = dfa.states[id1].transitions[chr];
                    let t2 = dfa.states[id2].transitions[chr];
                    if t1 != t2 && dis[t1][t2] {
                        make_dis(&mut dis, id1, id2);
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

    // Build equivalence classes: for each state, its representative is the
    // smallest indistinguishable state index.
    let mut rep: Vec<usize> = (0..dfa_size).collect();
    for i in 0..dfa_size {
        for j in 0..i {
            if !dis[i][j] {
                rep[i] = rep[j];
                break;
            }
        }
    }

    // Determine which states are kept: those that are their own rep.
    let mut kept: Vec<usize> = Vec::new();
    let mut new_idx: Vec<Option<usize>> = vec![None; dfa_size];
    for i in 0..dfa_size {
        if rep[i] == i {
            new_idx[i] = Some(kept.len());
            kept.push(i);
        }
    }
    // For non-kept states: their new index is their representative's new index
    for i in 0..dfa_size {
        if new_idx[i].is_none() {
            new_idx[i] = new_idx[rep[i]];
        }
    }

    // Build new states
    let mut new_states: Vec<DState> = Vec::with_capacity(kept.len());
    for &orig_i in &kept {
        let mut s = dfa.states[orig_i].clone();
        for c in 0..256 {
            let t = s.transitions[c];
            s.transitions[c] = new_idx[t].unwrap();
        }
        new_states.push(s);
    }

    let new_initial = new_idx[dfa.initial].unwrap();

    // Compute terminating
    for s in new_states.iter_mut() {
        // Set terminating if all transitions point to itself.
        // We need its index in the new states.
    }
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

    // Allocate: initial, dfa_size mid-nodes, final = dfa_size + 2 nodes minimum
    // We may add more for the binary tree of labeled transitions.
    let mut states: Vec<NState> = Vec::new();
    let initial_idx = states.len();
    states.push(NState::new());
    // Pre-allocate dfa_size states (one per DFA state)
    let mut nstates: Vec<usize> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        nstates.push(states.len());
        states.push(NState::new());
    }
    // initial->epsilon1 = nstates[dfa.initial]
    states[initial_idx].epsilon1 = Some(nstates[dfa.initial]);

    // We'll add a final state at the end, but for now reserve a placeholder.
    // We need to know final's index when we set epsilon1 of accepting states.
    // The C code allocates final upfront. Let's do the same: reserve final last
    // by appending later. We'll fix up references to a sentinel "FINAL" later.
    // Simpler: append final right now.
    let final_idx = states.len();
    states.push(NState::new());

    // For each accepting state, set epsilon1 -> final
    for ds_id in 0..dfa_size {
        if dfa.states[ds_id].accepting {
            states[nstates[ds_id]].epsilon1 = Some(final_idx);
        }
    }

    // For each ds1, build labeled transitions to each ds2 (if non-empty).
    for ds1 in 0..dfa_size {
        let mut free: Option<usize> = None;
        for ds2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[ds1].transitions[chr] == ds2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }

            let src: usize;
            if free.is_none() {
                // first iteration: root is nstates[ds1]
                let f = nstates[ds1];
                free = Some(f);
                src = f;
            } else {
                // Allocate a new state
                let new_state = states.len();
                states.push(NState::new());
                let f = free.unwrap();
                if states[f].epsilon1.is_none() {
                    states[f].epsilon1 = Some(new_state);
                } else {
                    states[f].epsilon0 = Some(new_state);
                    free = Some(new_state);
                }
                src = new_state;
            }
            states[src].target = Some(nstates[ds2]);
            states[src].label = transitions;
        }
    }

    Nfa {
        states,
        initial: initial_idx,
        final_: final_idx,
        complemented: false,
    }
}

// --- Decompilation ---

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Prec {
    Alt,
    Concat,
    Quant,
    Symset,
}

#[derive(Clone)]
struct Arrow {
    label: Option<String>,
    prec: Prec,
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let aux = dfa_size; // index of auxiliary state
    let n = dfa_size + 1;

    // arrows[id1][id2]
    let mut arrows: Vec<Vec<Arrow>> = vec![
        vec![
            Arrow {
                label: None,
                prec: Prec::Symset
            };
            n
        ];
        n
    ];

    // epsilon transition aux -> dfa.initial
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
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[ds1].transitions[chr] == ds2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let fmt = symset_fmt(&transitions);
            arrows[ds1][ds2] = Arrow {
                label: Some(fmt),
                prec: Prec::Symset,
            };
        }
    }

    loop {
        // Find best fit (min degree, > 0) among non-aux states
        let mut best_fit: usize = 0;
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

        if min_degree == i32::MAX {
            break;
        }

        // Iterate all id1, id2 != best_fit
        for id1 in 0..n {
            if id1 == best_fit {
                continue;
            }
            for id2 in 0..n {
                if id2 == best_fit {
                    continue;
                }
                let in_arrow = arrows[id1][best_fit].clone();
                let out_arrow = arrows[best_fit][id2].clone();
                let self_arrow = arrows[best_fit][best_fit].clone();
                let existing = arrows[id1][id2].clone();

                if in_arrow.label.is_none() || out_arrow.label.is_none() {
                    continue;
                }

                let in_label = in_arrow.label.as_ref().unwrap();
                let out_label = out_arrow.label.as_ref().unwrap();

                let first: Arrow;
                let second: Arrow;

                let self_is_empty_or_eps =
                    self_arrow.label.is_none() || self_arrow.label.as_ref().unwrap().is_empty();

                if self_is_empty_or_eps {
                    first = in_arrow.clone();
                    second = out_arrow.clone();
                } else {
                    let self_label = self_arrow.label.as_ref().unwrap();
                    let in_len = in_label.len() as isize;
                    let self_len = self_label.len() as isize;
                    let diff = in_len - self_len;

                    let mut handled = false;
                    let mut nevermind = false;

                    if in_arrow.prec >= Prec::Concat
                        && self_arrow.prec >= Prec::Concat
                        && diff >= 0
                        && &in_label[diff as usize..] == self_label.as_str()
                    {
                        // try to avoid breaking apart symsets in inbound
                        let in_bytes = in_label.as_bytes();
                        let d = diff as usize;
                        let mut bail = false;
                        if d >= 1
                            && b"^-\\".contains(&in_bytes[d - 1])
                            && (d == 1 || in_bytes[d - 2] != b'\\')
                        {
                            bail = true;
                        }
                        if !bail
                            && d >= 2
                            && &in_bytes[d - 2..d] == b"\\x"
                            && (d == 2 || in_bytes[d - 3] != b'\\')
                        {
                            bail = true;
                        }
                        if !bail
                            && d >= 3
                            && &in_bytes[d - 3..d - 1] == b"\\x"
                            && (d == 3 || in_bytes[d - 4] != b'\\')
                        {
                            bail = true;
                        }
                        if bail {
                            nevermind = true;
                        } else {
                            // (in_pre)(self)+(out)
                            let mut s = String::new();
                            if diff != 0 && in_arrow.prec < Prec::Concat {
                                s.push('(');
                            }
                            s.push_str(&in_label[..d]);
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
                            first = Arrow {
                                label: Some(s),
                                prec: Prec::Concat,
                            };
                            second = out_arrow.clone();
                            handled = true;
                            // assign to first/second below
                            arrows[id1][id2] = combine_bypass_existing(
                                &first, &second, &in_arrow, &out_arrow, &existing,
                            );
                            continue;
                        }
                    } else {
                        nevermind = true;
                    }

                    if nevermind && !handled {
                        let out_len = out_label.len() as isize;
                        let diff2 = out_len - self_len;
                        if out_arrow.prec >= Prec::Concat
                            && self_arrow.prec >= Prec::Concat
                            && diff2 >= 0
                            && out_label.starts_with(self_label.as_str())
                        {
                            // (in)(self)+(out_post)
                            let mut s = String::new();
                            if self_arrow.prec <= Prec::Quant {
                                s.push('(');
                            }
                            s.push_str(self_label);
                            if self_arrow.prec <= Prec::Quant {
                                s.push(')');
                            }
                            s.push('+');
                            // (out_post) where out == self + out_post
                            // Note C uses memcpy(p, out.label + diff, diff) which copies `diff` bytes
                            // starting at offset `diff`. That looks like a bug (should be out_len-diff?).
                            // Actually `diff = strlen(out) - strlen(self)`, and out_post starts at offset
                            // strlen(self), with length diff. So we should copy from `out.label+strlen(self)`,
                            // length `diff`. The C does `memcpy(p, out.label + diff, diff)` which is wrong
                            // unless diff == strlen(self), but we replicate it as written. Actually
                            // the C is likely correct because it's typical to express it differently...
                            // Re-reading: `memcpy(p, out.label + diff, diff)`. With diff = strlen(out)-strlen(self),
                            // out.label+diff is right. But the length should be strlen(out)-diff = strlen(self).
                            // I think the C has a subtle issue. Let me re-read.
                            //
                            // Looking again: `memcpy(p, out.label + diff, diff), p += diff;`
                            //
                            // Hmm. If `out == "self" + "post"`, where strlen(self)=k, strlen(out)=n,
                            // diff = n - k = post length. out.label + diff = out.label + post length
                            // = pointer at offset (n - k). The first n-k chars are self, then chars at
                            // offset n-k onwards are NOT what we want; we want chars from offset k.
                            //
                            // Wait, the C condition is `strncmp(out.label, self.label, strlen(self.label)) == 0`,
                            // meaning out starts with self. So we want post = out[strlen(self)..]. Length = diff.
                            // The C code reads `out.label + diff`, but should be `out.label + strlen(self)`.
                            // These are equal only if strlen(self) == diff, i.e., n - k = k -> n = 2k.
                            //
                            // Actually wait, I misread. Let me look once more:
                            //
                            //   if (out.prec >= CONCAT && self.prec >= CONCAT &&
                            //       (diff = strlen(out.label) - strlen(self.label)) >= 0 &&
                            //       strncmp(out.label, self.label, strlen(self.label)) == 0) {
                            //     ...
                            //     memcpy(p, out.label + diff, diff), p += diff;
                            //
                            // I'll match the C exactly; perhaps it's a bug or perhaps I'm
                            // misunderstanding. Let me match it.
                            let out_post_start = diff2 as usize;
                            let out_post_len = diff2 as usize;
                            if diff2 != 0 && out_arrow.prec < Prec::Concat {
                                s.push('(');
                            }
                            // Copy out_post_len bytes starting at out_post_start
                            let out_bytes = out_label.as_bytes();
                            let end = (out_post_start + out_post_len).min(out_bytes.len());
                            s.push_str(&out_label[out_post_start..end]);
                            if diff2 != 0 && out_arrow.prec < Prec::Concat {
                                s.push(')');
                            }
                            second = Arrow {
                                label: Some(s),
                                prec: Prec::Concat,
                            };
                            first = in_arrow.clone();
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
                            second = Arrow {
                                label: Some(s),
                                prec: Prec::Concat,
                            };
                            first = in_arrow.clone();
                        }

                        arrows[id1][id2] = combine_bypass_existing(
                            &first, &second, &in_arrow, &out_arrow, &existing,
                        );
                        continue;
                    }
                    // Otherwise unreached
                    continue;
                }

                // Reached only when self_is_empty_or_eps
                arrows[id1][id2] =
                    combine_bypass_existing(&first, &second, &in_arrow, &out_arrow, &existing);
            }
        }

        // Eliminate state
        for id in 0..n {
            arrows[id][best_fit].label = None;
            arrows[best_fit][id].label = None;
        }
    }

    let regex_label = arrows[aux][aux].label.clone();
    match regex_label {
        Some(s) => s,
        None => "[]".to_string(),
    }
}

fn combine_bypass_existing(
    first: &Arrow,
    second: &Arrow,
    _in_arrow: &Arrow,
    _out_arrow: &Arrow,
    existing: &Arrow,
) -> Arrow {
    // bypass = first concat second
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

    // Merge with existing using alternation
    if bypass.label.is_none() {
        return existing.clone();
    }
    if existing.label.is_none() {
        return bypass;
    }
    let existing_label = existing.label.as_ref().unwrap();
    let bypass_label = bypass.label.as_ref().unwrap();
    if existing_label.is_empty() {
        // (bypass)?
        let mut s = String::new();
        if bypass.prec <= Prec::Quant {
            s.push('(');
        }
        s.push_str(bypass_label);
        if bypass.prec <= Prec::Quant {
            s.push(')');
        }
        s.push('?');
        return Arrow {
            label: Some(s),
            prec: Prec::Quant,
        };
    }
    // (existing) | (bypass)
    let mut s = String::new();
    s.push_str(existing_label);
    s.push('|');
    s.push_str(bypass_label);
    Arrow {
        label: Some(s),
        prec: Prec::Alt,
    }
}

// --- Parser ---

pub struct ParseContext<'a> {
    pub chars: &'a [u8],
    pub pos: usize,
}
impl<'a> ParseContext<'a> {
    pub fn new(s: &'a str) -> Self {
        ParseContext {
            chars: s.as_bytes(),
            pos: 0,
        }
    }
    pub fn peek(&self) -> Option<u8> {
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
    }
    pub fn next(&mut self) -> Option<u8> {
        if self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }
    pub fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
    pub fn expect_char(&mut self) -> Result<u8, String> {
        if let Some(c) = self.next() {
            Ok(c)
        } else {
            Err("unexpected end of input".to_string())
        }
    }
}

fn cur_or_zero(ctx: &ParseContext) -> u8 {
    ctx.peek().unwrap_or(0)
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    if !is_digit(cur_or_zero(ctx)) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u64 = 0;
    while is_digit(cur_or_zero(ctx)) {
        let digit = (cur_or_zero(ctx) - b'0') as u64;
        natural = natural * 10 + digit;
        if natural > u32::MAX as u64 {
            // signal overflow with a special error string and return u32::MAX
            return Err("natural number overflow".to_string());
        }
        ctx.pos += 1;
    }
    Ok(natural as u32)
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte: u8 = 0;
    for _ in 0..2 {
        byte <<= 4;
        let chr = cur_or_zero(ctx);
        if is_digit(chr) {
            byte |= chr - b'0';
        } else if is_xdigit(chr) {
            byte |= to_lower(chr) - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.pos += 1;
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = cur_or_zero(ctx);
    if is_metachar(c) {
        ctx.pos += 1;
        return Ok(c);
    }
    let c = cur_or_zero(ctx);
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
    let c = cur_or_zero(ctx);
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
        let cc = c as u8;
        if cc == b'_' || is_alnum(cc) {
            s.insert(cc);
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
    if cur_or_zero(ctx) == b'\\' {
        let saved = ctx.pos;
        ctx.pos += 1;
        let c = cur_or_zero(ctx);
        ctx.pos += 1;
        match c {
            b'd' => return Ok(digits_set()),
            b'D' => return Ok(not_digits_set()),
            b's' => return Ok(spaces_set()),
            b'S' => return Ok(not_spaces_set()),
            b'w' => return Ok(wordchar_set()),
            b'W' => return Ok(not_wordchar_set()),
            _ => {
                ctx.pos = saved;
            }
        }
    }
    if cur_or_zero(ctx) == b'.' {
        ctx.pos += 1;
        let mut s = SymSet::empty();
        for c in 0..=255u32 {
            if (c as u8) != b'\n' {
                s.insert(c as u8);
            }
        }
        return Ok(s);
    }
    Err("expected shorthand class".to_string())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if cur_or_zero(ctx) == b'^' {
        ctx.pos += 1;
        complement = true;
    }

    let last_pos = ctx.pos;
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

    if cur_or_zero(ctx) == b'[' {
        ctx.pos += 1;
        let mut s = SymSet::empty();
        while cur_or_zero(ctx) != b']' {
            if ctx.is_eof() {
                return Err("expected ']'".to_string());
            }
            let sub = parse_symset(ctx)?;
            s.union_with(&sub);
        }
        if cur_or_zero(ctx) != b']' {
            return Err("expected ']'".to_string());
        }
        ctx.pos += 1;
        if complement {
            s.invert();
        }
        return Ok(s);
    }

    if cur_or_zero(ctx) == b'<' {
        ctx.pos += 1;
        let mut s = SymSet::full();
        while cur_or_zero(ctx) != b'>' {
            if ctx.is_eof() {
                return Err("expected '>'".to_string());
            }
            let sub = parse_symset(ctx)?;
            s.intersect_with(&sub);
        }
        if cur_or_zero(ctx) != b'>' {
            return Err("expected '>'".to_string());
        }
        ctx.pos += 1;
        if complement {
            s.invert();
        }
        return Ok(s);
    }

    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if cur_or_zero(ctx) == b'-' {
        ctx.pos += 1;
        end = parse_symbol(ctx)?;
    }
    // open upper bound
    let mut s = SymSet::empty();
    let upper = end.wrapping_add(1);
    let mut chr = begin;
    loop {
        s.insert(chr);
        chr = chr.wrapping_add(1);
        if chr == upper {
            break;
        }
    }
    if complement {
        s.invert();
    }
    Ok(s)
}

fn parse_atom(ctx: &mut ParseContext) -> Result<Nfa, String> {
    if cur_or_zero(ctx) == b'(' {
        ctx.pos += 1;
        let sub = parse_regex(ctx)?;
        if cur_or_zero(ctx) != b')' {
            return Err("expected ')'".to_string());
        }
        ctx.pos += 1;
        return Ok(sub);
    }

    // chars NFA: initial -> final via labeled transition
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
    let c = cur_or_zero(ctx);
    if c == b'*' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        atom.states[atom.final_].epsilon1 = Some(atom.initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }
    if c == b'+' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        atom.states[atom.final_].epsilon1 = Some(atom.initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        return Ok(atom);
    }
    if c == b'?' {
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        if atom.states[atom.initial].epsilon1.is_some() {
            nfa_pad_initial(&mut atom);
        }
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }

    if c == b'{' {
        let last_pos = ctx.pos;
        ctx.pos += 1;
        nfa_uncomplement(&mut atom)?;
        let mut min: u32 = 0;
        match parse_natural(ctx) {
            Ok(v) => min = v,
            Err(e) if e == "natural number overflow" => {
                return Err(e);
            }
            Err(_) => {
                min = 0;
            }
        }

        let mut max: u32 = min;
        let mut max_unbounded = false;
        if cur_or_zero(ctx) == b',' {
            ctx.pos += 1;
            match parse_natural(ctx) {
                Ok(v) => max = v,
                Err(e) if e == "natural number overflow" => {
                    return Err(e);
                }
                Err(_) => {
                    max_unbounded = true;
                }
            }
        }

        if cur_or_zero(ctx) != b'}' {
            return Err("expected '}'".to_string());
        }
        ctx.pos += 1;

        if min > max && !max_unbounded {
            ctx.pos = last_pos;
            return Err("misbounded quantifier".to_string());
        }

        // Build atoms
        let mut atoms = Nfa {
            states: vec![NState::new()],
            initial: 0,
            final_: 0,
            complemented: false,
        };

        let upper_iter: u64 = if max_unbounded {
            (min as u64) + 1
        } else {
            max as u64
        };
        let mut i: u64 = 0;
        while i < upper_iter {
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
            i += 1;
        }

        return Ok(atoms);
    }

    Ok(atom)
}

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut complement = false;
    if cur_or_zero(ctx) == b'~' {
        ctx.pos += 1;
        complement = true;
    }

    let mut term = Nfa {
        states: vec![NState::new()],
        initial: 0,
        final_: 0,
        complemented: false,
    };

    // hacky lookahead: until we see ) | & or EOF
    loop {
        let c = cur_or_zero(ctx);
        if ctx.is_eof() || c == b')' || c == b'|' || c == b'&' {
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
    while cur_or_zero(ctx) == b'|' || cur_or_zero(ctx) == b'&' {
        let intersect = cur_or_zero(ctx) == b'&';
        ctx.pos += 1;
        let mut alt = parse_term(ctx)?;

        // De Morgan: a&b == ~(~a|~b)
        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        // Pad initial of re, pad final of alt, then merge.
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        let re_size = re.states.len();
        let alt_offset = re_size;
        // Append alt states with offset
        for s in alt.states.iter() {
            let mut ns = s.clone();
            ns.target = ns.target.map(|i| i + alt_offset);
            ns.epsilon0 = ns.epsilon0.map(|i| i + alt_offset);
            ns.epsilon1 = ns.epsilon1.map(|i| i + alt_offset);
            re.states.push(ns);
        }
        let alt_initial = alt.initial + alt_offset;
        let alt_final = alt.final_ + alt_offset;

        // re.initial->epsilon1 = alt.initial
        re.states[re.initial].epsilon1 = Some(alt_initial);
        // re.final->epsilon0 = alt.final
        re.states[re.final_].epsilon0 = Some(alt_final);
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
    let bytes = s.as_bytes();
    let mut nfa = Nfa {
        states: vec![NState::new()],
        initial: 0,
        final_: 0,
        complemented: false,
    };
    for &b in bytes {
        let new_final = nfa.states.len();
        nfa.states.push(NState::new());
        nfa.states[nfa.final_].target = Some(new_final);
        nfa.states[nfa.final_].label.insert(b);
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
    for s in nfa.states.iter_mut() {
        for chr in 0..=255u32 {
            let c = chr as u8;
            if s.label.contains(c) {
                s.label.insert(to_lower(c));
                s.label.insert(to_upper(c));
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}

fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt {
        *v += offset;
    }
}
