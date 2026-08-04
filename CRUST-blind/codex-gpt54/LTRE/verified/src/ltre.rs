const METACHARS: &[u8] = br"\.-^$*+?{}[]<>()|&~";
const NO_STATE: usize = usize::MAX;

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
        bitset_test(&self.bits, c as usize)
    }

    pub fn insert(&mut self, c: u8) {
        bitset_set(&mut self.bits, c as usize);
    }

    pub fn invert(&mut self) {
        for byte in &mut self.bits {
            *byte = !*byte;
        }
    }

    pub fn union_with(&mut self, other: &SymSet) {
        for (dst, src) in self.bits.iter_mut().zip(other.bits.iter()) {
            *dst |= *src;
        }
    }

    pub fn intersect_with(&mut self, other: &SymSet) {
        for (dst, src) in self.bits.iter_mut().zip(other.bits.iter()) {
            *dst &= *src;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.bits.iter().all(|&b| b == 0)
    }
}

pub fn symset_fmt(set: &SymSet) -> String {
    fn is_printable(chr: u8) -> bool {
        matches!(chr, 0x20..=0x7e)
    }

    fn append_symbol(dst: &mut String, chr: u8) {
        let is_metachar = chr != 0 && METACHARS.contains(&chr);
        if !is_printable(chr) && !is_metachar {
            dst.push_str(&format!("\\x{chr:02x}"));
        } else {
            if is_metachar {
                dst.push('\\');
            }
            dst.push(chr as char);
        }
    }

    let mut buf = String::from("[");
    let mut nbuf = String::from("^[");
    let mut nsym = 0i32;
    let mut nnsym = 0i32;
    let mut chr = 0usize;

    while chr < 256 {
        let in_set = set.contains(chr as u8);
        if in_set {
            nsym += 1;
            append_symbol(&mut buf, chr as u8);
        } else {
            nnsym += 1;
            append_symbol(&mut nbuf, chr as u8);
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
            if in_set {
                nsym += 1;
                append_symbol(&mut buf, chr as u8);
            } else {
                nnsym += 1;
                append_symbol(&mut nbuf, chr as u8);
            }
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
        return format!("^{}", &nbuf[2..nbuf.len() - 1]);
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

fn remap_state(mut state: NState, map: &[usize]) -> NState {
    state.target = state.target.map(|idx| map[idx]);
    state.epsilon0 = state.epsilon0.map(|idx| map[idx]);
    state.epsilon1 = state.epsilon1.map(|idx| map[idx]);
    state
}

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

    let mut map = vec![0usize; nfa2.states.len()];
    for idx in 0..nfa2.states.len() {
        if idx == nfa2.initial {
            map[idx] = nfa1.final_;
        } else {
            map[idx] = nfa1.states.len();
            nfa1.states.push(NState::new());
        }
    }

    for (old_idx, state) in nfa2.states.into_iter().enumerate() {
        let new_idx = map[old_idx];
        nfa1.states[new_idx] = remap_state(state, &map);
    }
    nfa1.final_ = map[nfa2.final_];
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
    nfa.states.push(NState::new());
    nfa.states[nfa.final_].epsilon0 = Some(idx);
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

    for (idx, state) in nfa.states.iter().enumerate() {
        if let Some(next) = state.epsilon0 {
            println!("  {idx} --> {next}");
        }
        if let Some(next) = state.epsilon1 {
            println!("  {idx} --> {next}");
        }
        if state.label.is_empty() {
            continue;
        }

        let fmt = symset_fmt(&state.label);
        print!("  {idx} --");
        for b in fmt.bytes() {
            if b"\\\"#&{}()xo=- ".contains(&b) {
                print!("#{b};");
            } else {
                print!("{}", b as char);
            }
        }
        println!("--> {}", state.target.unwrap_or(idx));
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
    let mut dfa = dfa.clone();
    if dfa.states.iter().any(|st| st.transitions.iter().any(|&t| t == NO_STATE)) {
        let dead = find_or_create_dead(&mut dfa.states);
        for state in &mut dfa.states {
            for target in &mut state.transitions {
                if *target == NO_STATE {
                    *target = dead;
                }
            }
        }
    }

    let mut buf = Vec::new();
    leb128_put(&mut buf, dfa.states.len() as i32);
    for state in &dfa.states {
        buf.push((state.accepting as u8) << 1 | state.terminating as u8);
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
    let dfa_size = leb128_get(buf, &mut p)?;
    if dfa_size <= 0 {
        return Err("invalid dfa size".to_string());
    }

    let mut states = Vec::with_capacity(dfa_size as usize);
    for _ in 0..dfa_size {
        states.push(DState {
            transitions: [NO_STATE; 256],
            accepting: false,
            terminating: false,
            bitset: Vec::new(),
        });
    }

    for id in 0..dfa_size as usize {
        let flags = *buf.get(p).ok_or_else(|| "truncated dfa".to_string())?;
        p += 1;
        states[id].accepting = (flags >> 1) & 1 != 0;
        states[id].terminating = flags & 1 != 0;

        let mut chr = 0usize;
        while chr < 256 {
            let len = *buf.get(p).ok_or_else(|| "truncated dfa".to_string())? as usize;
            p += 1;
            let target = leb128_get(buf, &mut p)? as usize;
            if target >= states.len() {
                return Err("invalid dfa transition".to_string());
            }
            for _ in 0..=len {
                if chr >= 256 {
                    return Err("invalid run length".to_string());
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

    for (id1, state1) in dfa.states.iter().enumerate() {
        if state1.accepting {
            println!("  {id1} --> F( )");
        }

        for id2 in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            for chr in 0..=255u8 {
                if state1.transitions[chr as usize] == id2 {
                    transitions.insert(chr);
                }
            }
            if transitions.is_empty() {
                continue;
            }

            let fmt = symset_fmt(&transitions);
            print!("  {id1} --");
            for b in fmt.bytes() {
                if b"\\\"#&{}()xo=- ".contains(&b) {
                    print!("#{b};");
                } else {
                    print!("{}", b as char);
                }
            }
            println!("--> {id2}");
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
    let mut c = 0u32;
    loop {
        let byte = *buf.get(*p).ok_or_else(|| "truncated leb128".to_string())?;
        *p += 1;
        n |= ((byte & 0x7f) as i32) << (c * 7);
        c += 1;
        if byte & 0x80 == 0 {
            return Ok(n);
        }
        if c >= 5 {
            return Err("leb128 overflow".to_string());
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
    for &byte in s.as_bytes() {
        let initial = nfa.final_;
        let next = nfa.states.len();
        nfa.states.push(NState::new());
        nfa.states[initial].target = Some(next);
        nfa.states[initial].label.insert(byte);
        nfa.final_ = next;
    }
    nfa
}

pub fn ltre_partial(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    nfa_pad_initial(nfa);
    nfa_pad_final(nfa);
    let initial = nfa.initial;
    let final_ = nfa.final_;
    nfa.states[initial].target = Some(initial);
    nfa.states[final_].target = Some(final_);
    nfa.states[initial].label = SymSet::full();
    nfa.states[final_].label = SymSet::full();
    Ok(())
}

pub fn ltre_ignorecase(nfa: &mut Nfa) -> Result<(), String> {
    nfa_uncomplement(nfa)?;
    for state in &mut nfa.states {
        for chr in 0..=255u8 {
            if state.label.contains(chr) {
                state.label.insert(chr.to_ascii_lowercase());
                state.label.insert(chr.to_ascii_uppercase());
            }
        }
    }
    Ok(())
}

pub fn ltre_complement(nfa: &mut Nfa) {
    nfa.complemented = !nfa.complemented;
}

fn dfa_state_from_bitset(bitset: Vec<u8>, nfa: &Nfa) -> DState {
    DState {
        transitions: [NO_STATE; 256],
        accepting: bitset_test(&bitset, nfa.final_) ^ nfa.complemented,
        terminating: false,
        bitset,
    }
}

pub fn ltre_compile(nfa: Nfa) -> Dfa {
    let nfa_size = nfa.len();
    let bitset_size = nfa_size.div_ceil(8);
    let initial_bitset = epsilon_closure_vec(&nfa, nfa.initial, nfa_size);

    let mut dfa = Dfa {
        states: vec![dfa_state_from_bitset(initial_bitset, &nfa)],
        initial: 0,
    };

    let mut state_idx = 0usize;
    while state_idx < dfa.states.len() {
        for chr in 0..=255u8 {
            let bitset = step_powerset(&nfa, &dfa.states[state_idx].bitset, chr);
            let target = if let Some(existing) = dfa.states.iter().position(|st| st.bitset == bitset)
            {
                existing
            } else {
                dfa.states.push(dfa_state_from_bitset(bitset, &nfa));
                dfa.states.len() - 1
            };
            dfa.states[state_idx].transitions[chr as usize] = target;
        }
        state_idx += 1;
    }

    dfa_minimize(&mut dfa, nfa.complemented);
    for idx in 0..dfa.states.len() {
        dfa.states[idx].terminating = dfa.states[idx].transitions.iter().all(|&t| t == idx);
    }
    if bitset_size == 0 && dfa.states.is_empty() {
        dfa.states.push(DState {
            transitions: [0; 256],
            accepting: false,
            terminating: true,
            bitset: Vec::new(),
        });
    }
    dfa
}

fn find_or_create_dead(states: &mut Vec<DState>) -> usize {
    for (idx, state) in states.iter().enumerate() {
        if !state.accepting
            && state.terminating
            && state.transitions.iter().all(|&target| target == idx)
        {
            return idx;
        }
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
    let nfa_size = nfa.len();
    let mut out = vec![0u8; nfa_size.div_ceil(8)];
    for id in all_bitset_indices(bitset) {
        if id < nfa.states.len() && nfa.states[id].label.contains(chr) {
            if let Some(target) = nfa.states[id].target {
                epsilon_closure_into(nfa, target, &mut out);
            }
        }
    }
    out
}

fn epsilon_closure_vec(nfa: &Nfa, start: usize, nfa_size: usize) -> Vec<u8> {
    let mut bitset = vec![0u8; nfa_size.div_ceil(8)];
    epsilon_closure_into(nfa, start, &mut bitset);
    bitset
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
    if n <= 1 {
        return;
    }

    let row_size = n.div_ceil(8);
    let mut dis = vec![vec![0u8; row_size]; n];

    let are_dis = |dis: &[Vec<u8>], a: usize, b: usize| bitset_test(&dis[a], b);
    let make_dis = |dis: &mut [Vec<u8>], a: usize, b: usize| {
        bitset_set(&mut dis[a], b);
        bitset_set(&mut dis[b], a);
    };

    for i in 0..n {
        for j in i + 1..n {
            if dfa.states[i].accepting != dfa.states[j].accepting {
                make_dis(&mut dis, i, j);
            }
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for i in 0..n {
            for j in i + 1..n {
                if are_dis(&dis, i, j) {
                    continue;
                }
                for chr in 0..256 {
                    let t1 = dfa.states[i].transitions[chr];
                    let t2 = dfa.states[j].transitions[chr];
                    if t1 != t2 && are_dis(&dis, t1, t2) {
                        make_dis(&mut dis, i, j);
                        changed = true;
                        break;
                    }
                }
            }
        }
    }

    let mut rep = (0..n).collect::<Vec<_>>();
    for i in 0..n {
        for j in i + 1..n {
            if !are_dis(&dis, i, j) {
                rep[j] = rep[i];
            }
        }
    }

    let mut new_index = vec![NO_STATE; n];
    let mut new_states = Vec::new();
    for i in 0..n {
        if rep[i] == i {
            new_index[i] = new_states.len();
            new_states.push(dfa.states[i].clone());
        }
    }
    for i in 0..n {
        if rep[i] != i {
            new_index[i] = new_index[rep[i]];
        }
    }

    for state in &mut new_states {
        for target in &mut state.transitions {
            *target = new_index[*target];
        }
    }

    dfa.initial = new_index[dfa.initial];
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
    let dfa_size = dfa.len();
    let mut nfa = Nfa {
        states: vec![NState::new(), NState::new()],
        initial: 0,
        final_: 1,
        complemented: false,
    };

    let mut nstates = vec![0usize; dfa_size];
    for slot in &mut nstates {
        *slot = nfa.states.len();
        nfa.states.push(NState::new());
    }

    nfa.states[nfa.initial].epsilon1 = Some(nstates[dfa.initial]);
    for (id, dstate) in dfa.states.iter().enumerate() {
        if dstate.accepting {
            nfa.states[nstates[id]].epsilon1 = Some(nfa.final_);
        }
    }

    for (ds1_idx, ds1) in dfa.states.iter().enumerate() {
        let mut free = None::<usize>;
        for ds2_idx in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            for chr in 0..=255u8 {
                if ds1.transitions[chr as usize] == ds2_idx {
                    transitions.insert(chr);
                }
            }
            if transitions.is_empty() {
                continue;
            }

            let src = if free.is_none() {
                let root = nstates[ds1_idx];
                free = Some(root);
                root
            } else {
                let src = nfa.states.len();
                nfa.states.push(NState::new());
                let free_idx = free.unwrap();
                if nfa.states[free_idx].epsilon1.is_none() {
                    nfa.states[free_idx].epsilon1 = Some(src);
                } else {
                    nfa.states[free_idx].epsilon0 = Some(src);
                    free = Some(src);
                }
                src
            };

            nfa.states[src].target = Some(nstates[ds2_idx]);
            nfa.states[src].label = transitions;
        }
    }

    nfa
}

pub fn ltre_decompile(dfa: &Dfa) -> String {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
    enum Prec {
        Alt,
        Concat,
        Quant,
        Symset,
    }

    #[derive(Clone, Debug)]
    struct Arrow {
        label: Option<String>,
        prec: Prec,
    }

    let dfa_size = dfa.len();
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

    arrows[dfa_size][dfa.initial] = Arrow {
        label: Some(String::new()),
        prec: Prec::Symset,
    };

    for (id1, ds1) in dfa.states.iter().enumerate() {
        if ds1.accepting {
            arrows[id1][dfa_size] = Arrow {
                label: Some(String::new()),
                prec: Prec::Symset,
            };
        }

        for id2 in 0..dfa.states.len() {
            let mut transitions = SymSet::empty();
            for chr in 0..=255u8 {
                if ds1.transitions[chr as usize] == id2 {
                    transitions.insert(chr);
                }
            }
            if !transitions.is_empty() {
                arrows[id1][id2] = Arrow {
                    label: Some(symset_fmt(&transitions)),
                    prec: Prec::Symset,
                };
            }
        }
    }

    while let Some(best_fit) = {
        let mut best = None;
        let mut min_degree = usize::MAX;
        for id1 in 0..dfa_size {
            let degree = (0..dfa_size)
                .map(|id2| {
                    arrows[id1][id2].label.is_some() as usize
                        + arrows[id2][id1].label.is_some() as usize
                })
                .sum::<usize>();
            if degree == 0 {
                continue;
            }
            if degree < min_degree {
                min_degree = degree;
                best = Some(id1);
            }
        }
        best
    } {
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

                let (first, second) = if self_loop.label.is_none()
                    || self_loop.label.as_deref() == Some("")
                {
                    (inbound.clone(), outbound.clone())
                } else {
                    let self_label = self_loop.label.as_ref().unwrap();

                    let inbound_plus = if inbound.prec >= Prec::Concat
                        && self_loop.prec >= Prec::Concat
                        && in_label.len() >= self_label.len()
                        && in_label[in_label.len() - self_label.len()..] == *self_label
                    {
                        let diff = in_label.len() - self_label.len();
                        let inbound_bytes = in_label.as_bytes();
                        let preserve = (diff >= 1
                            && br"^-\\".contains(&inbound_bytes[diff - 1])
                            && (diff == 1 || inbound_bytes[diff - 2] != b'\\'))
                            || (diff >= 2
                                && &inbound_bytes[diff - 2..diff] == b"\\x"
                                && (diff == 2 || inbound_bytes[diff - 3] != b'\\'))
                            || (diff >= 3
                                && &inbound_bytes[diff - 3..diff - 1] == b"\\x"
                                && (diff == 3 || inbound_bytes[diff - 4] != b'\\'));

                        if preserve {
                            None
                        } else {
                            let mut label = String::new();
                            if diff != 0 && inbound.prec < Prec::Concat {
                                label.push('(');
                            }
                            label.push_str(&in_label[..diff]);
                            if diff != 0 && inbound.prec < Prec::Concat {
                                label.push(')');
                            }
                            if self_loop.prec <= Prec::Quant {
                                label.push('(');
                            }
                            label.push_str(self_label);
                            if self_loop.prec <= Prec::Quant {
                                label.push(')');
                            }
                            label.push('+');
                            Some((
                                Arrow {
                                    label: Some(label),
                                    prec: Prec::Concat,
                                },
                                outbound.clone(),
                            ))
                        }
                    } else {
                        None
                    };

                    if let Some(pair) = inbound_plus {
                        pair
                    } else if outbound.prec >= Prec::Concat
                        && self_loop.prec >= Prec::Concat
                        && out_label.len() >= self_label.len()
                        && out_label.starts_with(self_label)
                    {
                        let diff = out_label.len() - self_label.len();
                        let mut label = String::new();
                        if self_loop.prec <= Prec::Quant {
                            label.push('(');
                        }
                        label.push_str(self_label);
                        if self_loop.prec <= Prec::Quant {
                            label.push(')');
                        }
                        label.push('+');
                        if diff != 0 && outbound.prec < Prec::Concat {
                            label.push('(');
                        }
                        label.push_str(&out_label[diff..diff + diff.min(out_label.len() - diff)]);
                        if diff != 0 && outbound.prec < Prec::Concat {
                            label.push(')');
                        }
                        (
                            inbound.clone(),
                            Arrow {
                                label: Some(label),
                                prec: Prec::Concat,
                            },
                        )
                    } else {
                        let mut label = String::new();
                        if self_loop.prec <= Prec::Quant {
                            label.push('(');
                        }
                        label.push_str(self_label);
                        if self_loop.prec <= Prec::Quant {
                            label.push(')');
                        }
                        label.push('*');
                        if outbound.prec < Prec::Concat {
                            label.push('(');
                        }
                        label.push_str(&out_label);
                        if outbound.prec < Prec::Concat {
                            label.push(')');
                        }
                        (
                            inbound.clone(),
                            Arrow {
                                label: Some(label),
                                prec: Prec::Concat,
                            },
                        )
                    }
                };

                let bypass = match (first.label.as_deref(), second.label.as_deref()) {
                    (Some(""), _) => second.clone(),
                    (_, Some("")) => first.clone(),
                    (Some(first_label), Some(second_label)) => {
                        let mut label = String::new();
                        if first.prec < Prec::Concat {
                            label.push('(');
                        }
                        label.push_str(first_label);
                        if first.prec < Prec::Concat {
                            label.push(')');
                        }
                        if second.prec < Prec::Concat {
                            label.push('(');
                        }
                        label.push_str(second_label);
                        if second.prec < Prec::Concat {
                            label.push(')');
                        }
                        Arrow {
                            label: Some(label),
                            prec: Prec::Concat,
                        }
                    }
                    _ => Arrow {
                        label: None,
                        prec: Prec::Symset,
                    },
                };

                let merged = match (existing.label.as_deref(), bypass.label.as_deref()) {
                    (_, None) => existing,
                    (None, Some(_)) => bypass,
                    (Some(""), Some(bypass_label)) => {
                        let mut label = String::new();
                        if bypass.prec <= Prec::Quant {
                            label.push('(');
                        }
                        label.push_str(bypass_label);
                        if bypass.prec <= Prec::Quant {
                            label.push(')');
                        }
                        label.push('?');
                        Arrow {
                            label: Some(label),
                            prec: Prec::Quant,
                        }
                    }
                    (Some(existing_label), Some(bypass_label)) => Arrow {
                        label: Some(format!("{existing_label}|{bypass_label}")),
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
        let chr = self.peek()?;
        self.pos += 1;
        Some(chr)
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn expect_char(&mut self) -> Result<u8, String> {
        parse_symbol(self)
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

        let offset = re.states.len();
        let mut map = Vec::with_capacity(alt.states.len());
        for _ in 0..alt.states.len() {
            map.push(re.states.len());
            re.states.push(NState::new());
        }
        for (idx, state) in alt.states.into_iter().enumerate() {
            re.states[map[idx]] = remap_state(state, &map);
        }

        re.states[re.initial].epsilon1 = Some(map[alt.initial]);
        re.states[re.final_].epsilon0 = Some(map[alt.final_]);
        re.final_ = map[alt.final_];

        if offset == usize::MAX {
            return Err("state space overflow".to_string());
        }
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
    while !matches!(ctx.peek(), Some(b')') | Some(b'|') | Some(b'&') | None) {
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

    if ctx.peek() == Some(b'*') {
        ctx.next();
        nfa_uncomplement(&mut atom)?;
        let initial = atom.initial;
        let final_ = atom.final_;
        atom.states[final_].epsilon1 = Some(initial);
        nfa_pad_initial(&mut atom);
        nfa_pad_final(&mut atom);
        atom.states[atom.initial].epsilon1 = Some(atom.final_);
        return Ok(atom);
    }

    if ctx.peek() == Some(b'+') {
        ctx.next();
        nfa_uncomplement(&mut atom)?;
        let initial = atom.initial;
        let final_ = atom.final_;
        atom.states[final_].epsilon1 = Some(initial);
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

    if ctx.peek() == Some(b'{') {
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
            return Err("misbounded quantifier".to_string());
        }

        let mut atoms = Nfa::new_single();
        let mut i = 0u32;
        while if max_unbounded { i <= min } else { i < max } {
            let mut clone = nfa_clone(&atom);
            if i >= min {
                if max_unbounded {
                    let init = clone.initial;
                    let fin = clone.final_;
                    clone.states[fin].epsilon1 = Some(init);
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
        return Ok(atoms);
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

    let label = parse_symset(ctx)?;
    Ok(Nfa {
        states: vec![
            NState {
                label,
                target: Some(1),
                epsilon0: None,
                epsilon1: None,
            },
            NState::new(),
        ],
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

    let mut symset = if let Some(set) = parse_shorthand(ctx)? {
        set
    } else if ctx.peek() == Some(b'[') {
        ctx.next();
        let mut set = SymSet::empty();
        while !matches!(ctx.peek(), Some(b']') | None) {
            let sub = parse_symset(ctx)?;
            set.union_with(&sub);
        }
        if ctx.peek() != Some(b']') {
            return Err("expected ']'".to_string());
        }
        ctx.next();
        set
    } else if ctx.peek() == Some(b'<') {
        ctx.next();
        let mut set = SymSet::full();
        while !matches!(ctx.peek(), Some(b'>') | None) {
            let sub = parse_symset(ctx)?;
            set.intersect_with(&sub);
        }
        if ctx.peek() != Some(b'>') {
            return Err("expected '>'".to_string());
        }
        ctx.next();
        set
    } else {
        let begin = parse_symbol(ctx)?;
        let end = if ctx.peek() == Some(b'-') {
            ctx.next();
            parse_symbol(ctx)?
        } else {
            begin
        };

        let mut set = SymSet::empty();
        let mut chr = begin;
        loop {
            set.insert(chr);
            if chr == end {
                break;
            }
            chr = chr.wrapping_add(1);
        }
        set
    };

    if complement {
        symset.invert();
    }
    Ok(symset)
}

fn parse_shorthand(ctx: &mut ParseContext) -> Result<Option<SymSet>, String> {
    match (ctx.peek(), ctx.chars.get(ctx.pos + 1).copied()) {
        (Some(b'\\'), Some(b'd')) => {
            ctx.pos += 2;
            Ok(Some(digits_set()))
        }
        (Some(b'\\'), Some(b'D')) => {
            ctx.pos += 2;
            Ok(Some(not_digits_set()))
        }
        (Some(b'\\'), Some(b's')) => {
            ctx.pos += 2;
            Ok(Some(spaces_set()))
        }
        (Some(b'\\'), Some(b'S')) => {
            ctx.pos += 2;
            Ok(Some(not_spaces_set()))
        }
        (Some(b'\\'), Some(b'w')) => {
            ctx.pos += 2;
            Ok(Some(wordchar_set()))
        }
        (Some(b'\\'), Some(b'W')) => {
            ctx.pos += 2;
            Ok(Some(not_wordchar_set()))
        }
        (Some(b'.'), _) => {
            ctx.pos += 1;
            let mut set = SymSet::full();
            set.bits[(b'\n' / 8) as usize] &= !(1 << (b'\n' % 8));
            Ok(Some(set))
        }
        _ => Ok(None),
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
    for chr in [b' ', b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
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
    if !matches!(ctx.peek(), Some(b'0'..=b'9')) {
        return Err("expected natural number".to_string());
    }

    let mut natural = 0u32;
    while let Some(chr @ b'0'..=b'9') = ctx.peek() {
        let digit = (chr - b'0') as u32;
        natural = natural
            .checked_mul(10)
            .and_then(|n| n.checked_add(digit))
            .ok_or_else(|| "natural number overflow".to_string())?;
        ctx.next();
    }
    Ok(natural)
}

fn shift_option(opt: &mut Option<usize>, offset: usize) {
    if let Some(idx) = opt.as_mut() {
        *idx += offset;
    }
}

fn parse_hexbyte(ctx: &mut ParseContext) -> Result<u8, String> {
    let mut byte = 0u8;
    for _ in 0..2 {
        byte <<= 4;
        let chr = ctx.next().ok_or_else(|| "expected hex digit".to_string())?;
        match chr {
            b'0'..=b'9' => byte |= chr - b'0',
            b'a'..=b'f' => byte |= chr - b'a' + 10,
            b'A'..=b'F' => byte |= chr - b'A' + 10,
            _ => return Err("expected hex digit".to_string()),
        }
    }
    Ok(byte)
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
        Some(b'x') => parse_hexbyte(ctx),
        Some(_) | None => Err("unknown escape".to_string()),
    }
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
    if !matches!(chr, 0x20..=0x7e) {
        return Err("unexpected nonprintable character".to_string());
    }
    ctx.next();
    Ok(chr)
}
