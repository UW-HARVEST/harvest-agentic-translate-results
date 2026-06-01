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
        (self.bits[(c >> 3) as usize] >> (c & 7)) & 1 != 0
    }
    pub fn insert(&mut self, c: u8) {
        self.bits[(c >> 3) as usize] |= 1 << (c & 7);
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

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_metachar_byte(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn is_print_byte(c: u8) -> bool {
    (0x20..=0x7e).contains(&c)
}

fn append_symset_char(buf: &mut Vec<u8>, c: u8) {
    if !is_print_byte(c) && !is_metachar_byte(c) {
        let s = format!("\\x{:02x}", c);
        buf.extend_from_slice(s.as_bytes());
    } else {
        if is_metachar_byte(c) {
            buf.push(b'\\');
        }
        buf.push(c);
    }
}

pub fn symset_fmt(set: &SymSet) -> String {
    let mut buf: Vec<u8> = vec![b'['];
    let mut nbuf: Vec<u8> = vec![b'^', b'['];
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;

    let mut chr: u32 = 0;
    while chr < 256 {
        // Append start of run
        let c = chr as u8;
        let in_set = set.contains(c);
        if in_set {
            nsym += 1;
        } else {
            nnsym += 1;
        }
        if in_set {
            append_symset_char(&mut buf, c);
        } else {
            append_symset_char(&mut nbuf, c);
        }

        // Find run end
        let start = chr;
        while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
            chr += 1;
        }

        if chr - start >= 2 {
            // Append dash, decrement count
            if in_set {
                buf.push(b'-');
                nsym -= 1;
            } else {
                nbuf.push(b'-');
                nnsym -= 1;
            }
        }
        if chr - start >= 1 {
            // Append end of run (which has same membership as start)
            let c2 = chr as u8;
            // membership of c2 == membership of c (start) because the run consists of
            // chars with same membership
            if in_set {
                nsym += 1;
                append_symset_char(&mut buf, c2);
            } else {
                nnsym += 1;
                append_symset_char(&mut nbuf, c2);
            }
        }

        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        let inner = &buf[1..buf.len() - 1];
        return String::from_utf8_lossy(inner).into_owned();
    } else if nnsym == 1 {
        let content = &nbuf[2..nbuf.len() - 1];
        let mut result = b"^".to_vec();
        result.extend_from_slice(content);
        return String::from_utf8_lossy(&result).into_owned();
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

fn map_nstate(st: &NState, map: &[usize]) -> NState {
    NState {
        label: st.label,
        target: st.target.map(|t| map[t]),
        epsilon0: st.epsilon0.map(|t| map[t]),
        epsilon1: st.epsilon1.map(|t| map[t]),
    }
}

pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ {
        // nfa1 is single-state, replace with nfa2 entirely
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        // nfa2 is single-state, do nothing
        return;
    }
    // Merge nfa2 into nfa1: nfa2.initial's contents go into nfa1.final_;
    // other nfa2 states get appended.
    let n1 = nfa1.states.len();
    let mut map: Vec<usize> = Vec::with_capacity(nfa2.states.len());
    let mut next_idx = n1;
    for i in 0..nfa2.states.len() {
        if i == nfa2.initial {
            map.push(nfa1.final_);
        } else {
            map.push(next_idx);
            next_idx += 1;
        }
    }

    // Replace nfa1.states[nfa1.final_] with nfa2's initial state (mapped)
    let initial_state = nfa2.states[nfa2.initial].clone();
    nfa1.states[nfa1.final_] = map_nstate(&initial_state, &map);

    // Append all other states from nfa2 (in original order, skipping initial)
    for (i, st) in nfa2.states.iter().enumerate() {
        if i != nfa2.initial {
            nfa1.states.push(map_nstate(st, &map));
        }
    }

    nfa1.final_ = map[nfa2.final_];
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    let mut new_state = NState::new();
    new_state.epsilon0 = Some(nfa.initial);
    nfa.states.push(new_state);
    nfa.initial = new_idx;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_idx = nfa.states.len();
    let new_state = NState::new();
    nfa.states[nfa.final_].epsilon0 = Some(new_idx);
    nfa.states.push(new_state);
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
    for (id, state) in nfa.states.iter().enumerate() {
        if let Some(e0) = state.epsilon0 {
            println!("  {} --> {}", id, e0);
        }
        if let Some(e1) = state.epsilon1 {
            println!("  {} --> {}", id, e1);
        }
        if state.label.is_empty() {
            continue;
        }
        // Mermaid escaping
        print!("  {} --", id);
        let fmt = symset_fmt(&state.label);
        for c in fmt.bytes() {
            if b"\\\"#&{}()xo=- ".contains(&c) {
                print!("#{};", c);
            } else {
                print!("{}", c as char);
            }
        }
        if let Some(t) = state.target {
            println!("--> {}", t);
        } else {
            println!("--> ?");
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
            buf.push((chr - start) as u8);
            leb128_put(&mut buf, state.transitions[chr] as i32);
            chr += 1;
        }
    }

    buf
}

pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p = 0;
    let dfa_size = leb128_get(buf, &mut p)? as usize;

    let mut states: Vec<DState> = (0..dfa_size)
        .map(|_| DState {
            transitions: [0; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        })
        .collect();

    for id in 0..dfa_size {
        if p >= buf.len() {
            return Err("unexpected end of input".to_string());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;

        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("unexpected end of input".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            for _ in 0..=len {
                if chr >= 256 {
                    return Err("RLE overflow".to_string());
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
            for chr in 0..256u32 {
                if ds1.transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
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
    while ((n as u32) >> 7) != 0 {
        buf.push(((n as u8) & 0x7f) | 0x80);
        n = ((n as u32) >> 7) as i32;
    }
    buf.push(n as u8);
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: i32 = 0;
    let mut c: u32 = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128: unexpected end of input".to_string());
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
        let new_idx = nfa.states.len();
        let new_state = NState::new();
        nfa.states.push(new_state);
        let final_old = nfa.final_;
        nfa.states[final_old].label.insert(b);
        nfa.states[final_old].target = Some(new_idx);
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
        for chr in 0..=255u8 {
            if state.label.contains(chr) {
                let lower = chr.to_ascii_lowercase();
                let upper = chr.to_ascii_uppercase();
                state.label.insert(lower);
                state.label.insert(upper);
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();

    let mut states: Vec<DState> = Vec::new();

    // Initial state from epsilon closure of NFA initial
    let init_bs = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let init_acc = bitset_test(&init_bs, nfa.final_) ^ nfa.complemented;
    states.push(DState {
        transitions: [0; 256],
        accepting: init_acc,
        terminating: false,
        bitset: init_bs,
    });

    // BFS
    let mut idx = 0;
    while idx < states.len() {
        for chr in 0..256u32 {
            let new_bs = step_powerset(&nfa, &states[idx].bitset, chr as u8);
            // find or insert
            let mut found = None;
            for (i, st) in states.iter().enumerate() {
                if st.bitset == new_bs {
                    found = Some(i);
                    break;
                }
            }
            let target = match found {
                Some(i) => i,
                None => {
                    let acc = bitset_test(&new_bs, nfa.final_) ^ nfa.complemented;
                    states.push(DState {
                        transitions: [0; 256],
                        accepting: acc,
                        terminating: false,
                        bitset: new_bs,
                    });
                    states.len() - 1
                }
            };
            states[idx].transitions[chr as usize] = target;
        }
        idx += 1;
    }

    let mut dfa = Dfa { states, initial: 0 };
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    for (i, s) in states.iter().enumerate() {
        if !s.accepting && s.transitions.iter().all(|&t| t == i) {
            return i;
        }
    }
    let new_id = states.len();
    states.push(DState {
        transitions: [new_id; 256],
        accepting: false,
        terminating: true,
        bitset: Vec::new(),
    });
    new_id
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

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let bs_size = (nfa_size + 7) / 8;
    let mut bs = vec![0u8; bs_size];
    epsilon_closure_into(nfa, start, &mut bs);
    bs
}

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    let mut stack: Vec<usize> = vec![st_id];
    while let Some(id) = stack.pop() {
        if bitset_test(bitset, id) {
            continue;
        }
        bitset_set(bitset, id);
        if let Some(e0) = nfa.states[id].epsilon0 {
            stack.push(e0);
        }
        if let Some(e1) = nfa.states[id].epsilon1 {
            stack.push(e1);
        }
    }
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }

    // Distinguishability matrix
    let mut dis: Vec<Vec<bool>> = vec![vec![false; n]; n];

    // Initialize: states with different accepting are distinguishable
    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                dis[i][j] = true;
                dis[j][i] = true;
            }
        }
    }

    // Iterate to fixpoint
    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            for j in (i + 1)..n {
                if !dis[i][j] {
                    for chr in 0..256 {
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
        }
    }

    // For each state, find its leader: smallest index j < i such that !dis[j][i]
    let mut leader: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in 0..i {
            if !dis[j][i] {
                leader[i] = leader[j];
                break;
            }
        }
    }

    // Assign new ids in order
    let mut new_idx: Vec<Option<usize>> = vec![None; n];
    let mut next_id = 0;
    for i in 0..n {
        if leader[i] == i {
            new_idx[i] = Some(next_id);
            next_id += 1;
        }
    }

    // Build new states
    let mut new_states: Vec<DState> = Vec::with_capacity(next_id);
    for i in 0..n {
        if leader[i] == i {
            let mut s = dfa.states[i].clone();
            for chr in 0..256 {
                let target = s.transitions[chr];
                let target_leader = leader[target];
                s.transitions[chr] = new_idx[target_leader].unwrap();
            }
            new_states.push(s);
        }
    }

    // Update initial
    let initial_leader = leader[dfa.initial];
    let new_initial = new_idx[initial_leader].unwrap();

    // Set terminating
    for i in 0..new_states.len() {
        let mut term = true;
        for chr in 0..256 {
            if new_states[i].transitions[chr] != i {
                term = false;
                break;
            }
        }
        new_states[i].terminating = term;
    }

    dfa.states = new_states;
    dfa.initial = new_initial;
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    let byte_i = idx >> 3;
    if byte_i >= bs.len() {
        return false;
    }
    bs[byte_i] & (1 << (idx & 7)) != 0
}

fn bitset_set(bs: &mut [u8], idx: usize) {
    bs[idx >> 3] |= 1 << (idx & 7);
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
    for &c in input {
        if dfa.states[state].terminating {
            break;
        }
        state = dfa.states[state].transitions[c as usize];
    }
    dfa.states[state].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    if dfap.is_none() {
        *dfap = Some(Dfa::new());
    }
    let dfa = dfap.as_mut().unwrap();

    let nfa_size = nfa.states.len();

    if dfa.states.is_empty() {
        let bs = epsilon_closure_vec(nfa, nfa.initial, nfa_size);
        let acc = bitset_test(&bs, nfa.final_) ^ nfa.complemented;
        dfa.states.push(DState {
            transitions: [usize::MAX; 256],
            accepting: acc,
            terminating: false,
            bitset: bs,
        });
        dfa.initial = 0;
    }

    let mut state = dfa.initial;
    for &c in input {
        let next_state = dfa.states[state].transitions[c as usize];
        let target = if next_state == usize::MAX {
            // Lazy step
            let new_bs = step_powerset(nfa, &dfa.states[state].bitset, c);
            let mut found = None;
            for (i, st) in dfa.states.iter().enumerate() {
                if st.bitset == new_bs {
                    found = Some(i);
                    break;
                }
            }
            let t = match found {
                Some(i) => i,
                None => {
                    let acc = bitset_test(&new_bs, nfa.final_) ^ nfa.complemented;
                    dfa.states.push(DState {
                        transitions: [usize::MAX; 256],
                        accepting: acc,
                        terminating: false,
                        bitset: new_bs,
                    });
                    dfa.states.len() - 1
                }
            };
            dfa.states[state].transitions[c as usize] = t;
            t
        } else {
            next_state
        };
        state = target;
    }

    dfa.states[state].accepting
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();
    let mut states: Vec<NState> = Vec::new();

    // Index 0: initial
    states.push(NState::new());
    let initial_idx = 0;

    // Index 1: final
    states.push(NState::new());
    let final_idx = 1;

    // Indices 2..2+dfa_size: nstates[id] for each dfa state
    for _ in 0..dfa_size {
        states.push(NState::new());
    }
    let nstate_idx = |id: usize| -> usize { 2 + id };

    // initial.epsilon1 -> nstates[dfa.initial]
    states[initial_idx].epsilon1 = Some(nstate_idx(dfa.initial));

    // accepting nstates' epsilon1 -> final
    for (id, dstate) in dfa.states.iter().enumerate() {
        if dstate.accepting {
            states[nstate_idx(id)].epsilon1 = Some(final_idx);
        }
    }

    // Build binary trees for each ds1
    for (id1, ds1) in dfa.states.iter().enumerate() {
        let mut free_idx: Option<usize> = None;
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if ds1.transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }

            let src_idx;
            if free_idx.is_none() {
                let root = nstate_idx(id1);
                free_idx = Some(root);
                src_idx = root;
            } else {
                let free_i = free_idx.unwrap();
                let new_idx = states.len();
                states.push(NState::new());
                src_idx = new_idx;
                if states[free_i].epsilon1.is_none() {
                    states[free_i].epsilon1 = Some(new_idx);
                } else {
                    states[free_i].epsilon0 = Some(new_idx);
                    free_idx = Some(new_idx);
                }
            }

            states[src_idx].target = Some(nstate_idx(id2));
            states[src_idx].label = transitions;
        }
    }

    Nfa {
        states,
        initial: initial_idx,
        final_: final_idx,
        complemented: false,
    }
}

const PREC_ALT: i32 = 0;
const PREC_CONCAT: i32 = 1;
const PREC_QUANT: i32 = 2;
const PREC_SYMSET: i32 = 3;

#[derive(Clone, Debug)]
struct Arrow {
    label: Option<String>,
    prec: i32,
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let n = dfa_size + 1;
    let aux = dfa_size; // auxiliary state index

    // arrows[id1][id2]
    let mut arrows: Vec<Vec<Arrow>> = (0..n)
        .map(|_| {
            (0..n)
                .map(|_| Arrow {
                    label: None,
                    prec: PREC_SYMSET,
                })
                .collect()
        })
        .collect();

    // Epsilon transition from aux to dfa.initial
    arrows[aux][dfa.initial] = Arrow {
        label: Some(String::new()),
        prec: PREC_SYMSET,
    };

    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            arrows[id1][aux] = Arrow {
                label: Some(String::new()),
                prec: PREC_SYMSET,
            };
        }
        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256u32 {
                if ds1.transitions[chr as usize] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let fmt = symset_fmt(&transitions);
            arrows[id1][id2] = Arrow {
                label: Some(fmt),
                prec: PREC_SYMSET,
            };
        }
    }

    loop {
        // Find best fit (state with minimum non-zero degree)
        let mut best_fit: Option<usize> = None;
        let mut min_degree: i32 = i32::MAX;
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

        let bf = match best_fit {
            Some(b) => b,
            None => break,
        };

        // Bypass through bf
        for id1 in 0..n {
            if id1 == bf {
                continue;
            }
            for id2 in 0..n {
                if id2 == bf {
                    continue;
                }
                let in_a = arrows[id1][bf].clone();
                let out_a = arrows[bf][id2].clone();
                let self_a = arrows[bf][bf].clone();
                let existing = arrows[id1][id2].clone();

                if in_a.label.is_none() || out_a.label.is_none() {
                    continue;
                }

                let in_label = in_a.label.as_ref().unwrap().as_bytes();
                let out_label = out_a.label.as_ref().unwrap().as_bytes();
                let self_label_opt = self_a.label.as_ref();

                // Decide first/second
                let (first, second) = compute_first_second(&in_a, &out_a, &self_a);

                // Concatenate first and second to form bypass
                let bypass = concat_arrows(&first, &second);

                // Merge with existing
                let merged = merge_arrows(&existing, &bypass);

                arrows[id1][id2] = merged;
                let _ = (in_label, out_label, self_label_opt);
            }
        }

        // Eliminate the bf state by clearing all its in/out arrows
        for id in 0..n {
            arrows[id][bf].label = None;
            arrows[bf][id].label = None;
        }
    }

    arrows[aux][aux]
        .label
        .clone()
        .unwrap_or_else(|| "[]".to_string())
}

fn compute_first_second(in_a: &Arrow, out_a: &Arrow, self_a: &Arrow) -> (Arrow, Arrow) {
    // Self-transition handling
    if self_a.label.is_none() || self_a.label.as_ref().unwrap().is_empty() {
        // (in)[]*(out) == (in)()*(out) == (in)(out)
        return (in_a.clone(), out_a.clone());
    }

    let self_label = self_a.label.as_ref().unwrap();
    let in_label = in_a.label.as_ref().unwrap();
    let out_label = out_a.label.as_ref().unwrap();

    // Try splitting into in_pre + self
    let try_pre = in_a.prec >= PREC_CONCAT && self_a.prec >= PREC_CONCAT;
    if try_pre {
        if in_label.len() >= self_label.len() {
            let diff = in_label.len() - self_label.len();
            if &in_label[diff..] == self_label.as_str() {
                // Check the hacky avoidance conditions
                let in_bytes = in_label.as_bytes();
                let mut nevermind = false;
                if diff >= 1 {
                    let c = in_bytes[diff - 1];
                    if b"^-\\".contains(&c) && (diff == 1 || in_bytes[diff - 2] != b'\\') {
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
                    // Build first = (in_pre)(self)+
                    let mut first_str = String::new();
                    if diff != 0 && in_a.prec < PREC_CONCAT {
                        first_str.push('(');
                    }
                    first_str.push_str(&in_label[..diff]);
                    if diff != 0 && in_a.prec < PREC_CONCAT {
                        first_str.push(')');
                    }
                    if self_a.prec <= PREC_QUANT {
                        first_str.push('(');
                    }
                    first_str.push_str(self_label);
                    if self_a.prec <= PREC_QUANT {
                        first_str.push(')');
                    }
                    first_str.push('+');
                    return (
                        Arrow {
                            label: Some(first_str),
                            prec: PREC_CONCAT,
                        },
                        out_a.clone(),
                    );
                }
            }
        }
    }

    // Try splitting into self + out_post
    let try_post = out_a.prec >= PREC_CONCAT && self_a.prec >= PREC_CONCAT;
    if try_post {
        if out_label.len() >= self_label.len() {
            let s_len = self_label.len();
            if &out_label[..s_len] == self_label.as_str() {
                let diff = out_label.len() - self_label.len();
                let mut second_str = String::new();
                if self_a.prec <= PREC_QUANT {
                    second_str.push('(');
                }
                second_str.push_str(self_label);
                if self_a.prec <= PREC_QUANT {
                    second_str.push(')');
                }
                second_str.push('+');
                if diff != 0 && out_a.prec < PREC_CONCAT {
                    second_str.push('(');
                }
                second_str.push_str(&out_label[s_len..]);
                if diff != 0 && out_a.prec < PREC_CONCAT {
                    second_str.push(')');
                }
                return (
                    in_a.clone(),
                    Arrow {
                        label: Some(second_str),
                        prec: PREC_CONCAT,
                    },
                );
            }
        }
    }

    // Default: (in)(self)*(out)
    let mut second_str = String::new();
    if self_a.prec <= PREC_QUANT {
        second_str.push('(');
    }
    second_str.push_str(self_label);
    if self_a.prec <= PREC_QUANT {
        second_str.push(')');
    }
    second_str.push('*');
    if out_a.prec < PREC_CONCAT {
        second_str.push('(');
    }
    second_str.push_str(out_label);
    if out_a.prec < PREC_CONCAT {
        second_str.push(')');
    }
    (
        in_a.clone(),
        Arrow {
            label: Some(second_str),
            prec: PREC_CONCAT,
        },
    )
}

fn concat_arrows(first: &Arrow, second: &Arrow) -> Arrow {
    let f_label = first.label.as_ref().unwrap();
    let s_label = second.label.as_ref().unwrap();

    if f_label.is_empty() {
        return second.clone();
    }
    if s_label.is_empty() {
        return first.clone();
    }

    let mut s = String::new();
    if first.prec < PREC_CONCAT {
        s.push('(');
    }
    s.push_str(f_label);
    if first.prec < PREC_CONCAT {
        s.push(')');
    }
    if second.prec < PREC_CONCAT {
        s.push('(');
    }
    s.push_str(s_label);
    if second.prec < PREC_CONCAT {
        s.push(')');
    }
    Arrow {
        label: Some(s),
        prec: PREC_CONCAT,
    }
}

fn merge_arrows(existing: &Arrow, bypass: &Arrow) -> Arrow {
    if bypass.label.is_none() {
        return existing.clone();
    }
    if existing.label.is_none() {
        return bypass.clone();
    }
    let bypass_label = bypass.label.as_ref().unwrap();
    let existing_label = existing.label.as_ref().unwrap();

    if existing_label.is_empty() {
        // ()|(bypass) == (bypass)?
        let mut s = String::new();
        if bypass.prec <= PREC_QUANT {
            s.push('(');
        }
        s.push_str(bypass_label);
        if bypass.prec <= PREC_QUANT {
            s.push(')');
        }
        s.push('?');
        return Arrow {
            label: Some(s),
            prec: PREC_QUANT,
        };
    }

    // (existing)|(bypass)
    let mut s = String::new();
    s.push_str(existing_label);
    s.push('|');
    s.push_str(bypass_label);
    Arrow {
        label: Some(s),
        prec: PREC_ALT,
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
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
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
        self.next().ok_or_else(|| "unexpected end of input".to_string())
    }
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte = 0u8;
    for _ in 0..2 {
        byte <<= 4;
        let chr = match ctx.peek() {
            Some(c) => c,
            None => return Err("expected hex digit".to_string()),
        };
        if chr.is_ascii_digit() {
            byte |= chr - b'0';
        } else if chr.is_ascii_hexdigit() {
            byte |= chr.to_ascii_lowercase() - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.next();
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = match ctx.peek() {
        Some(c) => c,
        None => return Err("unknown escape".to_string()),
    };
    if METACHARS.contains(&c) {
        ctx.next();
        return Ok(c);
    }
    let saved = ctx.pos;
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
            ctx.pos = saved;
            Err("unknown escape".to_string())
        }
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        return parse_escape(ctx);
    }
    let c = match ctx.peek() {
        Some(c) => c,
        None => return Err("expected symbol".to_string()),
    };
    if METACHARS.contains(&c) {
        return Err("unexpected metacharacter".to_string());
    }
    if !is_print_byte(c) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.next();
    Ok(c)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let saved = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        match ctx.peek() {
            Some(c) => {
                ctx.next();
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
            }
            None => {}
        }
        // backtrack
        ctx.pos = saved;
    }
    if ctx.peek() == Some(b'.') {
        ctx.next();
        // anything except '\n'
        let mut s = SymSet::full();
        // remove '\n' (0x0a)
        let mut n_set = SymSet::empty();
        n_set.insert(b'\n');
        n_set.invert();
        s.intersect_with(&n_set);
        return Ok(s);
    }
    Err("expected shorthand class".to_string())
}

fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let complement = ctx.peek() == Some(b'^');
    if complement {
        ctx.next();
    }

    let last_pos = ctx.pos;

    let mut result;

    // Try shorthand
    match parse_shorthand(ctx) {
        Ok(s) => {
            result = s;
        }
        Err(_) => {
            ctx.pos = last_pos;
            // Try [...] or <...>
            if ctx.peek() == Some(b'[') {
                ctx.next();
                result = SymSet::empty();
                while ctx.peek().map_or(false, |c| c != b']') {
                    let sub = parse_symset(ctx)?;
                    result.union_with(&sub);
                }
                if ctx.peek() != Some(b']') {
                    return Err("expected ']'".to_string());
                }
                ctx.next();
            } else if ctx.peek() == Some(b'<') {
                ctx.next();
                result = SymSet::full();
                while ctx.peek().map_or(false, |c| c != b'>') {
                    let sub = parse_symset(ctx)?;
                    result.intersect_with(&sub);
                }
                if ctx.peek() != Some(b'>') {
                    return Err("expected '>'".to_string());
                }
                ctx.next();
            } else {
                // Single symbol or range
                let begin = parse_symbol(ctx)?;
                let mut end = begin;
                if ctx.peek() == Some(b'-') {
                    ctx.next();
                    end = parse_symbol(ctx)?;
                }
                result = SymSet::empty();
                let mut chr = begin;
                let end_after = end.wrapping_add(1);
                loop {
                    result.insert(chr);
                    chr = chr.wrapping_add(1);
                    if chr == end_after {
                        break;
                    }
                }
            }
        }
    }

    if complement {
        result.invert();
    }
    Ok(result)
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

    let label = parse_symset(ctx)?;
    let mut initial = NState::new();
    let final_st = NState::new();
    initial.label = label;
    initial.target = Some(1);
    let nfa = Nfa {
        states: vec![initial, final_st],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    match ctx.peek() {
        Some(b'*') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[fin].epsilon1 = Some(init);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[init].epsilon1 = Some(fin);
            Ok(atom)
        }
        Some(b'+') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[fin].epsilon1 = Some(init);
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
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[init].epsilon1 = Some(fin);
            Ok(atom)
        }
        Some(b'{') => {
            let last_pos = ctx.pos;
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            let min = match parse_natural(ctx) {
                Ok(n) => n,
                Err(e) if e == "natural number overflow" => return Err(e),
                Err(_) => 0,
            };
            let mut max = min;
            let mut max_unbounded = false;
            if ctx.peek() == Some(b',') {
                ctx.next();
                match parse_natural(ctx) {
                    Ok(n) => max = n,
                    Err(e) if e == "natural number overflow" => return Err(e),
                    Err(_) => {
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

            let mut atoms = Nfa::new_single();

            let mut i: u32 = 0;
            loop {
                let cont = if max_unbounded { i <= min } else { i < max };
                if !cont {
                    break;
                }
                let mut clone = atom.clone();
                if i >= min {
                    if max_unbounded {
                        let init_c = clone.initial;
                        let fin_c = clone.final_;
                        clone.states[fin_c].epsilon1 = Some(init_c);
                        nfa_pad_initial(&mut clone);
                        nfa_pad_final(&mut clone);
                    }
                    let init_c = clone.initial;
                    let fin_c = clone.final_;
                    clone.states[init_c].epsilon1 = Some(fin_c);
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

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let complement = ctx.peek() == Some(b'~');
    if complement {
        ctx.next();
    }

    let mut term = Nfa::new_single();

    loop {
        match ctx.peek() {
            None | Some(b')') | Some(b'|') | Some(b'&') => break,
            _ => {}
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

    while matches!(ctx.peek(), Some(b'|') | Some(b'&')) {
        let intersect = ctx.peek() == Some(b'&');
        ctx.next();
        let mut alt = parse_term(ctx)?;

        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        // Build alternation:
        // -->O-->(re)--->
        //     -->(alt)-->O-->
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        let offset = re.states.len();
        let alt_initial_new = alt.initial + offset;
        let alt_final_new = alt.final_ + offset;

        // Append alt states with offset
        for st in alt.states.into_iter() {
            re.states.push(NState {
                label: st.label,
                target: st.target.map(|t| t + offset),
                epsilon0: st.epsilon0.map(|t| t + offset),
                epsilon1: st.epsilon1.map(|t| t + offset),
            });
        }

        let re_init = re.initial;
        let re_fin = re.final_;
        re.states[re_init].epsilon1 = Some(alt_initial_new);
        re.states[re_fin].epsilon0 = Some(alt_final_new);
        re.final_ = alt_final_new;

        re.complemented ^= intersect;
    }

    Ok(re)
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
        // C isspace: ' ', '\t', '\n', '\v', '\f', '\r'
        if c == b' ' || c == b'\t' || c == b'\n' || c == 0x0b || c == 0x0c || c == b'\r' {
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
        if c == b'_' || c.is_ascii_alphanumeric() {
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

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    if !ctx.peek().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        return Err("expected natural number".to_string());
    }
    let mut n: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !c.is_ascii_digit() {
            break;
        }
        let d = (c - b'0') as u32;
        if n > u32::MAX / 10 || n * 10 > u32::MAX - d {
            return Err("natural number overflow".to_string());
        }
        n = n * 10 + d;
        ctx.next();
    }
    Ok(n)
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt {
        *v += offset;
    }
}
