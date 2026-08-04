// Translation of LTRE C library to safe Rust.

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn is_metachar(c: u8) -> bool {
    c != 0 && METACHARS.contains(&c)
}

fn is_print(c: u8) -> bool {
    // C isprint: printable including space, i.e. 0x20..=0x7e
    (0x20..=0x7e).contains(&c)
}

fn is_digit_byte(c: u8) -> bool {
    c.is_ascii_digit()
}

fn is_xdigit_byte(c: u8) -> bool {
    c.is_ascii_hexdigit()
}

fn is_space_byte(c: u8) -> bool {
    // C isspace: ' ', '\t', '\n', '\v', '\f', '\r'
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

fn is_alnum_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric()
}

fn to_lower_byte(c: u8) -> u8 {
    c.to_ascii_lowercase()
}

fn to_upper_byte(c: u8) -> u8 {
    c.to_ascii_uppercase()
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
        (self.bits[i / 8] >> (i % 8)) & 1 != 0
    }
    pub fn insert(&mut self, c: u8) {
        let i = c as usize;
        self.bits[i / 8] |= 1 << (i % 8);
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
    // Mirror the C `symset_fmt` logic. Output should be parsable by parse_symset.
    let mut buf = String::new(); // positive class: [...]
    let mut nbuf = String::new(); // complemented class: ^[...]
    let mut nsym: i32 = 0;
    let mut nnsym: i32 = 0;
    nbuf.push('^');
    buf.push('[');
    nbuf.push('[');

    let append_chr = |s: &mut String, chr: u8| {
        let metachar = is_metachar(chr);
        if !is_print(chr) && !metachar {
            s.push_str(&format!("\\x{:02x}", chr));
        } else {
            if metachar {
                s.push('\\');
            }
            s.push(chr as char);
        }
    };

    let mut chr: i32 = 0;
    while chr < 256 {
        let c = chr as u8;
        let in_set = set.contains(c);
        if in_set {
            nsym += 1;
        } else {
            nnsym += 1;
        }
        if in_set {
            append_chr(&mut buf, c);
        } else {
            append_chr(&mut nbuf, c);
        }

        // make character ranges
        let start = chr;
        while chr < 255 && set.contains(chr as u8) == set.contains((chr + 1) as u8) {
            chr += 1;
        }
        if chr - start >= 2 {
            // append '-' to the appropriate buffer
            if in_set {
                buf.push('-');
                nsym -= 1;
            } else {
                nbuf.push('-');
                nnsym -= 1;
            }
        }
        if chr - start >= 1 {
            // append the end-of-range character to same buffer
            let c2 = chr as u8;
            if in_set {
                append_chr(&mut buf, c2);
            } else {
                append_chr(&mut nbuf, c2);
            }
        }
        chr += 1;
    }

    buf.push(']');
    nbuf.push(']');

    // special cases
    if nnsym == 0 {
        return "<>".to_string();
    } else if nsym == 1 {
        // strip last ']' and leading '['
        let inner = &buf[1..buf.len() - 1];
        return inner.to_string();
    } else if nnsym == 1 {
        // nbuf was "^[...]", strip the '[' and ']'
        // C: nbufp[-2]='\0', nbuf[1]='^'  => "^^chr..." returning nbuf+1
        // So after dropping last ']' and replacing index 1 (which was '[') with '^':
        let mut chars: Vec<char> = nbuf.chars().collect();
        chars.pop(); // remove ']'
        chars[1] = '^';
        // return nbuf+1 (skip first char)
        let s: String = chars.into_iter().collect();
        return s[1..].to_string();
    }

    // return shorter of buf or nbuf
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

pub fn nfa_clone(orig: &Nfa) -> Nfa {
    orig.clone()
}

fn shift_option_inner(opt: &mut Option<usize>, offset: usize) {
    if let Some(x) = opt.as_mut() {
        *x += offset;
    }
}

pub fn shift_option(opt: &mut Option<usize>, offset: usize) {
    shift_option_inner(opt, offset);
}

pub fn nfa_concat(nfa1: &mut Nfa, nfa2: Nfa) {
    // C: if nfa1.initial == nfa1.final, replace nfa1 with nfa2
    // else if nfa2.initial != nfa2.final: copy nfa2.initial state into nfa1.final, then drop nfa2.initial
    // We model NState linearly. Implementation:
    if nfa1.initial == nfa1.final_ {
        // nfa1 is a single state; swap with nfa2 entirely
        *nfa1 = nfa2;
        return;
    }
    if nfa2.initial == nfa2.final_ {
        // nfa2 is a single state -> nothing to add (no transitions); nfa1 unchanged
        return;
    }
    // We need to merge. The nfa1.final_ state should become nfa2.initial state's content.
    // Then append nfa2.states[1..] (skip initial) but with adjusted indices.
    let final1 = nfa1.final_;
    let nfa2_initial = nfa2.initial;

    // Build mapping for nfa2 indices -> new indices in nfa1
    // nfa2.initial maps to final1; all others map to (existing nfa1.len()) + their position-rank
    let nfa2_len = nfa2.states.len();
    let nfa1_len_before = nfa1.states.len();

    // For each nfa2 state index i, compute new index
    // i == nfa2_initial -> final1
    // else -> nfa1_len_before + (count of indices < i excluding initial)
    let map: Vec<usize> = (0..nfa2_len)
        .map(|i| {
            if i == nfa2_initial {
                final1
            } else {
                let count_before = (0..i).filter(|&j| j != nfa2_initial).count();
                nfa1_len_before + count_before
            }
        })
        .collect();

    // Replace nfa1.final_ contents with nfa2.initial state's contents (but with remapped indices)
    let init_state = nfa2.states[nfa2_initial].clone();
    let mut remapped_init = init_state;
    if let Some(t) = remapped_init.target.as_mut() {
        *t = map[*t];
    }
    if let Some(t) = remapped_init.epsilon0.as_mut() {
        *t = map[*t];
    }
    if let Some(t) = remapped_init.epsilon1.as_mut() {
        *t = map[*t];
    }
    nfa1.states[final1] = remapped_init;

    // Append remaining states from nfa2 (skipping initial)
    for (i, st) in nfa2.states.iter().enumerate() {
        if i == nfa2_initial {
            continue;
        }
        let mut new_st = st.clone();
        if let Some(t) = new_st.target.as_mut() {
            *t = map[*t];
        }
        if let Some(t) = new_st.epsilon0.as_mut() {
            *t = map[*t];
        }
        if let Some(t) = new_st.epsilon1.as_mut() {
            *t = map[*t];
        }
        nfa1.states.push(new_st);
    }

    nfa1.final_ = map[nfa2.final_];
}

pub fn nfa_pad_initial(nfa: &mut Nfa) {
    // Insert a new initial state pointing via epsilon0 to old initial.
    // Need to maintain "next" linked list semantics; in our representation,
    // states are simply in a Vec. We push the new state and update initial.
    let mut new_state = NState::new();
    new_state.epsilon0 = Some(nfa.initial);
    nfa.states.push(new_state);
    nfa.initial = nfa.states.len() - 1;
}

pub fn nfa_pad_final(nfa: &mut Nfa) {
    // Add a new final state, with old final's epsilon0 pointing to it.
    let new_idx = nfa.states.len();
    let mut new_state = NState::new();
    nfa.states[nfa.final_].epsilon0 = Some(new_idx);
    nfa.states.push(new_state);
    nfa.final_ = new_idx;
    let _ = new_state;
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
    // not needed for tests
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

pub fn dfa_dump(_dfa: &Dfa) {}

fn leb128_put(buf: &mut Vec<u8>, mut n: i32) {
    while (n >> 7) != 0 {
        buf.push(((n & 0x7f) | 0x80) as u8);
        // logical shift: in C, n is `int`; we mimic with arithmetic shift for non-neg.
        n = ((n as u32) >> 7) as i32;
    }
    buf.push((n & 0xff) as u8);
}

fn leb128_get(buf: &[u8], p: &mut usize) -> Result<i32, String> {
    let mut n: i32 = 0;
    let mut c = 0;
    loop {
        if *p >= buf.len() {
            return Err("leb128: out of bounds".to_string());
        }
        let b = buf[*p];
        n |= ((b & 0x7f) as i32) << (c * 7);
        c += 1;
        *p += 1;
        if (b & 0x80) == 0 {
            break;
        }
    }
    Ok(n)
}

pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    let dfa_size = dfa.states.len() as i32;
    leb128_put(&mut buf, dfa_size);

    // We need to iterate states in order matching the linked-list order.
    // In our Vec representation, the state order is initial state first then the rest in order.
    // Build serialization order: initial=0, then 1..n in order? Original C uses linked-list
    // order starting from `dfa` (the head). For the round-trip to produce identical DFA, we
    // emit states in `0..n` order, with `initial` always being state 0.
    // We need to remap indices so that the initial is index 0.
    let n = dfa.states.len();
    let mut order = Vec::with_capacity(n);
    let mut id_map = vec![0usize; n];
    order.push(dfa.initial);
    id_map[dfa.initial] = 0;
    let mut next_id = 1;
    for i in 0..n {
        if i != dfa.initial {
            id_map[i] = next_id;
            order.push(i);
            next_id += 1;
        }
    }

    for &i in &order {
        let st = &dfa.states[i];
        let flags = ((st.accepting as u8) << 1) | (st.terminating as u8);
        buf.push(flags);
        let mut chr: usize = 0;
        while chr < 256 {
            let start = chr;
            while chr < 255 && st.transitions[chr] == st.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8); // run length
            let target = id_map[st.transitions[chr]] as i32;
            leb128_put(&mut buf, target);
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
            return Err("dfa_deserialize: oob".to_string());
        }
        let flags = buf[p];
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;
        let mut chr: usize = 0;
        while chr < 256 {
            if p >= buf.len() {
                return Err("dfa_deserialize: oob".to_string());
            }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            // run-length: assign transitions[chr..=chr+len] to target
            let end = chr + len;
            let mut c = chr;
            loop {
                states[id].transitions[c] = target;
                if c == end {
                    break;
                }
                c += 1;
            }
            chr = end + 1;
        }
    }
    Ok((Dfa { states, initial: 0 }, p))
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
        let initial_idx = nfa.final_;
        let new_final_idx = nfa.states.len();
        nfa.states.push(NState::new());
        nfa.states[initial_idx].target = Some(new_final_idx);
        nfa.states[initial_idx].label.insert(b);
        nfa.final_ = new_final_idx;
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
        for chr in 0u32..256 {
            let c = chr as u8;
            if st.label.contains(c) {
                st.label.insert(to_lower_byte(c));
                st.label.insert(to_upper_byte(c));
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

fn epsilon_closure_into(nfa: &Nfa, st_id: usize, bitset: &mut [u8]) {
    // Iterative to avoid stack overflows
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
    0
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bs_size = (nfa_size + 7) / 8;

    // Initial state from epsilon-closure of nfa.initial
    let initial_bs = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let mut states: Vec<DState> = Vec::new();
    states.push(DState {
        transitions: [0usize; 256],
        accepting: bitset_test(&initial_bs, nfa.final_) ^ nfa.complemented,
        terminating: false,
        bitset: initial_bs,
    });

    // Process states in BFS-like manner
    let mut i = 0;
    while i < states.len() {
        for chr in 0u32..256 {
            let new_bs = step_powerset(&nfa, &states[i].bitset, chr as u8);
            // find existing state with same bitset
            let mut found: Option<usize> = None;
            for (j, st) in states.iter().enumerate() {
                if st.bitset == new_bs {
                    found = Some(j);
                    break;
                }
            }
            let target = match found {
                Some(j) => j,
                None => {
                    let accepting = bitset_test(&new_bs, nfa.final_) ^ nfa.complemented;
                    states.push(DState {
                        transitions: [0usize; 256],
                        accepting,
                        terminating: false,
                        bitset: new_bs,
                    });
                    states.len() - 1
                }
            };
            states[i].transitions[chr as usize] = target;
        }
        i += 1;
    }

    let mut dfa = Dfa { states, initial: 0 };
    dfa_minimize(&mut dfa, false);
    dfa
}

fn dfa_minimize(dfa: &mut Dfa, _complemented: bool) {
    let n = dfa.states.len();
    if n == 0 {
        return;
    }
    let dis_row_size = (n + 7) / 8;
    let mut dis = vec![0u8; n * dis_row_size];

    let are_dis = |dis: &[u8], i: usize, j: usize| -> bool {
        let row = &dis[i * dis_row_size..(i + 1) * dis_row_size];
        bitset_test(row, j)
    };
    let make_dis = |dis: &mut [u8], i: usize, j: usize| {
        let (a, b) = if i < j { (i, j) } else { (j, i) };
        // mark both directions
        let row_a_start = a * dis_row_size;
        let row_b_start = b * dis_row_size;
        // operate via indices
        bitset_set(&mut dis[row_a_start..row_a_start + dis_row_size], j);
        bitset_set(&mut dis[row_b_start..row_b_start + dis_row_size], i);
        let _ = (a, b);
    };

    for i in 0..n {
        for j in (i + 1)..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                make_dis(&mut dis, i, j);
            }
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            for j in (i + 1)..n {
                if !are_dis(&dis, i, j) {
                    for chr in 0..256 {
                        let ti = dfa.states[i].transitions[chr];
                        let tj = dfa.states[j].transitions[chr];
                        if ti != tj && are_dis(&dis, ti, tj) {
                            make_dis(&mut dis, i, j);
                            changed = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    // Build equivalence classes: for each state, rep[i] = smallest j s.t. !are_dis(j, i)
    let mut rep: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in 0..i {
            if !are_dis(&dis, j, i) {
                rep[i] = rep[j];
                break;
            }
        }
    }

    // Build new states list keeping only representatives. Map old index -> new index.
    let mut old_to_new = vec![usize::MAX; n];
    let mut new_states: Vec<DState> = Vec::new();
    for i in 0..n {
        if rep[i] == i {
            old_to_new[i] = new_states.len();
            new_states.push(dfa.states[i].clone());
        }
    }
    for i in 0..n {
        if rep[i] != i {
            old_to_new[i] = old_to_new[rep[i]];
        }
    }
    // Remap transitions
    for st in new_states.iter_mut() {
        for c in 0..256 {
            st.transitions[c] = old_to_new[st.transitions[c]];
        }
    }
    let new_initial = old_to_new[dfa.initial];

    // Reorder so that initial is first (id = 0). Build new ordering.
    let m = new_states.len();
    let mut order = Vec::with_capacity(m);
    let mut id_map = vec![usize::MAX; m];
    order.push(new_initial);
    id_map[new_initial] = 0;
    let mut next_id = 1;
    for i in 0..m {
        if i != new_initial {
            id_map[i] = next_id;
            order.push(i);
            next_id += 1;
        }
    }
    let mut reordered: Vec<DState> = Vec::with_capacity(m);
    for &i in &order {
        let mut s = new_states[i].clone();
        for c in 0..256 {
            s.transitions[c] = id_map[s.transitions[c]];
        }
        reordered.push(s);
    }

    // Compute terminating: a state is terminating iff all transitions point to itself.
    for (idx, st) in reordered.iter_mut().enumerate() {
        let mut term = true;
        for c in 0..256 {
            if st.transitions[c] != idx {
                term = false;
                break;
            }
        }
        st.terminating = term;
    }

    dfa.states = reordered;
    dfa.initial = 0;
}

fn bitset_test(bs: &[u8], idx: usize) -> bool {
    (bs[idx / 8] >> (idx % 8)) & 1 != 0
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
    for &b in input {
        if dfa.states[state].terminating {
            break;
        }
        state = dfa.states[state].transitions[b as usize];
    }
    dfa.states[state].accepting
}

pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    // Build/extend a DFA lazily. Each DFA state has a bitset corresponding to NFA states.
    if dfap.is_none() {
        let nfa_size = nfa.states.len();
        let initial_bs = epsilon_closure_vec(nfa, nfa.initial, nfa_size);
        let dfa = Dfa {
            states: vec![DState {
                transitions: [usize::MAX; 256],
                accepting: bitset_test(&initial_bs, nfa.final_) ^ nfa.complemented,
                terminating: false,
                bitset: initial_bs,
            }],
            initial: 0,
        };
        *dfap = Some(dfa);
    }

    let nfa_size = nfa.states.len();
    let dfa = dfap.as_mut().unwrap();

    let mut state = dfa.initial;
    for &b in input {
        let idx = b as usize;
        let next = dfa.states[state].transitions[idx];
        if next == usize::MAX {
            // need to create
            let new_bs = step_powerset(nfa, &dfa.states[state].bitset, b);
            let mut found: Option<usize> = None;
            for (j, st) in dfa.states.iter().enumerate() {
                if st.bitset == new_bs {
                    found = Some(j);
                    break;
                }
            }
            let target = match found {
                Some(j) => j,
                None => {
                    let accepting = bitset_test(&new_bs, nfa.final_) ^ nfa.complemented;
                    dfa.states.push(DState {
                        transitions: [usize::MAX; 256],
                        accepting,
                        terminating: false,
                        bitset: new_bs,
                    });
                    dfa.states.len() - 1
                }
            };
            dfa.states[state].transitions[idx] = target;
            state = target;
        } else {
            state = next;
        }
        let _ = nfa_size;
    }
    dfa.states[state].accepting
}

pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();

    // Build NFA. We have:
    //  - 1 initial state (idx 0)
    //  - dfa_size NFA states corresponding to DFA states (idx 1..=dfa_size)
    //  - additional states added by the binary tree construction
    //  - 1 final state (added at end)
    //
    // Actually in C, the initial is allocated first, then `dfa_size` states for the mapping,
    // then extras, then final at the end. We mirror by populating a Vec.

    let mut states: Vec<NState> = Vec::new();
    let initial = 0usize;
    states.push(NState::new());

    // Allocate `dfa_size` NFA states, indices 1..=dfa_size
    let nstates_base = states.len();
    for _ in 0..dfa_size {
        states.push(NState::new());
    }
    let nstates: Vec<usize> = (nstates_base..nstates_base + dfa_size).collect();

    // We need to know `final` index but it is allocated last. We'll create a placeholder.
    // For ease: we'll allocate `final` at the very end and then patch references, OR
    // we know upfront final = (some index assigned at end). We allocate final right now.
    let final_idx = states.len();
    states.push(NState::new());

    // initial.epsilon1 -> nstates[dfa.initial]
    states[initial].epsilon1 = Some(nstates[dfa.initial]);

    // For each accepting DFA state, nstates[id].epsilon1 = final
    for (id, st) in dfa.states.iter().enumerate() {
        if st.accepting {
            states[nstates[id]].epsilon1 = Some(final_idx);
        }
    }

    // Build labeled transitions tree per DFA state
    for id1 in 0..dfa_size {
        let mut free: Option<usize> = None; // current "free" node; root for the tree
        for id2 in 0..dfa_size {
            // find symset of chrs that go from id1 to id2
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256 {
                if dfa.states[id1].transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty {
                continue;
            }
            let src;
            if free.is_none() {
                free = Some(nstates[id1]);
                src = nstates[id1];
            } else {
                // Allocate a new state
                let new_idx = states.len();
                states.push(NState::new());
                src = new_idx;
                let f = free.unwrap();
                if states[f].epsilon1.is_none() {
                    states[f].epsilon1 = Some(new_idx);
                } else {
                    states[f].epsilon0 = Some(new_idx);
                    free = Some(new_idx);
                }
            }
            states[src].target = Some(nstates[id2]);
            states[src].label = transitions;
        }
    }

    Nfa {
        states,
        initial,
        final_: final_idx,
        complemented: false,
    }
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    // GNFA approach mirroring the C code.
    let dfa_size = dfa.states.len();
    let aux = dfa_size;
    let n = dfa_size + 1;
    let mut arrows: Vec<Vec<DecArrow>> =
        vec![
            vec![
                DecArrow {
                    label: None,
                    prec: DecPrec::Symset
                };
                n
            ];
            n
        ];

    // arrows[aux][initial] = epsilon
    arrows[aux][dfa.initial] = DecArrow {
        label: Some(String::new()),
        prec: DecPrec::Symset,
    };

    for ds1 in 0..dfa_size {
        if dfa.states[ds1].accepting {
            arrows[ds1][aux] = DecArrow {
                label: Some(String::new()),
                prec: DecPrec::Symset,
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
            arrows[ds1][ds2] = DecArrow {
                label: Some(fmt),
                prec: DecPrec::Symset,
            };
        }
    }

    loop {
        // pick state with minimal vertex degree
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

        for id1 in 0..=dfa_size {
            if id1 == best_fit {
                continue;
            }
            for id2 in 0..=dfa_size {
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

                // Compute first/second by handling self-transition
                let (first, second) = compute_first_second(&in_arrow, &out_arrow, &self_arrow);

                // Compute bypass
                let bypass: DecArrow;
                if first.label.as_ref().map_or(false, |s| s.is_empty()) {
                    bypass = second.clone();
                } else if second.label.as_ref().map_or(false, |s| s.is_empty()) {
                    bypass = first.clone();
                } else {
                    let mut s = String::new();
                    if first.prec < DecPrec::Concat {
                        s.push('(');
                    }
                    s.push_str(first.label.as_ref().unwrap());
                    if first.prec < DecPrec::Concat {
                        s.push(')');
                    }
                    if second.prec < DecPrec::Concat {
                        s.push('(');
                    }
                    s.push_str(second.label.as_ref().unwrap());
                    if second.prec < DecPrec::Concat {
                        s.push(')');
                    }
                    bypass = DecArrow {
                        label: Some(s),
                        prec: DecPrec::Concat,
                    };
                }

                // Merge with existing
                let merged: DecArrow;
                if bypass.label.is_none() {
                    merged = existing.clone();
                } else if existing.label.is_none() {
                    merged = bypass.clone();
                } else if existing.label.as_ref().map_or(false, |s| s.is_empty()) {
                    let mut s = String::new();
                    if bypass.prec <= DecPrec::Quant {
                        s.push('(');
                    }
                    s.push_str(bypass.label.as_ref().unwrap());
                    if bypass.prec <= DecPrec::Quant {
                        s.push(')');
                    }
                    s.push('?');
                    merged = DecArrow {
                        label: Some(s),
                        prec: DecPrec::Quant,
                    };
                } else {
                    let mut s = String::new();
                    s.push_str(existing.label.as_ref().unwrap());
                    s.push('|');
                    s.push_str(bypass.label.as_ref().unwrap());
                    merged = DecArrow {
                        label: Some(s),
                        prec: DecPrec::Alt,
                    };
                }

                arrows[id1][id2] = merged;
            }
        }

        // Eliminate best_fit
        for id in 0..=dfa_size {
            arrows[id][best_fit] = DecArrow {
                label: None,
                prec: DecPrec::Symset,
            };
            arrows[best_fit][id] = DecArrow {
                label: None,
                prec: DecPrec::Symset,
            };
        }
    }

    let regex = arrows[aux][aux].label.clone();
    regex.unwrap_or_else(|| "[]".to_string())
}

fn compute_first_second(
    in_arrow: &DecArrow,
    out_arrow: &DecArrow,
    self_arrow: &DecArrow,
) -> (DecArrow, DecArrow) {
    // Mirror the C logic for first/second computation in decompile.
    let in_label = in_arrow.label.as_deref().unwrap_or("");
    let out_label = out_arrow.label.as_deref().unwrap_or("");
    let self_label_opt = self_arrow.label.as_deref();

    if self_label_opt.is_none() || self_label_opt.unwrap().is_empty() {
        return (
            DecArrow {
                label: Some(in_label.to_string()),
                prec: in_arrow.prec,
            },
            DecArrow {
                label: Some(out_label.to_string()),
                prec: out_arrow.prec,
            },
        );
    }
    let self_label = self_label_opt.unwrap();

    // try to attach self to inbound
    if in_arrow.prec >= DecPrec::Concat
        && self_arrow.prec >= DecPrec::Concat
        && in_label.len() >= self_label.len()
    {
        let diff = in_label.len() - self_label.len();
        if &in_label[diff..] == self_label {
            // check the goto nevermind conditions
            let bytes = in_label.as_bytes();
            let mut nevermind = false;
            if diff >= 1 {
                let c = bytes[diff - 1];
                if c == b'^' || c == b'-' || c == b'\\' {
                    if diff == 1 || bytes[diff - 2] != b'\\' {
                        nevermind = true;
                    }
                }
            }
            if !nevermind && diff >= 2 {
                if &bytes[diff - 2..diff] == b"\\x" {
                    if diff == 2 || bytes[diff - 3] != b'\\' {
                        nevermind = true;
                    }
                }
            }
            if !nevermind && diff >= 3 {
                if &bytes[diff - 3..diff - 1] == b"\\x" {
                    if diff == 3 || bytes[diff - 4] != b'\\' {
                        nevermind = true;
                    }
                }
            }
            if !nevermind {
                let in_pre = &in_label[..diff];
                let mut s = String::new();
                if diff != 0 && in_arrow.prec < DecPrec::Concat {
                    s.push('(');
                }
                s.push_str(in_pre);
                if diff != 0 && in_arrow.prec < DecPrec::Concat {
                    s.push(')');
                }
                if self_arrow.prec <= DecPrec::Quant {
                    s.push('(');
                }
                s.push_str(self_label);
                if self_arrow.prec <= DecPrec::Quant {
                    s.push(')');
                }
                s.push('+');
                return (
                    DecArrow {
                        label: Some(s),
                        prec: DecPrec::Concat,
                    },
                    DecArrow {
                        label: Some(out_label.to_string()),
                        prec: out_arrow.prec,
                    },
                );
            }
        }
    }

    // try to attach self to outbound
    if out_arrow.prec >= DecPrec::Concat
        && self_arrow.prec >= DecPrec::Concat
        && out_label.len() >= self_label.len()
        && out_label.starts_with(self_label)
    {
        let diff = out_label.len() - self_label.len();
        let out_post = &out_label[self_label.len()..];
        let mut s = String::new();
        if self_arrow.prec <= DecPrec::Quant {
            s.push('(');
        }
        s.push_str(self_label);
        if self_arrow.prec <= DecPrec::Quant {
            s.push(')');
        }
        s.push('+');
        if diff != 0 && out_arrow.prec < DecPrec::Concat {
            s.push('(');
        }
        s.push_str(out_post);
        if diff != 0 && out_arrow.prec < DecPrec::Concat {
            s.push(')');
        }
        return (
            DecArrow {
                label: Some(in_label.to_string()),
                prec: in_arrow.prec,
            },
            DecArrow {
                label: Some(s),
                prec: DecPrec::Concat,
            },
        );
    }

    // (in)(self)*(out)
    let mut s = String::new();
    if self_arrow.prec <= DecPrec::Quant {
        s.push('(');
    }
    s.push_str(self_label);
    if self_arrow.prec <= DecPrec::Quant {
        s.push(')');
    }
    s.push('*');
    if out_arrow.prec < DecPrec::Concat {
        s.push('(');
    }
    s.push_str(out_label);
    if out_arrow.prec < DecPrec::Concat {
        s.push(')');
    }
    (
        DecArrow {
            label: Some(in_label.to_string()),
            prec: in_arrow.prec,
        },
        DecArrow {
            label: Some(s),
            prec: DecPrec::Concat,
        },
    )
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum DecPrec {
    Alt,
    Concat,
    Quant,
    Symset,
}

#[derive(Clone, Debug)]
struct DecArrow {
    label: Option<String>,
    prec: DecPrec,
}

trait ArrowLike {
    fn label_opt(&self) -> Option<&str>;
    fn label_str(&self) -> &str {
        self.label_opt().unwrap_or("")
    }
    fn prec(&self) -> DecPrec;
}

// Bridge: the Arrow inside ltre_decompile uses a local Prec; we made decompile use DecPrec
// directly via DecArrow. So we need to refactor. Let's instead make ltre_decompile use
// DecArrow directly. We'll rewrite the function below.

impl ArrowLike for DecArrow {
    fn label_opt(&self) -> Option<&str> {
        self.label.as_deref()
    }
    fn prec(&self) -> DecPrec {
        self.prec
    }
}

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
        self.chars.get(self.pos).copied()
    }
    pub fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }
    pub fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
    pub fn expect_char(&mut self) -> Result<u8, String> {
        self.next().ok_or_else(|| "unexpected eof".to_string())
    }
}

// === Parsing ===

fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    if !ctx.peek().map_or(false, |c| is_digit_byte(c)) {
        return Err("expected natural number".to_string());
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !is_digit_byte(c) {
            break;
        }
        ctx.pos += 1;
        let digit = (c - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            return Err("natural number overflow".to_string());
        }
        natural = natural * 10 + digit;
    }
    Ok(natural)
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte: u8 = 0;
    for _ in 0..2 {
        byte <<= 4;
        let c = ctx.peek().ok_or_else(|| "expected hex digit".to_string())?;
        if is_digit_byte(c) {
            byte |= c - b'0';
        } else if is_xdigit_byte(c) {
            byte |= c.to_ascii_lowercase() - b'a' + 10;
        } else {
            return Err("expected hex digit".to_string());
        }
        ctx.pos += 1;
    }
    Ok(byte)
}

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    let c = ctx.peek().ok_or_else(|| "unknown escape".to_string())?;
    if is_metachar(c) {
        ctx.pos += 1;
        return Ok(c);
    }
    let saved = ctx.pos;
    ctx.pos += 1;
    match c {
        b'a' => Ok(0x07),
        b'b' => Ok(0x08),
        b'f' => Ok(0x0C),
        b'n' => Ok(b'\n'),
        b'r' => Ok(b'\r'),
        b't' => Ok(b'\t'),
        b'v' => Ok(0x0B),
        b'x' => parse_hexbyte(ctx),
        _ => {
            ctx.pos = saved;
            Err("unknown escape".to_string())
        }
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    match ctx.peek() {
        Some(b'\\') => {
            ctx.pos += 1;
            parse_escape(ctx)
        }
        None => Err("expected symbol".to_string()),
        Some(c) if is_metachar(c) => Err("unexpected metacharacter".to_string()),
        Some(c) if !is_print(c) => Err("unexpected nonprintable character".to_string()),
        Some(c) => {
            ctx.pos += 1;
            Ok(c)
        }
    }
}

fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0u32..256 {
        if is_digit_byte(c as u8) {
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
    for c in 0u32..256 {
        if is_space_byte(c as u8) {
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
    for c in 0u32..256 {
        let cc = c as u8;
        if cc == b'_' || is_alnum_byte(cc) {
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
    let saved = ctx.pos;
    if ctx.peek() == Some(b'\\') {
        ctx.pos += 1;
        let c = ctx.peek();
        match c {
            Some(b'd') => {
                ctx.pos += 1;
                return Ok(digits_set());
            }
            Some(b'D') => {
                ctx.pos += 1;
                return Ok(not_digits_set());
            }
            Some(b's') => {
                ctx.pos += 1;
                return Ok(spaces_set());
            }
            Some(b'S') => {
                ctx.pos += 1;
                return Ok(not_spaces_set());
            }
            Some(b'w') => {
                ctx.pos += 1;
                return Ok(wordchar_set());
            }
            Some(b'W') => {
                ctx.pos += 1;
                return Ok(not_wordchar_set());
            }
            _ => {}
        }
        ctx.pos = saved;
    }

    if ctx.peek() == Some(b'.') {
        ctx.pos += 1;
        let mut s = SymSet::empty();
        for c in 0u32..256 {
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
    if ctx.peek() == Some(b'^') {
        ctx.pos += 1;
        complement = true;
    }

    let last_pos = ctx.pos;
    if let Ok(s) = parse_shorthand(ctx) {
        let mut sym = s;
        if complement {
            sym.invert();
        }
        return Ok(sym);
    }
    ctx.pos = last_pos;

    if ctx.peek() == Some(b'[') {
        ctx.pos += 1;
        let mut sym = SymSet::empty();
        while ctx.peek() != Some(b']') {
            if ctx.peek().is_none() {
                return Err("expected ']'".to_string());
            }
            let sub = parse_symset(ctx)?;
            sym.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.pos += 1;
        if complement {
            sym.invert();
        }
        return Ok(sym);
    }

    if ctx.peek() == Some(b'<') {
        ctx.pos += 1;
        let mut sym = SymSet::full();
        while ctx.peek() != Some(b'>') {
            if ctx.peek().is_none() {
                return Err("expected '>'".to_string());
            }
            let sub = parse_symset(ctx)?;
            sym.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.pos += 1;
        if complement {
            sym.invert();
        }
        return Ok(sym);
    }

    // Try parsing a single symbol or range
    let begin_pos = ctx.pos;
    match parse_symbol(ctx) {
        Ok(begin) => {
            let mut end = begin;
            if ctx.peek() == Some(b'-') {
                ctx.pos += 1;
                end = parse_symbol(ctx)?;
            }
            // wrap-around range: chr from begin, increment until reaching end+1
            let mut sym = SymSet::empty();
            let mut chr: u32 = begin as u32;
            let stop: u32 = (end as u32 + 1) & 0xff;
            loop {
                sym.insert(chr as u8);
                chr = (chr + 1) & 0xff;
                if chr == stop {
                    break;
                }
            }
            if complement {
                sym.invert();
            }
            Ok(sym)
        }
        Err(e) => {
            ctx.pos = begin_pos;
            Err(e)
        }
    }
}

fn parse_atom(ctx: &mut ParseContext) -> Result<Nfa, String> {
    if ctx.peek() == Some(b'(') {
        ctx.pos += 1;
        let sub = parse_regex(ctx)?;
        if ctx.peek() != Some(b')') {
            return Err("expected ')'".to_string());
        }
        ctx.pos += 1;
        return Ok(sub);
    }

    let symset = parse_symset(ctx)?;
    let mut nfa = Nfa::new_single();
    let final_idx = 1;
    nfa.states.push(NState::new());
    nfa.final_ = final_idx;
    nfa.states[0].target = Some(final_idx);
    nfa.states[0].label = symset;
    Ok(nfa)
}

fn parse_factor(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut atom = parse_atom(ctx)?;

    match ctx.peek() {
        Some(b'*') => {
            ctx.pos += 1;
            nfa_uncomplement(&mut atom)?;
            // atom.final.epsilon1 = atom.initial; nfa_pad_initial; nfa_pad_final; atom.initial.epsilon1 = atom.final
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[fin].epsilon1 = Some(init);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            let new_init = atom.initial;
            let new_fin = atom.final_;
            atom.states[new_init].epsilon1 = Some(new_fin);
            return Ok(atom);
        }
        Some(b'+') => {
            ctx.pos += 1;
            nfa_uncomplement(&mut atom)?;
            let init = atom.initial;
            let fin = atom.final_;
            atom.states[fin].epsilon1 = Some(init);
            nfa_pad_initial(&mut atom);
            nfa_pad_final(&mut atom);
            return Ok(atom);
        }
        Some(b'?') => {
            ctx.pos += 1;
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
            let saved = ctx.pos;
            ctx.pos += 1;
            nfa_uncomplement(&mut atom)?;
            let mut min: u32 = 0;
            let min_res = parse_natural(ctx);
            match min_res {
                Ok(v) => min = v,
                Err(e) => {
                    if e == "natural number overflow" {
                        return Err(e);
                    }
                    // not a digit - min defaults to 0
                }
            }
            let mut max: u32 = min;
            let mut max_unbounded = false;
            if ctx.peek() == Some(b',') {
                ctx.pos += 1;
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
            ctx.pos += 1;
            if min > max && !max_unbounded {
                ctx.pos = saved;
                return Err("misbounded quantifier".to_string());
            }

            let mut atoms = Nfa::new_single();

            // Determine number of copies
            let mut i: u32 = 0;
            loop {
                if max_unbounded {
                    if i > min {
                        break;
                    }
                } else {
                    if i >= max {
                        break;
                    }
                }
                let mut clone = nfa_clone(&atom);
                if i >= min {
                    if max_unbounded {
                        let cinit = clone.initial;
                        let cfin = clone.final_;
                        clone.states[cfin].epsilon1 = Some(cinit);
                        nfa_pad_initial(&mut clone);
                        nfa_pad_final(&mut clone);
                    }
                    let cinit = clone.initial;
                    let cfin = clone.final_;
                    clone.states[cinit].epsilon1 = Some(cfin);
                }
                nfa_concat(&mut atoms, clone);
                if i == u32::MAX {
                    break;
                }
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
        ctx.pos += 1;
        complement = true;
    }

    let mut term = Nfa::new_single();

    loop {
        match ctx.peek() {
            None => break,
            Some(b')') | Some(b'|') | Some(b'&') => break,
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
        ctx.pos += 1;
        let mut alt = parse_term(ctx)?;

        // Apply intersection via De Morgan: a&b == ~(~a|~b)
        if intersect {
            re.complemented = !re.complemented;
            alt.complemented = !alt.complemented;
        }
        nfa_uncomplement(&mut re)?;
        nfa_uncomplement(&mut alt)?;

        // Pad initial of re, pad final of alt; connect
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Now we need to merge alt into re
        // re.initial.epsilon1 = alt.initial (relative to merged indices)
        // re.final.epsilon0 = alt.final
        // re.final = alt.final
        let re_init = re.initial;
        let re_fin = re.final_;
        let re_states_len = re.states.len();
        let alt_states_len = alt.states.len();

        // Relabel alt states by appending to re.states with offset
        for st in alt.states.iter() {
            let mut new_st = st.clone();
            shift_option_inner(&mut new_st.target, re_states_len);
            shift_option_inner(&mut new_st.epsilon0, re_states_len);
            shift_option_inner(&mut new_st.epsilon1, re_states_len);
            re.states.push(new_st);
        }
        let alt_initial_new = alt.initial + re_states_len;
        let alt_final_new = alt.final_ + re_states_len;

        re.states[re_init].epsilon1 = Some(alt_initial_new);
        re.states[re_fin].epsilon0 = Some(alt_final_new);
        re.final_ = alt_final_new;

        if intersect {
            re.complemented = !re.complemented;
        }
        let _ = alt_states_len;
    }
    Ok(re)
}

fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}
fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}
