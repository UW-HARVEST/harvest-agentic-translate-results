use std::collections::{BTreeMap, HashMap};

const METACHARS: &[u8] = br"\.-^$*+?{}[]<>()|&~";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymSet {
    bits: [u8; 256 / 8],
}

impl SymSet {
    pub fn empty() -> Self {
        Self { bits: [0; 32] }
    }

    pub fn full() -> Self {
        Self { bits: [0xff; 32] }
    }

    pub fn contains(&self, c: u8) -> bool {
        self.bits[c as usize / 8] & (1 << (c % 8)) != 0
    }

    pub fn insert(&mut self, c: u8) {
        self.bits[c as usize / 8] |= 1 << (c % 8);
    }

    pub fn invert(&mut self) {
        for byte in &mut self.bits {
            *byte = !*byte;
        }
    }

    pub fn union_with(&mut self, other: &SymSet) {
        for (lhs, rhs) in self.bits.iter_mut().zip(other.bits.iter()) {
            *lhs |= *rhs;
        }
    }

    pub fn intersect_with(&mut self, other: &SymSet) {
        for (lhs, rhs) in self.bits.iter_mut().zip(other.bits.iter()) {
            *lhs &= *rhs;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }
}

pub fn symset_fmt(set: &SymSet) -> String {
    let mut buf = String::from("[");
    let mut nbuf = String::from("^[");
    let mut nsym = 0usize;
    let mut nnsym = 0usize;

    let mut chr = 0u16;
    while chr <= 255 {
        let start = chr as u8;
        let present = set.contains(start);
        if present {
            nsym += 1;
        } else {
            nnsym += 1;
        }

        let target = if present { &mut buf } else { &mut nbuf };
        append_sym(target, start);

        while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
            chr += 1;
        }

        let end = chr as u8;
        if end.wrapping_sub(start) >= 2 {
            target.push('-');
            if present {
                nsym -= 1;
            } else {
                nnsym -= 1;
            }
        }
        if end != start {
            if present {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            append_sym(target, end);
        }

        chr += 1;
    }

    buf.push(']');
    nbuf.push(']');

    if nnsym == 0 {
        return "<>".to_string();
    }
    if nsym == 1 {
        return buf[1..buf.len() - 1].to_string();
    }
    if nnsym == 1 {
        let inner = &nbuf[2..nbuf.len() - 1];
        return format!("^{}", inner);
    }

    if buf.len() < nbuf.len() {
        buf
    } else {
        nbuf
    }
}

fn append_sym(out: &mut String, chr: u8) {
    let is_metachar = chr != 0 && METACHARS.contains(&chr);
    let is_print = chr.is_ascii_graphic() || chr == b' ';
    if !is_print && !is_metachar {
        out.push_str(&format!("\\x{chr:02x}"));
    } else {
        if is_metachar {
            out.push('\\');
        }
        out.push(chr as char);
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
        Self {
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
        Self {
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

    let base_len = nfa1.states.len();
    let nfa2_initial = nfa2.initial;
    let merge_idx = nfa1.final_;

    nfa1.states[merge_idx] = remap_nstate(&nfa2.states[nfa2_initial], nfa2_initial, base_len, merge_idx);

    for (old_idx, state) in nfa2.states.iter().enumerate() {
        if old_idx == nfa2_initial {
            continue;
        }
        nfa1.states
            .push(remap_nstate(state, nfa2_initial, base_len, merge_idx));
    }

    nfa1.final_ = remap_index(nfa2.final_, nfa2_initial, base_len, merge_idx);
}

fn remap_nstate(state: &NState, old_initial: usize, base_len: usize, merge_idx: usize) -> NState {
    let mut out = state.clone();
    out.target = out
        .target
        .map(|idx| remap_index(idx, old_initial, base_len, merge_idx));
    out.epsilon0 = out
        .epsilon0
        .map(|idx| remap_index(idx, old_initial, base_len, merge_idx));
    out.epsilon1 = out
        .epsilon1
        .map(|idx| remap_index(idx, old_initial, base_len, merge_idx));
    out
}

fn remap_index(idx: usize, old_initial: usize, base_len: usize, merge_idx: usize) -> usize {
    if idx == old_initial {
        merge_idx
    } else {
        base_len + idx - 1
    }
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let idx = nfa.states.len();
    let mut state = NState::new();
    state.epsilon0 = Some(nfa.initial);
    nfa.states.push(state);
    nfa.initial = idx;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let idx = nfa.states.len();
    nfa.states[nfa.final_].epsilon0 = Some(idx);
    nfa.states.push(NState::new());
    nfa.final_ = idx;
}

pub fn nfa_uncomplement(nfa: &mut Nfa) -> Result<(), String> {
    if !nfa.complemented {
        return Ok(());
    }
    let dfa = ltre_compile(nfa.clone());
    *nfa = ltre_uncompile(&dfa);
    Ok(())
}

pub fn nfa_dump(nfa: &Nfa) {
    println!("graph LR");
    println!("  I( ) --> {}", nfa.initial);
    println!("  {} --> F( )", nfa.final_);
    for (id, state) in nfa.states.iter().enumerate() {
        if let Some(next) = state.epsilon0 {
            println!("  {} --> {}", id, next);
        }
        if let Some(next) = state.epsilon1 {
            println!("  {} --> {}", id, next);
        }
        if !state.label.is_empty() {
            println!("  {} --{}--> {}", id, symset_fmt(&state.label), state.target.unwrap_or(id));
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
            .field("bitset", &self.bitset)
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
        Self {
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
        buf.push(((state.accepting as u8) << 1) | state.terminating as u8);
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            let target = state.transitions[chr];
            while chr + 1 < 256 && state.transitions[chr + 1] == target {
                chr += 1;
            }
            buf.push((chr - start) as u8);
            leb128_put(&mut buf, target as i32);
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
            transitions: [0; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        });
    }

    for id in 0..dfa_size {
        let flags = *buf.get(p).ok_or_else(|| "short buffer".to_string())?;
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;

        let mut chr = 0usize;
        while chr < 256 {
            let len = *buf.get(p).ok_or_else(|| "short buffer".to_string())? as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            if target >= dfa_size {
                return Err("invalid transition target".to_string());
            }
            for slot in &mut states[id].transitions[chr..=chr + len] {
                *slot = target;
            }
            chr += len + 1;
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
            let mut set = SymSet::empty();
            for chr in 0..=255u8 {
                if ds1.transitions[chr as usize] == id2 {
                    set.insert(chr);
                }
            }
            if !set.is_empty() {
                println!("  {} --{}--> {}", id1, symset_fmt(&set), id2);
            }
        }
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
    let mut n = 0i32;
    let mut c = 0;
    loop {
        let byte = *buf.get(*p).ok_or_else(|| "short buffer".to_string())?;
        *p += 1;
        n |= ((byte & 0x7f) as i32) << (c * 7);
        c += 1;
        if byte & 0x80 == 0 {
            return Ok(n);
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
    for byte in s.bytes() {
        let initial = nfa.final_;
        let next = nfa.states.len();
        let mut state = NState::new();
        state.label.insert(byte);
        state.target = Some(next);
        nfa.states[initial] = state;
        nfa.states.push(NState::new());
        nfa.final_ = next;
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
    for state in &mut nfa.states {
        let mut extra = Vec::new();
        for chr in 0..=255u8 {
            if state.label.contains(chr) {
                extra.push(chr.to_ascii_lowercase());
                extra.push(chr.to_ascii_uppercase());
            }
        }
        for chr in extra {
            state.label.insert(chr);
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.len();
    let initial_bitset = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let mut dfa = Dfa::new();
    dfa.states.push(DState {
        transitions: [0; 256],
        accepting: bitset_test(&initial_bitset, nfa.final_) ^ nfa.complemented,
        terminating: false,
        bitset: initial_bitset,
    });

    let mut idx = 0usize;
    while idx < dfa.states.len() {
        for chr in 0..=255u8 {
            let next = step_powerset(&nfa, &dfa.states[idx].bitset, chr);
            let target = if let Some(existing) = dfa.states.iter().position(|s| s.bitset == next) {
                existing
            } else {
                let accepting = bitset_test(&next, nfa.final_) ^ nfa.complemented;
                dfa.states.push(DState {
                    transitions: [0; 256],
                    accepting,
                    terminating: false,
                    bitset: next,
                });
                dfa.states.len() - 1
            };
            dfa.states[idx].transitions[chr as usize] = target;
        }
        idx += 1;
    }

    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    if let Some(idx) = states
        .iter()
        .position(|s| !s.accepting && s.transitions.iter().enumerate().all(|(i, &t)| i == t))
    {
        return idx;
    }
    let idx = states.len();
    states.push(DState {
        transitions: [idx; 256],
        accepting: false,
        terminating: true,
        bitset: Vec::new(),
    });
    idx
}

fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let mut out = vec![0u8; (nfa.len() + 7) / 8];
    for id in all_bitset_indices(bitset) {
        if id < nfa.len() && nfa.states[id].label.contains(chr) {
            if let Some(target) = nfa.states[id].target {
                epsilon_closure_into(nfa, target, &mut out);
            }
        }
    }
    out
}

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let mut out = vec![0u8; (nfa_size + 7) / 8];
    epsilon_closure_into(nfa, start, &mut out);
    out
}

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    if bitset_test(bitset, st_id) {
        return;
    }
    bitset_set(bitset, st_id);
    if let Some(next) = nfa.states[st_id].epsilon0 {
        epsilon_closure_into(nfa, next, bitset);
    }
    if let Some(next) = nfa.states[st_id].epsilon1 {
        epsilon_closure_into(nfa, next, bitset);
    }
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }

    let mut dis = vec![vec![false; n]; n];
    for i in 0..n {
        for j in i + 1..n {
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
            for j in i + 1..n {
                if dis[i][j] {
                    continue;
                }
                for chr in 0..256 {
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

    let mut rep = vec![0usize; n];
    for i in 0..n {
        rep[i] = i;
        for j in 0..i {
            if !dis[i][j] {
                rep[i] = rep[j];
                break;
            }
        }
    }

    let mut rep_to_new = BTreeMap::new();
    let mut new_states = Vec::new();
    for (old, &root) in rep.iter().enumerate() {
        if root == old {
            rep_to_new.insert(old, new_states.len());
            new_states.push(dfa.states[old].clone());
        }
    }

    for state in &mut new_states {
        for chr in 0..256 {
            let old_target = state.transitions[chr];
            let root = rep[old_target];
            state.transitions[chr] = rep_to_new[&root];
        }
        state.terminating = state.transitions.iter().all(|&t| t == 0) || false;
    }

    for (idx, state) in new_states.iter_mut().enumerate() {
        state.terminating = state.transitions.iter().all(|&t| t == idx);
    }

    dfa.initial = rep_to_new[&rep[dfa.initial]];
    dfa.states = new_states;
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

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut state = dfa.initial;
    let mut i = 0usize;
    while i < input.len() && !dfa.states[state].terminating {
        state = dfa.states[state].transitions[input[i] as usize];
        i += 1;
    }
    dfa.states[state].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    if dfap.is_none() {
        *dfap = Some(ltre_compile(nfa.clone()));
    }
    ltre_matches(dfap.as_ref().unwrap(), input)
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };

    let mut nstates = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        let idx = nfa.states.len();
        nfa.states.push(NState::new());
        nstates.push(idx);
    }

    nfa.states[nfa.initial].epsilon1 = Some(nstates[dfa.initial]);
    for (id, dstate) in dfa.states.iter().enumerate() {
        if dstate.accepting {
            nfa.states[nstates[id]].epsilon1 = Some(nfa.final_);
        }
    }

    for (ds1, state1) in dfa.states.iter().enumerate() {
        let mut free: Option<usize> = None;
        for ds2 in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            for chr in 0..=255u8 {
                if state1.transitions[chr as usize] == ds2 {
                    transitions.insert(chr);
                }
            }
            if transitions.is_empty() {
                continue;
            }

            let src = if free.is_none() {
                let root = nstates[ds1];
                free = Some(root);
                root
            } else {
                let idx = nfa.states.len();
                nfa.states.push(NState::new());
                let free_idx = free.unwrap();
                if nfa.states[free_idx].epsilon1.is_none() {
                    nfa.states[free_idx].epsilon1 = Some(idx);
                } else {
                    nfa.states[free_idx].epsilon0 = Some(idx);
                    free = Some(idx);
                }
                idx
            };

            nfa.states[src].target = Some(nstates[ds2]);
            nfa.states[src].label = transitions;
        }
    }

    nfa
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
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

    let dfa_size = dfa.states.len();
    let mut arrows = vec![
        vec![
            Arrow {
                label: None,
                prec: Prec::Symset,
            };
            dfa_size + 1
        ];
        dfa_size + 1
    ];

    arrows[dfa_size][dfa.initial].label = Some(String::new());
    arrows[dfa_size][dfa.initial].prec = Prec::Symset;

    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            arrows[id1][dfa_size].label = Some(String::new());
            arrows[id1][dfa_size].prec = Prec::Symset;
        }
        for id2 in 0..dfa.states.len() {
            let mut set = SymSet::empty();
            for chr in 0..=255u8 {
                if ds1.transitions[chr as usize] == id2 {
                    set.insert(chr);
                }
            }
            if !set.is_empty() {
                arrows[id1][id2].label = Some(symset_fmt(&set));
                arrows[id1][id2].prec = Prec::Symset;
            }
        }
    }

    loop {
        let mut best_fit = None;
        let mut min_degree = usize::MAX;
        for id1 in 0..dfa_size {
            let mut degree = 0usize;
            for id2 in 0..dfa_size {
                degree += usize::from(arrows[id1][id2].label.is_some());
                degree += usize::from(arrows[id2][id1].label.is_some());
            }
            if degree == 0 {
                continue;
            }
            if degree < min_degree {
                min_degree = degree;
                best_fit = Some(id1);
            }
        }

        let Some(best_fit) = best_fit else {
            break;
        };

        for id1 in 0..=dfa_size {
            if id1 == best_fit {
                continue;
            }
            for id2 in 0..=dfa_size {
                if id2 == best_fit {
                    continue;
                }
                let inbound = arrows[id1][best_fit].clone();
                let outbound = arrows[best_fit][id2].clone();
                let self_loop = arrows[best_fit][best_fit].clone();
                let existing = arrows[id1][id2].clone();

                let (Some(in_label), Some(out_label)) =
                    (inbound.label.clone(), outbound.label.clone())
                else {
                    continue;
                };

                let (first_label, first_prec, second_label, second_prec) =
                    if self_loop.label.as_deref().is_none() || self_loop.label.as_deref() == Some("")
                    {
                        (in_label.clone(), inbound.prec, out_label.clone(), outbound.prec)
                    } else if inbound.prec >= Prec::Concat
                        && self_loop.prec >= Prec::Concat
                        && in_label.ends_with(self_loop.label.as_ref().unwrap())
                        && !break_inbound_suffix(&in_label, self_loop.label.as_ref().unwrap())
                    {
                        let diff = in_label.len() - self_loop.label.as_ref().unwrap().len();
                        let mut first = String::new();
                        if diff != 0 && inbound.prec < Prec::Concat {
                            first.push('(');
                        }
                        first.push_str(&in_label[..diff]);
                        if diff != 0 && inbound.prec < Prec::Concat {
                            first.push(')');
                        }
                        if self_loop.prec <= Prec::Quant {
                            first.push('(');
                        }
                        first.push_str(self_loop.label.as_ref().unwrap());
                        if self_loop.prec <= Prec::Quant {
                            first.push(')');
                        }
                        first.push('+');
                        (first, Prec::Concat, out_label.clone(), outbound.prec)
                    } else if outbound.prec >= Prec::Concat
                        && self_loop.prec >= Prec::Concat
                        && out_label.starts_with(self_loop.label.as_ref().unwrap())
                    {
                        let diff = out_label.len() - self_loop.label.as_ref().unwrap().len();
                        let mut second = String::new();
                        if self_loop.prec <= Prec::Quant {
                            second.push('(');
                        }
                        second.push_str(self_loop.label.as_ref().unwrap());
                        if self_loop.prec <= Prec::Quant {
                            second.push(')');
                        }
                        second.push('+');
                        if diff != 0 && outbound.prec < Prec::Concat {
                            second.push('(');
                        }
                        second.push_str(&out_label[out_label.len() - diff..]);
                        if diff != 0 && outbound.prec < Prec::Concat {
                            second.push(')');
                        }
                        (in_label.clone(), inbound.prec, second, Prec::Concat)
                    } else {
                        let mut second = String::new();
                        if self_loop.prec <= Prec::Quant {
                            second.push('(');
                        }
                        second.push_str(self_loop.label.as_ref().unwrap());
                        if self_loop.prec <= Prec::Quant {
                            second.push(')');
                        }
                        second.push('*');
                        if outbound.prec < Prec::Concat {
                            second.push('(');
                        }
                        second.push_str(&out_label);
                        if outbound.prec < Prec::Concat {
                            second.push(')');
                        }
                        (in_label.clone(), inbound.prec, second, Prec::Concat)
                    };

                let (bypass_label, bypass_prec) = if first_label.is_empty() {
                    (second_label, second_prec)
                } else if second_label.is_empty() {
                    (first_label, first_prec)
                } else {
                    let mut bypass = String::new();
                    if first_prec < Prec::Concat {
                        bypass.push('(');
                    }
                    bypass.push_str(&first_label);
                    if first_prec < Prec::Concat {
                        bypass.push(')');
                    }
                    if second_prec < Prec::Concat {
                        bypass.push('(');
                    }
                    bypass.push_str(&second_label);
                    if second_prec < Prec::Concat {
                        bypass.push(')');
                    }
                    (bypass, Prec::Concat)
                };

                let merged = match (&existing.label, bypass_label.as_str()) {
                    (_, "") if bypass_label.is_empty() => existing.clone(),
                    (None, _) => Arrow {
                        label: Some(bypass_label),
                        prec: bypass_prec,
                    },
                    (Some(existing_label), _) if existing_label.is_empty() => {
                        let mut merged = String::new();
                        if bypass_prec <= Prec::Quant {
                            merged.push('(');
                        }
                        merged.push_str(&bypass_label);
                        if bypass_prec <= Prec::Quant {
                            merged.push(')');
                        }
                        merged.push('?');
                        Arrow {
                            label: Some(merged),
                            prec: Prec::Quant,
                        }
                    }
                    (Some(existing_label), _) => Arrow {
                        label: Some(format!("{}|{}", existing_label, bypass_label)),
                        prec: Prec::Alt,
                    },
                };

                arrows[id1][id2] = merged;
            }
        }

        for id in 0..=dfa_size {
            arrows[id][best_fit].label = None;
            arrows[best_fit][id].label = None;
        }
    }

    arrows[dfa_size][dfa_size]
        .label
        .clone()
        .unwrap_or_else(|| "[]".to_string())
}

fn break_inbound_suffix(in_label: &str, self_label: &str) -> bool {
    let diff = in_label.len() - self_label.len();
    let bytes = in_label.as_bytes();

    if diff >= 1
        && matches!(bytes[diff - 1], b'^' | b'-' | b'\\')
        && (diff == 1 || bytes[diff - 2] != b'\\')
    {
        return true;
    }
    if diff >= 2
        && &bytes[diff - 2..diff] == br"\x"
        && (diff == 2 || bytes[diff - 3] != b'\\')
    {
        return true;
    }
    if diff >= 3
        && &bytes[diff - 3..diff - 1] == br"\x"
        && (diff == 3 || bytes[diff - 4] != b'\\')
    {
        return true;
    }
    false
}

struct ParseContext<'a> {
    chars: &'a [u8],
    pos: usize,
}

impl<'a> ParseContext<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            chars: s.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let out = self.peek();
        if out.is_some() {
            self.pos += 1;
        }
        out
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn expect_char(&mut self) -> Result<u8, String> {
        self.next().ok_or_else(|| "unexpected end of input".to_string())
    }
}

fn parse_regex(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut re = parse_term(ctx)?;
    while matches!(ctx.peek(), Some(b'|') | Some(b'&')) {
        let intersect = ctx.next() == Some(b'&');
        let mut alt = parse_term(ctx)?;

        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);
        let re_final = re.final_;
        re.states[re.initial].epsilon1 = Some(alt.initial);
        re.states[re_final].epsilon0 = Some(alt.final_);

        let old_alt_initial = alt.initial;
        let base = re.states.len();
        for (idx, state) in alt.states.iter().enumerate() {
            let mapped = remap_nstate(state, old_alt_initial, base, re_final);
            if idx == old_alt_initial {
                continue;
            }
            re.states.push(mapped);
        }
        re.final_ = remap_index(alt.final_, old_alt_initial, base, re_final);
        if intersect {
            re.complemented = !re.complemented;
        }
    }
    Ok(re)
}

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let complement = if ctx.peek() == Some(b'~') {
        ctx.next();
        true
    } else {
        false
    };

    let mut term = Nfa::new_single();
    while !matches!(ctx.peek(), None | Some(b')') | Some(b'|') | Some(b'&')) {
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
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            atom.states[atom.initial].epsilon1 = Some(atom.final_);
            Ok(atom)
        }
        Some(b'+') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            atom.states[atom.final_].epsilon1 = Some(atom.initial);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            Ok(atom)
        }
        Some(b'?') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            if atom.states[atom.initial].epsilon1.is_some() {
                nfa_pad_initial(&mut atom);
            }
            atom.states[atom.initial].epsilon1 = Some(atom.final_);
            Ok(atom)
        }
        Some(b'{') => {
            let last_pos = ctx.pos;
            ctx.next();
            nfa_uncomplement(&mut atom)?;

            let min = match parse_natural(ctx) {
                Ok(v) => v,
                Err(err) if err == "expected natural number" => 0,
                Err(err) => return Err(err),
            };

            let mut max = min;
            let mut max_unbounded = false;
            if ctx.peek() == Some(b',') {
                ctx.next();
                match parse_natural(ctx) {
                    Ok(v) => max = v,
                    Err(err) if err == "expected natural number" => max_unbounded = true,
                    Err(err) => return Err(err),
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

            let mut atoms = Nfa::new_single();
            let mut i = 0u32;
            loop {
                if max_unbounded {
                    if i > min {
                        break;
                    }
                } else if i >= max {
                    break;
                }

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

                if i == u32::MAX {
                    break;
                }
                i += 1;
            }
            Ok(atoms)
        }
        _ => Ok(atom),
    }
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
    let mut initial = NState::new();
    initial.label = symset;
    initial.target = Some(1);
    Ok(Nfa {
        states: vec![initial, NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    })
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let complement = if ctx.peek() == Some(b'^') {
        ctx.next();
        true
    } else {
        false
    };

    let mut symset = if let Some(set) = try_parse_shorthand(ctx)? {
        set
    } else if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut out = SymSet::empty();
        while !matches!(ctx.peek(), None | Some(b']')) {
            let sub = parse_symset(ctx)?;
            out.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.next();
        out
    } else if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut out = SymSet::full();
        while !matches!(ctx.peek(), None | Some(b'>')) {
            let sub = parse_symset(ctx)?;
            out.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.next();
        out
    } else {
        let begin = parse_symbol(ctx)?;
        let end = if ctx.peek() == Some(b'-') {
            ctx.next();
            parse_symbol(ctx)?
        } else {
            begin
        };

        let mut out = SymSet::empty();
        let mut chr = begin;
        let end_open = end.wrapping_add(1);
        loop {
            out.insert(chr);
            chr = chr.wrapping_add(1);
            if chr == end_open {
                break;
            }
        }
        out
    };

    if complement {
        symset.invert();
    }
    Ok(symset)
}

fn try_parse_shorthand(ctx: &mut ParseContext) -> Result<Option<SymSet>, String> {
    let save = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        let out = match ctx.next() {
            Some(b'd') => Some(digits_set()),
            Some(b'D') => Some(not_digits_set()),
            Some(b's') => Some(spaces_set()),
            Some(b'S') => Some(not_spaces_set()),
            Some(b'w') => Some(wordchar_set()),
            Some(b'W') => Some(not_wordchar_set()),
            Some(other) => {
                ctx.pos = save;
                if other == 0 {
                    None
                } else {
                    None
                }
            }
            None => {
                ctx.pos = save;
                None
            }
        };
        if out.is_some() {
            return Ok(out);
        }
        ctx.pos = save;
    }

    if ctx.peek() == Some(b'.') {
        ctx.next();
        let mut set = SymSet::full();
        set.bits[(b'\n' as usize) / 8] &= !(1 << (b'\n' % 8));
        return Ok(Some(set));
    }

    Ok(None)
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        return parse_escape(ctx);
    }

    let chr = ctx.peek().ok_or_else(|| "expected symbol".to_string())?;
    if METACHARS.contains(&chr) {
        return Err("unexpected metacharacter".to_string());
    }
    if !(chr.is_ascii_graphic() || chr == b' ') {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.next();
    Ok(chr)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    if let Some(chr) = ctx.peek() {
        if METACHARS.contains(&chr) {
            ctx.next();
            return Ok(chr);
        }
    }

    match ctx.next() {
        Some(b'a') => Ok(0x07),
        Some(b'b') => Ok(0x08),
        Some(b'f') => Ok(0x0c),
        Some(b'n') => Ok(b'\n'),
        Some(b'r') => Ok(b'\r'),
        Some(b't') => Ok(b'\t'),
        Some(b'v') => Ok(0x0b),
        Some(b'x') => {
            let hi = parse_hex_nibble(ctx.expect_char()?)?;
            let lo = parse_hex_nibble(ctx.expect_char()?)?;
            Ok((hi << 4) | lo)
        }
        Some(_) | None => Err("unknown escape".to_string()),
    }
}

fn parse_hex_nibble(chr: u8) -> Result<u8, String> {
    match chr {
        b'0'..=b'9' => Ok(chr - b'0'),
        b'a'..=b'f' => Ok(chr - b'a' + 10),
        b'A'..=b'F' => Ok(chr - b'A' + 10),
        _ => Err("expected hex digit".to_string()),
    }
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}

fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

fn digits_set() -> SymSet {
    let mut set = SymSet::empty();
    for chr in b'0'..=b'9' {
        set.insert(chr);
    }
    set
}

fn not_digits_set() -> SymSet {
    let mut set = digits_set();
    set.invert();
    set
}

fn spaces_set() -> SymSet {
    let mut set = SymSet::empty();
    for chr in [b' ', 0x0c, b'\n', b'\r', b'\t', 0x0b] {
        set.insert(chr);
    }
    set
}

fn not_spaces_set() -> SymSet {
    let mut set = spaces_set();
    set.invert();
    set
}

fn wordchar_set() -> SymSet {
    let mut set = SymSet::empty();
    set.insert(b'_');
    for chr in b'0'..=b'9' {
        set.insert(chr);
    }
    for chr in b'a'..=b'z' {
        set.insert(chr);
    }
    for chr in b'A'..=b'Z' {
        set.insert(chr);
    }
    set
}

fn not_wordchar_set() -> SymSet {
    let mut set = wordchar_set();
    set.invert();
    set
}

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    let mut natural = 0u32;
    let mut saw_digit = false;
    while let Some(chr) = ctx.peek() {
        if !chr.is_ascii_digit() {
            break;
        }
        saw_digit = true;
        let digit = (chr - b'0') as u32;
        natural = natural
            .checked_mul(10)
            .and_then(|n| n.checked_add(digit))
            .ok_or_else(|| "natural number overflow".to_string())?;
        ctx.next();
    }
    if saw_digit {
        Ok(natural)
    } else {
        Err("expected natural number".to_string())
    }
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt {
        *v += offset;
    }
}
