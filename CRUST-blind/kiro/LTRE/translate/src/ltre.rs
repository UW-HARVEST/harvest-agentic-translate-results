#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SymSet {
    bits: [u8; 256 / 8],
}
impl SymSet {
    pub fn empty() -> Self {
        SymSet { bits: [0; 256 / 8] }
    }
    pub fn full() -> Self {
        SymSet { bits: [0xff; 256 / 8] }
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
        for i in 0..self.bits.len() {
            self.bits[i] |= other.bits[i];
        }
    }
    pub fn intersect_with(&mut self, other: &SymSet) {
        for i in 0..self.bits.len() {
            self.bits[i] &= other.bits[i];
        }
    }
    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }
}
pub fn symset_fmt(set: &SymSet) -> String {
    const METACHARS: &str = "\\.-^$*+?{}[]<>()|&~";
    let mut buf = String::new();
    let mut nbuf = String::new();
    let mut nsym = 0i32;
    let mut nnsym = 0i32;

    nbuf.push('^');
    buf.push('[');
    nbuf.push('[');

    let mut chr: i32 = 0;
    while chr < 256 {
        let mut first_pass = true;
        loop {
            let c = chr as u8;
            let in_set = set.contains(c);
            if in_set { nsym += 1; } else { nnsym += 1; }
            let p = if in_set { &mut buf } else { &mut nbuf };
            let is_metachar = c != 0 && METACHARS.contains(c as char);
            if !(c as char).is_ascii_graphic() && (c as char) != ' ' && !is_metachar {
                p.push_str(&format!("\\x{:02x}", c));
            } else {
                if is_metachar { p.push('\\'); }
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
            if chr - start >= 1 && first_pass {
                first_pass = false;
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
        buf.pop(); // remove ']'
        return buf[1..].to_string(); // skip '['
    } else if nnsym == 1 {
        nbuf.pop(); // remove ']'
        let mut result = String::from("^");
        result.push_str(&nbuf[2..]); // skip '^['
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
        let states = vec![NState::new()];
        Nfa { states, initial: 0, final_: 0, complemented: false }
    }
    pub fn len(&self) -> usize {
        self.states.len()
    }
}
pub fn nfa_free(_nfa: Nfa) {}
pub fn dfa_free(_dfa: Dfa) {}
pub fn nfa_clone(orig: &Nfa) -> Nfa {
    let n = orig.states.len();
    let mut new_states: Vec<NState> = (0..n).map(|_| NState::new()).collect();
    for i in 0..n {
        new_states[i].label = orig.states[i].label;
        new_states[i].target = orig.states[i].target;
        new_states[i].epsilon0 = orig.states[i].epsilon0;
        new_states[i].epsilon1 = orig.states[i].epsilon1;
    }
    Nfa {
        states: new_states,
        initial: orig.initial,
        final_: orig.final_,
        complemented: orig.complemented,
    }
}
pub fn nfa_concat(nfa1: &mut Nfa, mut nfa2: Nfa) {
    if nfa1.initial == nfa1.final_ {
        *nfa1 = nfa2;
    } else if nfa2.initial != nfa2.final_ {
        // Merge nfa2.initial into nfa1.final_
        let offset = nfa1.states.len();
        // Copy nfa2.initial's fields into nfa1.final_
        let ini2 = nfa2.initial;
        nfa1.states[nfa1.final_].label = nfa2.states[ini2].label;
        nfa1.states[nfa1.final_].target = nfa2.states[ini2].target.map(|t| t + offset);
        nfa1.states[nfa1.final_].epsilon0 = nfa2.states[ini2].epsilon0.map(|t| t + offset);
        nfa1.states[nfa1.final_].epsilon1 = nfa2.states[ini2].epsilon1.map(|t| t + offset);
        // Add all nfa2 states except initial, shifting indices
        for (i, st) in nfa2.states.iter().enumerate() {
            if i == ini2 { continue; }
            let mut ns = NState::new();
            ns.label = st.label;
            ns.target = st.target.map(|t| if t == ini2 { nfa1.final_ } else { t + offset });
            ns.epsilon0 = st.epsilon0.map(|t| if t == ini2 { nfa1.final_ } else { t + offset });
            ns.epsilon1 = st.epsilon1.map(|t| if t == ini2 { nfa1.final_ } else { t + offset });
            nfa1.states.push(ns);
        }
        // Map nfa2 indices to nfa1 indices: ini2 -> nfa1.final_, others -> offset + orig_idx
        // But we skipped ini2, so indices shift. We need a proper mapping.
        // Actually, let's redo this more carefully.
        // We append all nfa2 states (including initial) but remap initial to nfa1.final_.
        // Let's restart with a cleaner approach.
        nfa1.states.truncate(offset); // undo partial work
        // Restore nfa1.final_ state
        nfa1.states[nfa1.final_] = NState::new();

        // Build index mapping: nfa2 state i -> nfa1 state mapped[i]
        let n2 = nfa2.states.len();
        let mut mapped = vec![0usize; n2];
        mapped[ini2] = nfa1.final_;
        let mut next_id = offset;
        for i in 0..n2 {
            if i == ini2 { continue; }
            mapped[i] = next_id;
            next_id += 1;
        }
        // Now copy all nfa2 states into nfa1
        // First, copy ini2 into nfa1.final_
        for i in 0..n2 {
            let src = &nfa2.states[i];
            let ns = NState {
                label: src.label,
                target: src.target.map(|t| mapped[t]),
                epsilon0: src.epsilon0.map(|t| mapped[t]),
                epsilon1: src.epsilon1.map(|t| mapped[t]),
            };
            if i == ini2 {
                nfa1.states[nfa1.final_] = ns;
            } else {
                nfa1.states.push(ns);
            }
        }
        nfa1.final_ = mapped[nfa2.final_];
    }
}
pub fn nfa_pad_initial(nfa: &mut Nfa) {
    let new_id = nfa.states.len();
    let mut ns = NState::new();
    ns.epsilon0 = Some(nfa.initial);
    nfa.states.push(ns);
    nfa.initial = new_id;
}
pub fn nfa_pad_final(nfa: &mut Nfa) {
    let new_id = nfa.states.len();
    let ns = NState::new();
    nfa.states.push(ns);
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
    for (id, nstate) in nfa.states.iter().enumerate() {
        if let Some(e0) = nstate.epsilon0 {
            println!("  {} --> {}", id, e0);
        }
        if let Some(e1) = nstate.epsilon1 {
            println!("  {} --> {}", id, e1);
        }
        if nstate.label.is_empty() { continue; }
        if let Some(tgt) = nstate.target {
            let fmt = symset_fmt(&nstate.label);
            print!("  {} --", id);
            for ch in fmt.chars() {
                if "\\\"#&{}()xo=- ".contains(ch) {
                    print!("#{};", ch as u8);
                } else {
                    print!("{}", ch);
                }
            }
            println!("--> {}", tgt);
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
pub fn dfa_serialize(dfa: &Dfa) -> Vec<u8> {
    let dfa_size = dfa.states.len() as i32;
    let mut buf = Vec::new();
    leb128_put(&mut buf, dfa_size);
    for ds in &dfa.states {
        buf.push(((ds.accepting as u8) << 1) | (ds.terminating as u8));
        let mut chr = 0usize;
        while chr < 256 {
            let start = chr;
            while chr < 255 && ds.transitions[chr] == ds.transitions[chr + 1] {
                chr += 1;
            }
            buf.push((chr - start) as u8);
            leb128_put(&mut buf, ds.transitions[chr] as i32);
            chr += 1;
        }
    }
    buf
}
pub fn dfa_deserialize(buf: &[u8]) -> Result<(Dfa, usize), String> {
    let mut p = 0usize;
    let dfa_size = leb128_get(buf, &mut p)? as usize;
    let mut states: Vec<DState> = Vec::with_capacity(dfa_size);
    for _ in 0..dfa_size {
        states.push(DState {
            transitions: [0; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        });
    }
    for id in 0..dfa_size {
        if p >= buf.len() { return Err("unexpected end of buffer".into()); }
        states[id].accepting = (buf[p] >> 1) & 1 != 0;
        states[id].terminating = buf[p] & 1 != 0;
        p += 1;
        let mut chr = 0usize;
        while chr < 256 {
            if p >= buf.len() { return Err("unexpected end of buffer".into()); }
            let len = buf[p] as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            for _ in 0..=len {
                if chr < 256 {
                    states[id].transitions[chr] = target;
                    chr += 1;
                }
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
        for (id2, _) in dfa.states.iter().enumerate() {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if ds1.transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            let fmt = symset_fmt(&transitions);
            print!("  {} --", id1);
            for ch in fmt.chars() {
                if "\\\"#&{}()xo=- ".contains(ch) {
                    print!("#{};", ch as u8);
                } else {
                    print!("{}", ch);
                }
            }
            println!("--> {}", id2);
        }
    }
}
fn leb128_put(buf: &mut Vec<u8>, mut n: i32) {
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
    let mut n: i32 = 0;
    let mut c = 0;
    loop {
        if *p >= buf.len() { return Err("unexpected end of buffer".into()); }
        let byte = buf[*p];
        n |= ((byte & 0x7f) as i32) << (c * 7);
        c += 1;
        *p += 1;
        if byte & 0x80 == 0 { break; }
    }
    Ok(n)
}
pub fn ltre_parse(regex: &str) -> Result<Nfa, String> {
    let mut ctx = ParseContext::new(regex);
    let nfa = parse_regex(&mut ctx)?;
    if !ctx.is_eof() {
        return Err(format!("expected end of input near '{}'", &regex[ctx.pos..]));
    }
    Ok(nfa)
}
pub fn ltre_fixed_string(s: &str) -> Nfa {
    let mut nfa = Nfa::new_single();
    for &b in s.as_bytes() {
        let new_id = nfa.states.len();
        let mut ns = NState::new();
        nfa.states.push(ns);
        nfa.states[nfa.final_].target = Some(new_id);
        nfa.states[nfa.final_].label.insert(b);
        nfa.final_ = new_id;
    }
    nfa
}
pub fn ltre_partial(nfa: &mut Nfa) -> Result<(), String> {
    let _ = nfa_uncomplement(nfa);
    nfa_pad_initial(nfa);
    nfa_pad_final(nfa);
    nfa.states[nfa.initial].target = Some(nfa.initial);
    nfa.states[nfa.initial].label = SymSet::full();
    nfa.states[nfa.final_].target = Some(nfa.final_);
    nfa.states[nfa.final_].label = SymSet::full();
    Ok(())
}
pub fn ltre_ignorecase(nfa: &mut Nfa) -> Result<(), String> {
    let _ = nfa_uncomplement(nfa);
    for st in nfa.states.iter_mut() {
        let mut new_label = st.label;
        for chr in 0u8..=255 {
            if st.label.contains(chr) {
                new_label.insert((chr as char).to_ascii_lowercase() as u8);
                new_label.insert((chr as char).to_ascii_uppercase() as u8);
            }
        }
        st.label = new_label;
    }
    Ok(())
}
pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}
pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;

    // Start with epsilon closure of initial state
    let initial_bitset = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);
    let initial_accepting = bitset_test(&initial_bitset, nfa.final_) ^ nfa.complemented;

    let mut dfa_states: Vec<DState> = Vec::new();
    let mut bitsets: Vec<Vec<u8>> = Vec::new();

    dfa_states.push(DState {
        transitions: [0; 256],
        accepting: initial_accepting,
        terminating: false,
        bitset: initial_bitset.clone(),
    });
    bitsets.push(initial_bitset);

    let mut i = 0;
    while i < dfa_states.len() {
        for chr in 0..256u16 {
            let bs = step_powerset(&nfa, &dfa_states[i].bitset, chr as u8);
            // Find existing state with same bitset
            let mut found = None;
            for (j, existing) in bitsets.iter().enumerate() {
                if *existing == bs {
                    found = Some(j);
                    break;
                }
            }
            let target = match found {
                Some(j) => j,
                None => {
                    let accepting = bitset_test(&bs, nfa.final_) ^ nfa.complemented;
                    let id = dfa_states.len();
                    dfa_states.push(DState {
                        transitions: [0; 256],
                        accepting,
                        terminating: false,
                        bitset: bs.clone(),
                    });
                    bitsets.push(bs);
                    id
                }
            };
            dfa_states[i].transitions[chr as usize] = target;
        }
        i += 1;
    }

    let mut dfa = Dfa { states: dfa_states, initial: 0 };
    dfa_minimize(&mut dfa, nfa.complemented);
    dfa
}
fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    // Find a state where all transitions point to itself
    for (i, st) in states.iter().enumerate() {
        if st.transitions.iter().all(|&t| t == i) {
            return i;
        }
    }
    let id = states.len();
    let mut ds = DState {
        transitions: [id; 256],
        accepting: false,
        terminating: true,
        bitset: Vec::new(),
    };
    states.push(ds);
    id
}
fn step_powerset(nfa: &Nfa, bitset: &[u8], chr: u8) -> Vec<u8> {
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;
    let mut result = vec![0u8; bitset_size];
    for id in 0..nfa_size {
        if bitset_test(bitset, id) && nfa.states[id].label.contains(chr) {
            if let Some(tgt) = nfa.states[id].target {
                epsilon_closure_into(nfa, tgt, &mut result);
            }
        }
    }
    result
}
fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let bitset_size = (nfa_size + 7) / 8;
    let mut bitset = vec![0u8; bitset_size];
    epsilon_closure_into(nfa, start, &mut bitset);
    bitset
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
fn dfa_minimize(dfa: &mut Dfa, complemented: bool) {
    let dfa_size = dfa.states.len();
    if dfa_size == 0 { return; }

    let dis_size = (dfa_size + 7) / 8;
    let mut dis = vec![vec![0u8; dis_size]; dfa_size];

    // Initially mark pairs with different accepting values as distinguishable
    for i in 0..dfa_size {
        for j in (i + 1)..dfa_size {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                bitset_set(&mut dis[i], j);
                bitset_set(&mut dis[j], i);
            }
        }
    }

    // Iteratively refine
    let mut changed = true;
    while changed {
        changed = false;
        for id1 in 0..dfa_size {
            for id2 in (id1 + 1)..dfa_size {
                if bitset_test(&dis[id1], id2) { continue; }
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

    // Build mapping: for each state, find the representative (lowest-id indistinguishable state)
    let mut mapping = vec![0usize; dfa_size];
    for i in 0..dfa_size {
        mapping[i] = i;
        for j in 0..i {
            if !bitset_test(&dis[i], j) && mapping[j] == j {
                mapping[i] = j;
                break;
            }
        }
    }

    // Remap all transitions
    for i in 0..dfa_size {
        for chr in 0..256usize {
            dfa.states[i].transitions[chr] = mapping[dfa.states[i].transitions[chr]];
        }
    }

    // Remap initial
    dfa.initial = mapping[dfa.initial];

    // Remove merged states (keep only representatives)
    let mut keep: Vec<bool> = vec![false; dfa_size];
    for i in 0..dfa_size {
        if mapping[i] == i { keep[i] = true; }
    }

    // Build new state list and remap indices
    let mut new_id = vec![0usize; dfa_size];
    let mut new_states = Vec::new();
    for i in 0..dfa_size {
        if keep[i] {
            new_id[i] = new_states.len();
            new_states.push(dfa.states[i].clone());
        }
    }
    for i in 0..dfa_size {
        if !keep[i] {
            new_id[i] = new_id[mapping[i]];
        }
    }

    // Remap transitions in new states
    for st in new_states.iter_mut() {
        for chr in 0..256usize {
            st.transitions[chr] = new_id[st.transitions[chr]];
        }
    }

    // Flag terminating states
    for st in new_states.iter_mut() {
        st.terminating = true;
        for chr in 0..256usize {
            // A state is terminating if all transitions point to itself
            // We need to find its own id first
        }
    }
    for i in 0..new_states.len() {
        new_states[i].terminating = (0..256).all(|chr| new_states[i].transitions[chr] == i);
    }

    dfa.initial = new_id[dfa.initial];
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
    for &b in input {
        if dfa.states[state].terminating { break; }
        state = dfa.states[state].transitions[b as usize];
    }
    dfa.states[state].accepting
}
pub fn ltre_matches_lazy(dfap: &mut Option<Dfa>, nfa: &Nfa, input: &[u8]) -> bool {
    // Lazy compilation: build DFA states on demand
    let nfa_size = nfa.states.len();
    let bitset_size = (nfa_size + 7) / 8;

    if dfap.is_none() {
        let initial_bitset = epsilon_closure_vec(nfa, nfa.initial, nfa_size);
        let accepting = bitset_test(&initial_bitset, nfa.final_) ^ nfa.complemented;
        let ds = DState {
            transitions: [usize::MAX; 256],
            accepting,
            terminating: false,
            bitset: initial_bitset,
        };
        *dfap = Some(Dfa { states: vec![ds], initial: 0 });
    }

    let dfa = dfap.as_mut().unwrap();
    let mut state = dfa.initial;

    for &b in input {
        let chr = b as usize;
        if dfa.states[state].transitions[chr] == usize::MAX {
            // Create new state
            let bs = step_powerset(nfa, &dfa.states[state].bitset, b);
            // Find existing
            let mut found = None;
            for (j, st) in dfa.states.iter().enumerate() {
                if st.bitset == bs {
                    found = Some(j);
                    break;
                }
            }
            let target = match found {
                Some(j) => j,
                None => {
                    let accepting = bitset_test(&bs, nfa.final_) ^ nfa.complemented;
                    let id = dfa.states.len();
                    dfa.states.push(DState {
                        transitions: [usize::MAX; 256],
                        accepting,
                        terminating: false,
                        bitset: bs,
                    });
                    id
                }
            };
            dfa.states[state].transitions[chr] = target;
        }
        state = dfa.states[state].transitions[chr];
    }

    dfa.states[state].accepting
}
pub fn ltre_uncompile(dfa: &Dfa) -> Nfa {
    let dfa_size = dfa.states.len();

    // Create initial and final NFA states, plus one NFA state per DFA state
    let initial_id = 0;
    let mut states: Vec<NState> = Vec::new();
    states.push(NState::new()); // initial (id 0)

    // NFA states for DFA states: ids 1..=dfa_size
    let nstate_base = 1;
    for _ in 0..dfa_size {
        states.push(NState::new());
    }
    let final_id = states.len();
    states.push(NState::new()); // final

    // initial -> epsilon1 -> nstates[dfa.initial]
    states[initial_id].epsilon1 = Some(nstate_base + dfa.initial);

    // accepting DFA states -> epsilon1 -> final
    for (id, ds) in dfa.states.iter().enumerate() {
        if ds.accepting {
            states[nstate_base + id].epsilon1 = Some(final_id);
        }
    }

    // For each DFA state, build a binary tree of labeled transitions
    for (ds1_id, ds1) in dfa.states.iter().enumerate() {
        let mut free: Option<usize> = None;

        for ds2_id in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if ds1.transitions[chr] == ds2_id {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }

            let src;
            if free.is_none() {
                // First iteration: use the nstate corresponding to this DFA state
                free = Some(nstate_base + ds1_id);
                src = nstate_base + ds1_id;
            } else {
                let new_id = states.len();
                states.push(NState::new());
                src = new_id;

                let f = free.unwrap();
                if states[f].epsilon1.is_none() {
                    states[f].epsilon1 = Some(new_id);
                } else {
                    states[f].epsilon0 = Some(new_id);
                    free = Some(new_id);
                }
            }

            states[src].target = Some(nstate_base + ds2_id);
            states[src].label = transitions;
        }
    }

    Nfa {
        states,
        initial: initial_id,
        final_: final_id,
        complemented: false,
    }
}
pub fn ltre_decompile(dfa: &Dfa) -> String {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Prec { Alt, Concat, Quant, Symset }

    #[derive(Clone)]
    struct Arrow {
        label: Option<String>, // None = empty /[]/, Some("") = epsilon /()/
        prec: Prec,
    }

    let dfa_size = dfa.states.len();
    let aux = dfa_size; // auxiliary state index
    let n = dfa_size + 1;

    let mut arrows: Vec<Vec<Arrow>> = vec![vec![Arrow { label: None, prec: Prec::Symset }; n]; n];

    // Epsilon from aux to initial
    arrows[aux][dfa.initial].label = Some(String::new());
    arrows[aux][dfa.initial].prec = Prec::Symset;

    for (id1, ds1) in dfa.states.iter().enumerate() {
        // Accepting states -> epsilon to aux
        if ds1.accepting {
            arrows[id1][aux].label = Some(String::new());
            arrows[id1][aux].prec = Prec::Symset;
        }

        for id2 in 0..dfa_size {
            let mut transitions = SymSet::empty();
            let mut empty = true;
            for chr in 0..256usize {
                if ds1.transitions[chr] == id2 {
                    transitions.insert(chr as u8);
                    empty = false;
                }
            }
            if empty { continue; }
            let fmt = symset_fmt(&transitions);
            arrows[id1][id2].label = Some(fmt);
            arrows[id1][id2].prec = Prec::Symset;
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

                let (mut first_label, mut first_prec, mut second_label, mut second_prec);

                if self_arrow.label.is_none() || self_arrow.label.as_ref().unwrap().is_empty() {
                    first_label = in_label.clone();
                    first_prec = in_arrow.prec;
                    second_label = out_label.clone();
                    second_prec = out_arrow.prec;
                } else {
                    let self_label = self_arrow.label.as_ref().unwrap();
                    let self_prec = self_arrow.prec;

                    // Default: (in)(self)*(out)
                    first_label = in_label.clone();
                    first_prec = in_arrow.prec;
                    second_label = {
                        let mut p = String::new();
                        if (self_prec as u8) <= Prec::Quant as u8 { p.push('('); }
                        p.push_str(self_label);
                        if (self_prec as u8) <= Prec::Quant as u8 { p.push(')'); }
                        p.push('*');
                        if (out_arrow.prec as u8) < Prec::Concat as u8 { p.push('('); }
                        p.push_str(out_label);
                        if (out_arrow.prec as u8) < Prec::Concat as u8 { p.push(')'); }
                        p
                    };
                    second_prec = Prec::Concat;

                    // Try optimization: (in_pre)(self)+(out) where (in) ends with (self)
                    if in_arrow.prec as u8 >= Prec::Concat as u8 && self_prec as u8 >= Prec::Concat as u8 {
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
                            if !nevermind && d >= 2 && &in_label[d-2..d] == "\\x" && (d == 2 || in_label.as_bytes()[d - 3] != b'\\') {
                                nevermind = true;
                            }
                            if !nevermind && d >= 3 && &in_label[d-3..d-1] == "\\x" && (d == 3 || in_label.as_bytes()[d - 4] != b'\\') {
                                nevermind = true;
                            }

                            if !nevermind {
                                let mut p = String::new();
                                if d != 0 && (in_arrow.prec as u8) < Prec::Concat as u8 { p.push('('); }
                                p.push_str(&in_label[..d]);
                                if d != 0 && (in_arrow.prec as u8) < Prec::Concat as u8 { p.push(')'); }
                                if (self_prec as u8) <= Prec::Quant as u8 { p.push('('); }
                                p.push_str(self_label);
                                if (self_prec as u8) <= Prec::Quant as u8 { p.push(')'); }
                                p.push('+');
                                first_label = p;
                                first_prec = Prec::Concat;
                                second_label = out_label.clone();
                                second_prec = out_arrow.prec;
                            }
                        }
                    }

                    // If we didn't optimize with in, try: (in)(self)+(out_post) where (out) starts with (self)
                    if first_label == *in_label && first_prec as u8 == in_arrow.prec as u8 {
                        if out_arrow.prec as u8 >= Prec::Concat as u8 && self_prec as u8 >= Prec::Concat as u8 {
                            let diff = out_label.len() as isize - self_label.len() as isize;
                            if diff >= 0 && out_label.starts_with(self_label) {
                                let d = diff as usize;
                                let mut p = String::new();
                                if (self_prec as u8) <= Prec::Quant as u8 { p.push('('); }
                                p.push_str(self_label);
                                if (self_prec as u8) <= Prec::Quant as u8 { p.push(')'); }
                                p.push('+');
                                if d != 0 && (out_arrow.prec as u8) < Prec::Concat as u8 { p.push('('); }
                                p.push_str(&out_label[self_label.len()..]);
                                if d != 0 && (out_arrow.prec as u8) < Prec::Concat as u8 { p.push(')'); }
                                second_label = p;
                                second_prec = Prec::Concat;
                            }
                        }
                    }
                }

                // Concatenate first and second to create bypass
                let (bypass_label, bypass_prec);
                if first_label.is_empty() {
                    bypass_label = Some(second_label);
                    bypass_prec = second_prec;
                } else if second_label.is_empty() {
                    bypass_label = Some(first_label);
                    bypass_prec = first_prec;
                } else {
                    let mut p = String::new();
                    if (first_prec as u8) < Prec::Concat as u8 { p.push('('); }
                    p.push_str(&first_label);
                    if (first_prec as u8) < Prec::Concat as u8 { p.push(')'); }
                    if (second_prec as u8) < Prec::Concat as u8 { p.push('('); }
                    p.push_str(&second_label);
                    if (second_prec as u8) < Prec::Concat as u8 { p.push(')'); }
                    bypass_label = Some(p);
                    bypass_prec = Prec::Concat;
                }

                // Merge bypass with existing
                let merged;
                if bypass_label.is_none() {
                    merged = existing;
                } else if existing.label.is_none() {
                    merged = Arrow { label: bypass_label, prec: bypass_prec };
                } else if existing.label.as_ref().unwrap().is_empty() {
                    // ()|(bypass) == (bypass)?
                    let bl = bypass_label.unwrap();
                    let mut p = String::new();
                    if (bypass_prec as u8) <= Prec::Quant as u8 { p.push('('); }
                    p.push_str(&bl);
                    if (bypass_prec as u8) <= Prec::Quant as u8 { p.push(')'); }
                    p.push('?');
                    merged = Arrow { label: Some(p), prec: Prec::Quant };
                } else {
                    // (existing)|(bypass)
                    let el = existing.label.unwrap();
                    let bl = bypass_label.unwrap();
                    let mut p = String::new();
                    p.push_str(&el);
                    p.push('|');
                    p.push_str(&bl);
                    merged = Arrow { label: Some(p), prec: Prec::Alt };
                }

                arrows[id1][id2] = merged;
            }
        }

        // Eliminate the best_fit state
        for id in 0..n {
            arrows[id][best_fit] = Arrow { label: None, prec: Prec::Symset };
            arrows[best_fit][id] = Arrow { label: None, prec: Prec::Symset };
        }
    }

    match &arrows[aux][aux].label {
        Some(s) => s.clone(),
        None => "[]".to_string(),
    }
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
fn parse_regex(ctx: &mut ParseContext) -> Result<Nfa, String> {
    let mut re = parse_term(ctx)?;

    while ctx.peek() == Some(b'|') || ctx.peek() == Some(b'&') {
        let intersect = ctx.peek() == Some(b'&');
        ctx.next();
        let mut alt = parse_term(ctx)?;

        // De Morgan's law for intersection
        re.complemented ^= intersect;
        alt.complemented ^= intersect;
        let _ = nfa_uncomplement(&mut re);
        let _ = nfa_uncomplement(&mut alt);

        // -->O-->(re)--->
        //     -->(alt)-->O-->
        nfa_pad_initial(&mut re);
        nfa_pad_final(&mut alt);

        // Merge alt into re
        let offset = re.states.len();
        for mut st in alt.states {
            shift_option(&mut st.target, offset);
            shift_option(&mut st.epsilon0, offset);
            shift_option(&mut st.epsilon1, offset);
            re.states.push(st);
        }
        re.states[re.initial].epsilon1 = Some(alt.initial + offset);
        re.states[re.final_].epsilon0 = Some(alt.final_ + offset);
        re.final_ = alt.final_ + offset;

        re.complemented ^= intersect;
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

    while !matches!(ctx.peek(), None | Some(b')') | Some(b'|') | Some(b'&')) {
        let mut factor = parse_factor(ctx)?;
        let _ = nfa_uncomplement(&mut factor);
        nfa_concat(&mut term, factor);
    }

    if complement {
        term.complemented = true;
    }

    Ok(term)
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
            match parse_natural(ctx) {
                Ok(v) => max = v,
                Err(ref e) if e.contains("overflow") => return Err(e.clone()),
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

        let limit = if max_unbounded { min } else { max };
        let mut i: u32 = 0;
        loop {
            if max_unbounded { if i > limit { break; } } else { if i >= limit { break; } }
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

    Ok(atom)
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
fn parse_symset(ctx: &mut ParseContext) -> Result<SymSet, String> {
    let mut complement = false;
    if ctx.peek() == Some(b'^') {
        ctx.next();
        complement = true;
    }

    let saved = ctx.pos;

    // Try shorthand
    if let Ok(ss) = parse_shorthand(ctx) {
        if complement { let mut ss = ss; ss.invert(); return Ok(ss); }
        return Ok(ss);
    }
    ctx.pos = saved;

    // Try [...]
    if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut symset = SymSet::empty();
        while ctx.peek() != Some(b']') {
            let sub = parse_symset(ctx)?;
            symset.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".into());
        }
        ctx.next();
        if complement { symset.invert(); }
        return Ok(symset);
    }
    ctx.pos = saved;

    // Try <...>
    if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut symset = SymSet::full();
        while ctx.peek() != Some(b'>') {
            let sub = parse_symset(ctx)?;
            symset.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".into());
        }
        ctx.next();
        if complement { symset.invert(); }
        return Ok(symset);
    }
    ctx.pos = saved;

    // Try symbol (possibly with range)
    let begin = parse_symbol(ctx)?;
    let mut end = begin;
    if ctx.peek() == Some(b'-') {
        ctx.next();
        end = parse_symbol(ctx)?;
    }
    let mut symset = SymSet::empty();
    let end_open = end.wrapping_add(1);
    let mut c = begin;
    loop {
        symset.insert(c);
        c = c.wrapping_add(1);
        if c == end_open { break; }
    }
    if complement { symset.invert(); }
    Ok(symset)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<SymSet, String> {
    if ctx.peek() == Some(b'\\') {
        let saved = ctx.pos;
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
        // dot matches everything except \n
        s.bits[b'\n' as usize / 8] &= !(1 << (b'\n' as usize % 8));
        return Ok(s);
    }
    Err("expected shorthand class".into())
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte = 0u8;
    for _ in 0..2 {
        byte <<= 4;
        match ctx.peek() {
            Some(c) if c.is_ascii_digit() => { byte |= c - b'0'; ctx.next(); }
            Some(c) if c.is_ascii_hexdigit() => { byte |= (c as char).to_ascii_lowercase() as u8 - b'a' + 10; ctx.next(); }
            _ => return Err("expected hex digit".into()),
        }
    }
    Ok(byte)
}

const METACHARS: &[u8] = b"\\.-^$*+?{}[]<>()|&~";

fn parse_escape(ctx: &mut ParseContext) -> Result<u8, String> {
    if let Some(c) = ctx.peek() {
        if METACHARS.contains(&c) {
            ctx.next();
            return Ok(c);
        }
    }
    match ctx.peek() {
        Some(b'a') => { ctx.next(); Ok(0x07) }
        Some(b'b') => { ctx.next(); Ok(0x08) }
        Some(b'f') => { ctx.next(); Ok(0x0c) }
        Some(b'n') => { ctx.next(); Ok(b'\n') }
        Some(b'r') => { ctx.next(); Ok(b'\r') }
        Some(b't') => { ctx.next(); Ok(b'\t') }
        Some(b'v') => { ctx.next(); Ok(0x0b) }
        Some(b'x') => { ctx.next(); parse_hexbyte(ctx) }
        _ => Err("unknown escape".into()),
    }
}

fn parse_symbol(ctx: &mut ParseContext) -> Result<u8, String> {
    if ctx.peek() == Some(b'\\') {
        ctx.next();
        return parse_escape(ctx);
    }
    match ctx.peek() {
        None => Err("expected symbol".into()),
        Some(c) if METACHARS.contains(&c) => Err("unexpected metacharacter".into()),
        Some(c) if !(c as char).is_ascii_graphic() && c != b' ' => Err("unexpected nonprintable character".into()),
        Some(c) => { ctx.next(); Ok(c) }
    }
}
fn union_inplace(a: &mut SymSet, b: &SymSet) {
    a.union_with(b);
}
fn intersect_inplace(a: &mut SymSet, b: &SymSet) {
    a.intersect_with(b);
}
fn digits_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0u8..=255 { if (c as char).is_ascii_digit() { s.insert(c); } }
    s
}
fn not_digits_set() -> SymSet {
    let mut s = digits_set(); s.invert(); s
}
fn spaces_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0u8..=255 {
        if (c as char).is_ascii_whitespace() || c == 0x0b { s.insert(c); }
    }
    s
}
fn not_spaces_set() -> SymSet {
    let mut s = spaces_set(); s.invert(); s
}
fn wordchar_set() -> SymSet {
    let mut s = SymSet::empty();
    for c in 0u8..=255 { if c == b'_' || (c as char).is_ascii_alphanumeric() { s.insert(c); } }
    s
}
fn not_wordchar_set() -> SymSet {
    let mut s = wordchar_set(); s.invert(); s
}
fn parse_natural(ctx: &mut ParseContext) -> Result<u32, String> {
    match ctx.peek() {
        Some(c) if c.is_ascii_digit() => {}
        _ => return Err("expected natural number".into()),
    }
    let mut natural: u32 = 0;
    while let Some(c) = ctx.peek() {
        if !c.is_ascii_digit() { break; }
        ctx.next();
        let digit = (c - b'0') as u32;
        if natural > u32::MAX / 10 || natural * 10 > u32::MAX - digit {
            return Err("natural number overflow".into());
        }
        natural = natural * 10 + digit;
    }
    Ok(natural)
}
fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(ref mut v) = opt {
        *v += offset;
    }
}