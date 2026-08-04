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
        self.bits[(c as usize) / 8] & (1u8 << ((c as usize) % 8)) != 0
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
    // C isprint: printable chars including space (0x20..0x7e)
    (0x20..=0x7e).contains(&c)
}

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_xdigit(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn is_space(c: u8) -> bool {
    // C isspace: ' ', '\t', '\n', '\v', '\f', '\r'
    matches!(c, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn is_alnum(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn to_lower(c: u8) -> u8 {
    if (b'A'..=b'Z').contains(&c) {
        c + 32
    } else {
        c
    }
}

fn to_upper(c: u8) -> u8 {
    if (b'a'..=b'z').contains(&c) {
        c - 32
    } else {
        c
    }
}

pub fn symset_fmt(set: &SymSet) -> String {
    // Mirrors C symset_fmt logic. Output is parsable by parse_symset.
    let mut buf: Vec<u8> = Vec::new();
    let mut nbuf: Vec<u8> = Vec::new();
    let mut nsym = 0i32;
    let mut nnsym = 0i32;

    nbuf.push(b'^');
    buf.push(b'[');
    nbuf.push(b'[');

    let mut chr: i32 = 0;
    while chr < 256 {
        // append_chr loop body (handles range expansion)
        loop {
            let c = chr as u8;
            let in_set = set.contains(c);
            if in_set {
                nsym += 1;
            } else {
                nnsym += 1;
            }
            let p = if in_set { &mut buf } else { &mut nbuf };

            let metachar = is_metachar(c);
            if !is_print(c) && !metachar {
                p.extend_from_slice(format!("\\x{:02x}", c).as_bytes());
            } else {
                if metachar {
                    p.push(b'\\');
                }
                p.push(c);
            }

            // make character ranges
            let start = chr;
            while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
                chr += 1;
            }
            if chr - start >= 2 {
                let p2 = if set.contains(chr as u8) {
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
                continue; // re-execute append_chr with updated chr
            }
            break;
        }
        chr += 1;
    }

    buf.push(b']');
    nbuf.push(b']');

    // Special cases. C uses bufp[-2] which targets the closing ']' (since
    // bufp is one past the trailing '\0'). Our buffers don't have a trailing
    // null, so to match: drop the last byte (the ']') and skip the first byte
    // (the '[').
    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        let end = buf.len().saturating_sub(1);
        let s = &buf[1..end];
        return String::from_utf8_lossy(s).into_owned();
    } else if nnsym == 1 {
        let mut nbuf2 = nbuf.clone();
        if nbuf2.len() > 1 {
            nbuf2[1] = b'^';
        }
        let end = nbuf2.len().saturating_sub(1);
        let s = &nbuf2[1..end];
        return String::from_utf8_lossy(s).into_owned();
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
        // single state that is both initial and final
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

pub fn nfa_concat(nfa1: &mut Nfa, mut nfa2: Nfa) {
    // Visually concatenate nfa2 onto nfa1.
    // C: if nfap->initial == nfap->final, replace with nfa2.
    // Otherwise, copy nfa2.initial's contents into nfap->final and free nfa2.initial.
    // We'll merge nfa2's states (except its initial) into nfa1, replacing nfa1's
    // final state's contents with nfa2's initial state's contents.

    if nfa1.initial == nfa1.final_ {
        // Replace nfa1 with nfa2
        *nfa1 = nfa2;
        return;
    }

    if nfa2.initial == nfa2.final_ {
        // C: if nfa.initial == nfa.final and nfap->initial != nfap->final,
        // do nothing. Just discard nfa2.
        return;
    }

    // General case: nfa2 has multiple states. Merge them.
    // Strategy: append all nfa2 states to nfa1 except its initial; the initial's
    // contents replace nfa1.final_.
    let n1_len = nfa1.states.len();
    let old_init = nfa2.initial;

    // Build mapping for nfa2's state indices into nfa1's index space.
    // The initial of nfa2 maps to nfa1.final_.
    // All other states get appended.
    let n2_len = nfa2.states.len();
    let mut mapping: Vec<usize> = vec![0; n2_len];
    let mut next_idx = n1_len;
    for i in 0..n2_len {
        if i == old_init {
            mapping[i] = nfa1.final_;
        } else {
            mapping[i] = next_idx;
            next_idx += 1;
        }
    }

    // Remap nfa2 states
    let remap = |opt: Option<usize>| -> Option<usize> { opt.map(|i| mapping[i]) };
    for state in nfa2.states.iter_mut() {
        state.target = remap(state.target);
        state.epsilon0 = remap(state.epsilon0);
        state.epsilon1 = remap(state.epsilon1);
    }

    // Replace nfa1.final_'s contents with nfa2's initial state
    nfa1.states[nfa1.final_] = nfa2.states[old_init].clone();

    // Append all other states in order
    for i in 0..n2_len {
        if i != old_init {
            nfa1.states.push(nfa2.states[i].clone());
        }
    }

    // Update nfa1.final_ to point to mapped nfa2.final_
    nfa1.final_ = mapping[nfa2.final_];
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    // Add a new state, set its epsilon0 to old initial, make it new initial.
    let mut new_state = NState::new();
    new_state.epsilon0 = Some(nfa.initial);
    let new_idx = nfa.states.len();
    nfa.states.push(new_state);
    nfa.initial = new_idx;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    // Add new state, old final's epsilon0 -> new state, new state becomes final
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

pub fn nfa_dump(_nfa: &Nfa) {
    // Optional debug dump; not needed for correctness.
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
    let mut buf = Vec::new();
    leb128_put(&mut buf, dfa_size);

    for dstate in dfa.states.iter() {
        let flags = ((dstate.accepting as u8) << 1) | (dstate.terminating as u8);
        buf.push(flags);
        let mut chr: usize = 0;
        while chr < 256 {
            let start = chr;
            while chr < 255 && dstate.transitions[chr] == dstate.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8); // run length
            leb128_put(&mut buf, dstate.transitions[chr] as i32);
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
            return Err("dfa_deserialize: out of bounds".to_string());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;

        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("dfa_deserialize: out of bounds".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            // do/while: repeat len+1 times
            let mut count = 0;
            while count <= len && chr < 256 {
                states[id].transitions[chr] = target;
                chr += 1;
                count += 1;
            }
        }
    }

    Ok((
        Dfa {
            states,
            initial: 0,
        },
        p,
    ))
}

pub fn dfa_dump(_dfa: &Dfa) {
    // Optional debug
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
        nfa.states.push(NState::new());
        let prev_final = nfa.final_;
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
    for state in nfa.states.iter_mut() {
        let orig = state.label;
        for c in 0u8..=255u8 {
            if orig.contains(c) {
                state.label.insert(to_lower(c));
                state.label.insert(to_upper(c));
            }
            if c == 255 {
                break;
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
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
    let mut result = vec![0u8; bs_size];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) && nfa.states[id].label.contains(chr) {
            if let Some(t) = nfa.states[id].target {
                epsilon_closure_into(nfa, t, &mut result);
            }
        }
    }
    result
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    // Powerset construction
    let mut dfa = Dfa::new();
    // initial: epsilon-closure of nfa.initial
    let initial_bs = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let mut accepting = bitset_test(&initial_bs, nfa.final_);
    if nfa.complemented {
        accepting = !accepting;
    }
    dfa.states.push(DState {
        transitions: [0usize; 256],
        accepting,
        terminating: false,
        bitset: initial_bs,
    });

    let mut idx = 0;
    while idx < dfa.states.len() {
        for chr_i in 0..256u32 {
            let chr = chr_i as u8;
            let new_bs = step_powerset(&nfa, &dfa.states[idx].bitset.clone(), chr);

            // find existing state with same bitset
            let mut found: Option<usize> = None;
            for (j, st) in dfa.states.iter().enumerate() {
                if st.bitset == new_bs {
                    found = Some(j);
                    break;
                }
            }

            let target_id = if let Some(j) = found {
                j
            } else {
                let mut acc = bitset_test(&new_bs, nfa.final_);
                if nfa.complemented {
                    acc = !acc;
                }
                let new_id = dfa.states.len();
                dfa.states.push(DState {
                    transitions: [0usize; 256],
                    accepting: acc,
                    terminating: false,
                    bitset: new_bs,
                });
                new_id
            };

            dfa.states[idx].transitions[chr_i as usize] = target_id;
        }
        idx += 1;
    }

    let _ = bs_size;

    // Minimize
    dfa_minimize(&mut dfa, nfa.complemented);

    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }
    let row_size = (n + 7) / 8;
    // distinguishable matrix: dis[i*row_size + j/8] bit j%8
    let mut dis = vec![0u8; n * row_size];
    let make_dis = |dis: &mut [u8], i: usize, j: usize, row_size: usize| {
        dis[i * row_size + j / 8] |= 1u8 << (j % 8);
        dis[j * row_size + i / 8] |= 1u8 << (i % 8);
    };
    let are_dis = |dis: &[u8], i: usize, j: usize, row_size: usize| -> bool {
        dis[i * row_size + j / 8] & (1u8 << (j % 8)) != 0
    };

    // initial: pairs with different accepting
    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                make_dis(&mut dis, i, j, row_size);
            }
        }
    }

    // iterate to fixed point
    loop {
        let mut done = true;
        for id1 in 0..n {
            for id2 in (id1 + 1)..n {
                if !are_dis(&dis, id1, id2, row_size) {
                    for chr in 0..256 {
                        let t1 = dfa.states[id1].transitions[chr];
                        let t2 = dfa.states[id2].transitions[chr];
                        if t1 != t2 && are_dis(&dis, t1, t2, row_size) {
                            make_dis(&mut dis, id1, id2, row_size);
                            done = false;
                            break;
                        }
                    }
                }
            }
        }
        if done {
            break;
        }
    }

    // Build merge map: for each state, find smallest representative
    let mut rep: Vec<usize> = (0..n).collect();
    for id1 in 0..n {
        if rep[id1] != id1 {
            continue;
        }
        for id2 in (id1 + 1)..n {
            if rep[id2] != id2 {
                continue;
            }
            if !are_dis(&dis, id1, id2, row_size) {
                rep[id2] = id1;
            }
        }
    }

    // Path compression (single level should be enough since rep[id1] == id1 above)
    for i in 0..n {
        let mut r = rep[i];
        while rep[r] != r {
            r = rep[r];
        }
        rep[i] = r;
    }

    // Build new state list keeping insertion order of representatives
    let mut new_idx: Vec<Option<usize>> = vec![None; n];
    let mut new_states: Vec<DState> = Vec::new();
    for i in 0..n {
        if rep[i] == i {
            let new_id = new_states.len();
            new_idx[i] = Some(new_id);
            new_states.push(dfa.states[i].clone());
        }
    }
    for i in 0..n {
        if rep[i] != i {
            new_idx[i] = new_idx[rep[i]];
        }
    }

    // Remap transitions
    for st in new_states.iter_mut() {
        for chr in 0..256 {
            let t = st.transitions[chr];
            st.transitions[chr] = new_idx[t].unwrap();
        }
    }

    // Determine initial: was 0 in dfa, now new_idx[0]
    let new_initial = new_idx[dfa.initial].unwrap();

    // Compute terminating: state where all transitions go to itself
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

    dfa.states = new_states;
    dfa.initial = new_initial;
}

fn find_or_create_dead(_states: &mut Vec<DState>) -> usize {
    // Not used in our implementation; provide a stub
    0
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
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

pub fn ltre_matches(dfa: &Dfa, input: &[u8]) -> bool {
    let mut state = dfa.initial;
    let mut i = 0;
    while !dfa.states[state].terminating && i < input.len() {
        state = dfa.states[state].transitions[input[i] as usize];
        i += 1;
    }
    dfa.states[state].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    // For simplicity, compile fully on first call
    if dfap.is_none() {
        *dfap = Some(ltre_compile(nfa.clone()));
    }
    let dfa = dfap.as_ref().unwrap();
    ltre_matches(dfa, input)
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();

    // Build NFA: initial, final, then dfa_size states (one per DFA state)
    // Then auxiliary states appended as needed.
    let mut nfa = Nfa {
        states: Vec::new(),
        initial: 0,
        final_: 1,
        complemented: false,
    };
    nfa.states.push(NState::new()); // initial = 0
    nfa.states.push(NState::new()); // final = 1

    // For each DFA state, allocate an NFA state
    let nstates_base = 2;
    for _ in 0..dfa_size {
        nfa.states.push(NState::new());
    }

    let nstate_for = |id: usize| -> usize { nstates_base + id };

    nfa.states[nfa.initial].epsilon1 = Some(nstate_for(dfa.initial));

    for (id, dstate) in dfa.states.iter().enumerate() {
        if dstate.accepting {
            nfa.states[nstate_for(id)].epsilon1 = Some(nfa.final_);
        }
    }

    // For each ds1, build labeled transitions for each unique target ds2.
    // Map labeled transitions through a binary tree of epsilon-states.
    for ds1_id in 0..dfa_size {
        // Collect groupings: for each distinct target id, the symset of chars
        let mut groups: Vec<(usize, SymSet)> = Vec::new();
        for ds2_id in 0..dfa_size {
            let mut ts = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[ds1_id].transitions[chr] == ds2_id {
                    ts.insert(chr as u8);
                    empty = false;
                }
            }
            if !empty {
                groups.push((ds2_id, ts));
            }
        }

        let mut free_state: Option<usize> = None;

        for (ds2_id, ts) in groups.into_iter() {
            let src;
            if free_state.is_none() {
                // First iteration: source is the nstate for ds1
                src = nstate_for(ds1_id);
                free_state = Some(src);
            } else {
                // Allocate a new state
                let new_state = nfa.states.len();
                nfa.states.push(NState::new());
                src = new_state;

                let f = free_state.unwrap();
                if nfa.states[f].epsilon1.is_none() {
                    nfa.states[f].epsilon1 = Some(new_state);
                    // Stay at f
                } else {
                    nfa.states[f].epsilon0 = Some(new_state);
                    free_state = Some(new_state);
                }
            }

            nfa.states[src].target = Some(nstate_for(ds2_id));
            nfa.states[src].label = ts;
        }
    }

    nfa
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    let dfa_size = dfa.states.len();
    let aux = dfa_size;
    let n = dfa_size + 1;

    // arrows[i][j]
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

    // Epsilon from aux -> initial
    arrows[aux][dfa.initial] = Arrow {
        label: Some(String::new()),
        prec: Prec::Symset,
    };

    for ds1_id in 0..dfa_size {
        if dfa.states[ds1_id].accepting {
            arrows[ds1_id][aux] = Arrow {
                label: Some(String::new()),
                prec: Prec::Symset,
            };
        }
        for ds2_id in 0..dfa_size {
            let mut ts = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[ds1_id].transitions[chr] == ds2_id {
                    ts.insert(chr as u8);
                    empty = false;
                }
            }
            if !empty {
                let fmt = symset_fmt(&ts);
                arrows[ds1_id][ds2_id] = Arrow {
                    label: Some(fmt),
                    prec: Prec::Symset,
                };
            }
        }
    }

    loop {
        // Find best fit: state with minimal positive degree (excluding aux)
        let mut best_fit: Option<usize> = None;
        let mut min_degree = i32::MAX;
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
            Some(b) => b,
            None => break,
        };

        // For each pair of inbound and outbound transitions
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

                if in_arr.label.is_none() || out_arr.label.is_none() {
                    continue;
                }

                let in_label = in_arr.label.clone().unwrap();
                let out_label = out_arr.label.clone().unwrap();

                // Build first/second
                let (first, second) = arrow_concat_helper(
                    &in_label,
                    in_arr.prec,
                    out_label.as_str(),
                    out_arr.prec,
                    &self_arr,
                );

                // bypass = first . second
                let bypass = combine_concat(&first, &second);

                // merged = existing | bypass
                let merged = combine_alt(&existing, &bypass);

                arrows[id1][id2] = merged;
            }
        }

        // Eliminate best_fit's transitions
        for id in 0..n {
            arrows[id][best_fit].label = None;
            arrows[best_fit][id].label = None;
        }
    }

    let regex = arrows[aux][aux].label.clone();
    regex.unwrap_or_else(|| "[]".to_string())
}

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
        if self.pos < self.chars.len() {
            Some(self.chars[self.pos])
        } else {
            None
        }
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
        self.next().ok_or_else(|| "unexpected EOF".to_string())
    }
}

fn parse_regex(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut re = parse_term(ctx)?;
    while let Some(c) = ctx.peek() {
        if c != b'|' && c != b'&' {
            break;
        }
        ctx.next();
        let intersect = c == b'&';
        let mut alt = parse_term(ctx)?;

        // De Morgan: a&b = ~(~a|~b)
        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        // Pad re's initial and alt's final, then combine
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Need to merge alt's states into re's namespace
        let n_re = re.states.len();
        let n_alt = nstate_count(&alt);
        let alt_states = alt.states;
        let alt_initial = alt.initial;
        let alt_final = alt.final_;

        let map = |i: usize| -> usize { i + n_re };
        let mut new_alt_states = alt_states;
        for s in new_alt_states.iter_mut() {
            s.target = s.target.map(map);
            s.epsilon0 = s.epsilon0.map(map);
            s.epsilon1 = s.epsilon1.map(map);
        }
        re.states.extend(new_alt_states);

        let alt_initial_remapped = alt_initial + n_re;
        let alt_final_remapped = alt_final + n_re;

        let re_initial = re.initial;
        let re_final = re.final_;
        re.states[re_initial].epsilon1 = Some(alt_initial_remapped);
        re.states[re_final].epsilon0 = Some(alt_final_remapped);
        re.final_ = alt_final_remapped;

        if intersect {
            re.complemented = !re.complemented;
        }
        let _ = n_alt;
    }
    Ok(re)
}

fn nstate_count(nfa: &Nfa) -> usize {
    nfa.states.len()
}

fn parse_term(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'~') {
        ctx.next();
        complement = true;
    }

    let mut term = Nfa::new_single();

    loop {
        match ctx.peek() {
            None => break,
            Some(c) if c == b')' || c == b'|' || c == b'&' => break,
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

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    match ctx.peek() {
        Some(b'*') => {
            ctx.next();
            nfa_uncomplement(&mut atom)?;
            // atom.final.epsilon1 = atom.initial
            let f = atom.final_;
            let i = atom.initial;
            atom.states[f].epsilon1 = Some(i);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            let new_init = atom.initial;
            let new_final = atom.final_;
            atom.states[new_init].epsilon1 = Some(new_final);
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
            // If atom.initial.epsilon1 is set, pad initial first
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
            // Parse min
            let mut had_min = true;
            let min = match parse_natural(ctx) {
                Ok(v) => v,
                Err(e) => {
                    if e == "natural number overflow" {
                        return Err(e);
                    }
                    had_min = false;
                    0
                }
            };
            let _ = had_min;

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
                return Err("misbounded quantifier".to_string());
            }

            let mut atoms = Nfa::new_single();

            // Build copies. Bound the upper limit to avoid pathological loops.
            let limit: u32 = if max_unbounded { min + 1 } else { max };
            for i in 0..limit {
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
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };
    nfa.states[0].target = Some(1);
    nfa.states[0].label = symset;
    Ok(nfa)
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte: u8 = 0;
    for _ in 0..2 {
        byte <<= 4;
        let chr = ctx.peek().ok_or_else(|| "expected hex digit".to_string())?;
        if is_digit(chr) {
            byte |= chr - b'0';
        } else if is_xdigit(chr) {
            byte |= to_lower(chr) - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.next();
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or_else(|| "unknown escape".to_string())?;
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
            // backtrack and error
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
    let start = ctx.pos;
    if ctx.peek() == Some(b'\\') {
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
        ctx.pos = start;
    }

    if ctx.peek() == Some(b'.') {
        ctx.next();
        let mut s = SymSet::empty();
        for c in 0u8..=255 {
            if c != b'\n' {
                s.insert(c);
            }
            if c == 255 {
                break;
            }
        }
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

    let saved = ctx.pos;
    if let Ok(s) = parse_shorthand(ctx) {
        let mut s = s;
        if complement {
            s.invert();
        }
        return Ok(s);
    }
    ctx.pos = saved;

    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut sym = SymSet::empty();
        while ctx.peek() != Some(b']') {
            if ctx.peek().is_none() {
                return Err("expected ']'".to_string());
            }
            let sub = parse_symset(ctx)?;
            union_inplace(&mut sym, &sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.next();
        if complement {
            sym.invert();
        }
        return Ok(sym);
    }

    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut sym = SymSet::full();
        while ctx.peek() != Some(b'>') {
            if ctx.peek().is_none() {
                return Err("expected '>'".to_string());
            }
            let sub = parse_symset(ctx)?;
            intersect_inplace(&mut sym, &sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.next();
        if complement {
            sym.invert();
        }
        return Ok(sym);
    }

    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.next();
        end = parse_symbol(ctx)?;
    }
    let mut sym = SymSet::empty();
    let mut chr = begin;
    let stop = end.wrapping_add(1);
    loop {
        sym.insert(chr);
        chr = chr.wrapping_add(1);
        if chr == stop {
            break;
        }
    }

    if complement {
        sym.invert();
    }
    Ok(sym)
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}

fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}

fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0u8..=255 {
        if is_digit(c) {
            s.insert(c);
        }
        if c == 255 {
            break;
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
    for c in 0u8..=255 {
        if is_space(c) {
            s.insert(c);
        }
        if c == 255 {
            break;
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
    for c in 0u8..=255 {
        if c == b'_' || is_alnum(c) {
            s.insert(c);
        }
        if c == 255 {
            break;
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
    let c = ctx.peek().ok_or_else(|| "expected natural number".to_string())?;
    if !is_digit(c) {
        return Err("expected natural number".to_string());
    }
    let mut n: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !is_digit(c) {
            break;
        }
        let d = (c - b'0') as u32;
        if n > u32::MAX / 10 {
            return Err("natural number overflow".to_string());
        }
        n = n.checked_mul(10).ok_or_else(|| "natural number overflow".to_string())?;
        n = n.checked_add(d).ok_or_else(|| "natural number overflow".to_string())?;
        ctx.next();
    }
    Ok(n)
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(v) = opt {
        *v += offset;
    }
}

// =========================
// ltre_decompile combine helpers
// =========================

fn arrow_concat_helper(
    in_label: &str,
    in_prec: Prec,
    out_label: &str,
    out_prec: Prec,
    self_arr: &Arrow,
) -> (Arrow, Arrow) {
    let first;
    let second;

    if self_arr.label.is_none() || self_arr.label.as_deref() == Some("") {
        // (in)[]*(out) == (in)()*(out) == (in)(out)
        first = Arrow {
            label: Some(in_label.to_string()),
            prec: in_prec,
        };
        second = Arrow {
            label: Some(out_label.to_string()),
            prec: out_prec,
        };
        return (first, second);
    }

    let self_label = self_arr.label.clone().unwrap();
    let self_prec = self_arr.prec;

    // Try "in_pre + self+ + out" merge
    if in_prec >= Prec::Concat
        && self_prec >= Prec::Concat
        && in_label.len() >= self_label.len()
        && in_label.ends_with(&self_label)
    {
        let diff = in_label.len() - self_label.len();
        let in_bytes = in_label.as_bytes();
        // Heuristics to avoid breaking apart symsets
        let mut nevermind = false;
        if diff >= 1 {
            let prev = in_bytes[diff - 1];
            if matches!(prev, b'^' | b'-' | b'\\')
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
            s.push_str(&self_label);
            if self_prec <= Prec::Quant {
                s.push(')');
            }
            s.push('+');
            first = Arrow {
                label: Some(s),
                prec: Prec::Concat,
            };
            second = Arrow {
                label: Some(out_label.to_string()),
                prec: out_prec,
            };
            return (first, second);
        }
    }

    // Try "in + self+ + out_post" merge
    if out_prec >= Prec::Concat
        && self_prec >= Prec::Concat
        && out_label.len() >= self_label.len()
        && out_label.starts_with(&self_label)
    {
        let diff = out_label.len() - self_label.len();
        let mut s = String::new();
        if self_prec <= Prec::Quant {
            s.push('(');
        }
        s.push_str(&self_label);
        if self_prec <= Prec::Quant {
            s.push(')');
        }
        s.push('+');
        if diff != 0 && out_prec < Prec::Concat {
            s.push('(');
        }
        // C uses memcpy(p, out.label + diff, diff). That looks wrong (should be
        // strlen-diff, not diff). Mirror C exactly: copy `diff` bytes starting at
        // `out.label + diff`. This would mean it copies from offset diff for
        // `diff` bytes (likely a bug in C, but we mirror to match decompile).
        // Actually inspecting C: it's `memcpy(p, out.label + diff, diff)` which
        // copies `diff` bytes from offset `diff` -> not strlen-diff. But that
        // would be incorrect unless out.label has length 2*diff. Let me keep
        // matching out_label[diff..] (the rest after self).
        s.push_str(&out_label[self_label.len()..]);
        if diff != 0 && out_prec < Prec::Concat {
            s.push(')');
        }
        first = Arrow {
            label: Some(in_label.to_string()),
            prec: in_prec,
        };
        second = Arrow {
            label: Some(s),
            prec: Prec::Concat,
        };
        return (first, second);
    }

    // Default: (in)(self)*(out)
    let mut s = String::new();
    if self_prec <= Prec::Quant {
        s.push('(');
    }
    s.push_str(&self_label);
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
    first = Arrow {
        label: Some(in_label.to_string()),
        prec: in_prec,
    };
    second = Arrow {
        label: Some(s),
        prec: Prec::Concat,
    };
    (first, second)
}

fn combine_concat(first: &Arrow, second: &Arrow) -> Arrow {
    let f = match &first.label {
        Some(s) => s.as_str(),
        None => return Arrow { label: None, prec: Prec::Symset },
    };
    let s = match &second.label {
        Some(s) => s.as_str(),
        None => return Arrow { label: None, prec: Prec::Symset },
    };
    if f.is_empty() {
        return Arrow {
            label: Some(s.to_string()),
            prec: second.prec,
        };
    }
    if s.is_empty() {
        return Arrow {
            label: Some(f.to_string()),
            prec: first.prec,
        };
    }
    let mut out = String::new();
    if first.prec < Prec::Concat {
        out.push('(');
    }
    out.push_str(f);
    if first.prec < Prec::Concat {
        out.push(')');
    }
    if second.prec < Prec::Concat {
        out.push('(');
    }
    out.push_str(s);
    if second.prec < Prec::Concat {
        out.push(')');
    }
    Arrow {
        label: Some(out),
        prec: Prec::Concat,
    }
}

fn combine_alt(existing: &Arrow, bypass: &Arrow) -> Arrow {
    if bypass.label.is_none() {
        return existing.clone();
    }
    if existing.label.is_none() {
        return bypass.clone();
    }
    let e = existing.label.as_deref().unwrap();
    let b = bypass.label.as_deref().unwrap();
    if e.is_empty() {
        // (bypass)?
        let mut s = String::new();
        if bypass.prec <= Prec::Quant {
            s.push('(');
        }
        s.push_str(b);
        if bypass.prec <= Prec::Quant {
            s.push(')');
        }
        s.push('?');
        return Arrow {
            label: Some(s),
            prec: Prec::Quant,
        };
    }
    let mut s = String::new();
    s.push_str(e);
    s.push('|');
    s.push_str(b);
    Arrow {
        label: Some(s),
        prec: Prec::Alt,
    }
}

// Override placeholder build_first_second by re-providing a dispatcher that
// is actually called from ltre_decompile. We do this by wrapping above: the
// `build_first_second` function is unused; ltre_decompile calls
// arrow_concat_helper directly. To keep the original signature alive but
// unused, we leave it unreachable.
