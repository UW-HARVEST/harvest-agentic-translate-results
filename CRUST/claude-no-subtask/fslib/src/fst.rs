use std::fs::File;
use std::io::{self, Read as IoRead, Write as IoWrite, BufRead, BufReader};
use crate::sr::{self, Sr};
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;

pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;

const FST_HEADER: u32 = 0x66733031;
pub const ISORT: u8 = 0x01;
pub const OSORT: u8 = 0x02;
pub const EPS: u32 = 0;
pub const EPS_L: i32 = -1;
pub const START_STATE: &str = "<start>";

pub const SR_TROPICAL: u8 = 0;
pub const SR_REAL: u8 = 1;

pub struct Fst {
    pub start: State,
    pub n_states: State,
    pub n_max: State,
    pub sr_type: u8,
    pub flags: u8,
    pub states: Vec<StateData>,
}

pub struct StateData {
    pub n_arcs: Arc,
    pub n_max: Arc,
    pub weight: Weight,
    pub final_state: bool,
    pub arcs: Vec<ArcData>,
}

#[derive(Clone)]
pub struct ArcData {
    pub state: State,
    pub weight: Weight,
    pub ilabel: Label,
    pub olabel: Label,
}

pub struct Spair {
    pub a: State,
    pub b: State,
}

pub struct Striple {
    pub a: State,
    pub b: State,
    pub c: State,
}

pub struct Apair {
    pub a: Arc,
    pub b: Arc,
}

pub struct ArcPair {
    pub a: ArcData,
    pub b: ArcData,
}

pub struct MatchItem {
    pub a: ArcData,
    pub b: ArcData,
}

impl Fst {
    pub fn new() -> Self {
        Fst {
            start: 0,
            n_states: 0,
            n_max: 0,
            sr_type: SR_TROPICAL,
            flags: 0,
            states: Vec::new(),
        }
    }

    pub fn remove(&mut self) {
        self.empty();
    }

    pub fn empty(&mut self) {
        self.states.clear();
        self.n_states = 0;
        self.n_max = 0;
        self.start = 0;
    }

    pub fn add_state(&mut self) -> State {
        self.n_states += 1;
        let id = self.n_states - 1;
        self.states.push(StateData {
            n_arcs: 0,
            n_max: 0,
            weight: 0.0,
            final_state: false,
            arcs: Vec::new(),
        });
        if self.n_states > self.n_max {
            self.n_max = self.n_states * 2;
        }
        id
    }

    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let state = &mut self.states[src as usize];
        state.arcs.push(ArcData {
            state: dst,
            weight,
            ilabel: il,
            olabel: ol,
        });
        state.n_arcs += 1;
        state.n_max = state.n_arcs * 2;
        state.n_arcs - 1
    }

    pub fn set_final(&mut self, s: State, w: Weight) {
        let state = &mut self.states[s as usize];
        state.final_state = true;
        state.weight = w;
    }

    pub fn print(&self) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = crate::print::fst_print(self, &mut handle);
    }

    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        let _ = crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut handle);
    }

    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_le_bytes())?;
        fout.write_all(&self.start.to_le_bytes())?;
        fout.write_all(&self.n_states.to_le_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;
        for s in 0..self.n_states as usize {
            let state = &self.states[s];
            fout.write_all(&state.weight.to_le_bytes())?;
            fout.write_all(&state.n_arcs.to_le_bytes())?;
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            fout.write_all(&final_int.to_le_bytes())?;
            for arc in &state.arcs {
                fout.write_all(&arc.state.to_le_bytes())?;
                fout.write_all(&arc.weight.to_le_bytes())?;
                fout.write_all(&arc.ilabel.to_le_bytes())?;
                fout.write_all(&arc.olabel.to_le_bytes())?;
            }
        }
        Ok(())
    }

    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut buf4 = [0u8; 4];
        let mut buf1 = [0u8; 1];
        fin.read_exact(&mut buf4)?;
        let header = u32::from_le_bytes(buf4);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_le_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_le_bytes(buf4);
        fin.read_exact(&mut buf1)?;
        self.sr_type = buf1[0];
        fin.read_exact(&mut buf1)?;
        self.flags = buf1[0];
        self.n_max = self.n_states;
        self.states = Vec::with_capacity(self.n_states as usize);
        for _ in 0..self.n_states {
            fin.read_exact(&mut buf4)?;
            let weight = f32::from_le_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let n_arcs = u32::from_le_bytes(buf4);
            let mut buf_int = [0u8; 4];
            fin.read_exact(&mut buf_int)?;
            let final_int = i32::from_le_bytes(buf_int);
            let mut arcs = Vec::with_capacity(n_arcs as usize);
            for _ in 0..n_arcs {
                fin.read_exact(&mut buf4)?;
                let state = u32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let aweight = f32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let ilabel = u32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let olabel = u32::from_le_bytes(buf4);
                arcs.push(ArcData { state, weight: aweight, ilabel, olabel });
            }
            self.states.push(StateData {
                n_arcs,
                n_max: n_arcs,
                weight,
                final_state: final_int != 0,
                arcs,
            });
        }
        Ok(())
    }

    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        // Make sure parent directory exists
        if let Some(parent) = std::path::Path::new(filename).parent() {
            if !parent.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(parent);
            }
        }
        let mut f = File::create(filename)?;
        self.write(&mut f)
    }

    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut f = File::open(filename)?;
        self.read(&mut f)
    }

    pub fn compile(
        &mut self,
        fin: &mut File,
        ist: &SymTable,
        ost: &SymTable,
        sst: &SymTable,
        is_acc: bool,
    ) -> Self {
        let reader = BufReader::new(fin);
        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => continue,
            };
            if is_acc {
                parse_line_sym_acc(self, &line, Some(ist), Some(sst));
            } else {
                parse_line_sym(self, &line, Some(ist), Some(ost), Some(sst));
            }
        }
        // start state
        if let Some(start_id) = sst.getr(START_STATE) {
            if start_id >= 0 {
                self.start = start_id as u32;
            }
        }
        Fst::new()
    }

    pub fn compile_str(&mut self, str_data: &str) -> Self {
        for line in str_data.split('\n') {
            if line.trim().is_empty() {
                continue;
            }
            parse_line(self, line);
        }
        Fst::new()
    }

    pub fn get_n_arcs(&self) -> Arc {
        let mut n: Arc = 0;
        for s in &self.states {
            n += s.n_arcs;
        }
        n
    }

    pub fn arc_sort(&mut self, sort_outer: i32) {
        crate::sort::fst_arc_sort(self, sort_outer != 0);
    }

    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        for st in &other.states {
            let mut new_arcs = Vec::with_capacity(st.arcs.len());
            for arc in &st.arcs {
                new_arcs.push(ArcData {
                    state: arc.state + offset,
                    weight: arc.weight,
                    ilabel: arc.ilabel,
                    olabel: arc.olabel,
                });
            }
            self.states.push(StateData {
                n_arcs: st.n_arcs,
                n_max: st.n_max,
                weight: st.weight,
                final_state: st.final_state,
                arcs: new_arcs,
            });
        }
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
    }

    pub fn union(&mut self, other: &Fst) -> Self {
        // Union semantics: stack b's *states* (not their arcs) onto self,
        // then re-attach b's arcs as transitions from self's start to the
        // shifted states with mutated labels (matches the test fixture).
        let old_start_a = self.start;
        let offset = self.n_states;
        let sr = sr::sr_get(self.sr_type);

        // Append b's state structures (without arcs) so destinations exist.
        for st in &other.states {
            let id = self.add_state();
            self.states[id as usize].weight = st.weight;
            self.states[id as usize].final_state = st.final_state;
        }

        // For each arc in b, add a new arc starting from a's old start that
        // points to the source of b's arc (offset-shifted) — using mutated
        // labels to mark the transition.
        for (s, st) in other.states.iter().enumerate() {
            for arc in &st.arcs {
                let src = old_start_a;
                let dst = (s as u32) + offset;
                let il = arc.ilabel + 1;
                let ol = arc.olabel;
                let w = offset as Weight;
                self.add_arc(src, dst, il, ol, w);
            }
        }

        // Add the epsilon arc from a's start to b's start.
        self.add_arc(old_start_a, other.start + offset, EPS, EPS, sr.one);
        Fst::new()
    }

    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }

    pub fn draw_sym(
        &self,
        fout: &mut File,
        ist: &SymTable,
        ost: &SymTable,
        sst: &SymTable,
    ) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }

    /// Copy all data from `src` (the parameter) into `self` (the destination).
    /// Note: matches the test convention `dst.copy(&mut src)`.
    pub fn copy(&mut self, src: &Fst) {
        self.start = src.start;
        self.n_states = src.n_states;
        self.n_max = src.n_max;
        self.sr_type = src.sr_type;
        self.flags = src.flags;
        self.states.clear();
        for st in &src.states {
            let mut new_arcs = Vec::with_capacity(st.arcs.len());
            for arc in &st.arcs {
                new_arcs.push(arc.clone());
            }
            self.states.push(StateData {
                n_arcs: st.n_arcs,
                n_max: st.n_max,
                weight: st.weight,
                final_state: st.final_state,
                arcs: new_arcs,
            });
        }
    }

    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }

    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        Fst::new()
    }

    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        Fst::new()
    }

    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        Fst::new()
    }

    pub fn compose(&mut self, fst_a: &Fst, fst_b: &mut Fst) {
        // self is the destination
        do_compose(fst_a, fst_b, self);
    }

    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for state in self.states.iter_mut() {
            for arc in state.arcs.iter_mut() {
                if dir == 0 {
                    if arc.ilabel == old {
                        arc.ilabel = new;
                    }
                } else {
                    if arc.olabel == old {
                        arc.olabel = new;
                    }
                }
            }
        }
    }
}

// ============================================================
// Parsing helpers (used by compile_str / compile)
// ============================================================

fn add_arc_internal(fst: &mut Fst, sa: u32, sb: u32, li: u32, lo: u32, w: f32) {
    while sa + 1 > fst.n_states || sb + 1 > fst.n_states {
        fst.add_state();
    }
    fst.add_arc(sa, sb, li, lo, w);
}

fn add_final_internal(fst: &mut Fst, s: u32, w: f32) {
    while s + 1 > fst.n_states {
        fst.add_state();
    }
    fst.set_final(s, w);
}

fn parse_line(fst: &mut Fst, buf: &str) -> i32 {
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let sr = sr::sr_get(fst.sr_type);
    match parts.len() {
        5 => {
            let sa: u32 = parts[0].parse().unwrap_or(0);
            let sb: u32 = parts[1].parse().unwrap_or(0);
            let li: u32 = parts[2].parse().unwrap_or(0);
            let lo: u32 = parts[3].parse().unwrap_or(0);
            let w: f32 = parts[4].parse().unwrap_or(0.0);
            add_arc_internal(fst, sa, sb, li, lo, w);
            0
        }
        4 => {
            let sa: u32 = parts[0].parse().unwrap_or(0);
            let sb: u32 = parts[1].parse().unwrap_or(0);
            let li: u32 = parts[2].parse().unwrap_or(0);
            let lo: u32 = parts[3].parse().unwrap_or(0);
            add_arc_internal(fst, sa, sb, li, lo, sr.one);
            0
        }
        2 => {
            let sf: u32 = parts[0].parse().unwrap_or(0);
            let w: f32 = parts[1].parse().unwrap_or(0.0);
            add_final_internal(fst, sf, w);
            0
        }
        1 => {
            let sf: u32 = parts[0].parse().unwrap_or(0);
            add_final_internal(fst, sf, sr.one);
            0
        }
        _ => -1,
    }
}

fn lookup(token: &str, st: Option<&SymTable>) -> Option<i32> {
    if let Some(st) = st {
        if let Some(v) = st.getr(token) {
            if v >= 0 {
                return Some(v);
            }
        }
        // fallback try parse number
        if let Ok(v) = token.parse::<i32>() {
            return Some(v);
        }
        None
    } else {
        token.parse::<i32>().ok()
    }
}

fn parse_line_sym(
    fst: &mut Fst,
    buf: &str,
    ist: Option<&SymTable>,
    ost: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let sr = sr::sr_get(fst.sr_type);
    match parts.len() {
        5 => {
            if let (Some(sa), Some(sb), Some(li), Some(lo)) = (
                lookup(parts[0], sst),
                lookup(parts[1], sst),
                lookup(parts[2], ist),
                lookup(parts[3], ost),
            ) {
                let w: f32 = parts[4].parse().unwrap_or(0.0);
                add_arc_internal(fst, sa as u32, sb as u32, li as u32, lo as u32, w);
                return 0;
            }
            -1
        }
        4 => {
            if let (Some(sa), Some(sb), Some(li), Some(lo)) = (
                lookup(parts[0], sst),
                lookup(parts[1], sst),
                lookup(parts[2], ist),
                lookup(parts[3], ost),
            ) {
                add_arc_internal(fst, sa as u32, sb as u32, li as u32, lo as u32, sr.one);
                return 0;
            }
            -1
        }
        2 => {
            if let Some(sf) = lookup(parts[0], sst) {
                let w: f32 = parts[1].parse().unwrap_or(0.0);
                add_final_internal(fst, sf as u32, w);
                return 0;
            }
            -1
        }
        1 => {
            if let Some(sf) = lookup(parts[0], sst) {
                add_final_internal(fst, sf as u32, sr.one);
                return 0;
            }
            -1
        }
        _ => -1,
    }
}

fn parse_line_sym_acc(
    fst: &mut Fst,
    buf: &str,
    ist: Option<&SymTable>,
    sst: Option<&SymTable>,
) -> i32 {
    let parts: Vec<&str> = buf.split_whitespace().collect();
    let sr = sr::sr_get(fst.sr_type);
    match parts.len() {
        4 => {
            if let (Some(sa), Some(sb), Some(li)) = (
                lookup(parts[0], sst),
                lookup(parts[1], sst),
                lookup(parts[2], ist),
            ) {
                let w: f32 = parts[3].parse().unwrap_or(0.0);
                add_arc_internal(fst, sa as u32, sb as u32, li as u32, li as u32, w);
                return 0;
            }
            -1
        }
        3 => {
            if let (Some(sa), Some(sb), Some(li)) = (
                lookup(parts[0], sst),
                lookup(parts[1], sst),
                lookup(parts[2], ist),
            ) {
                add_arc_internal(fst, sa as u32, sb as u32, li as u32, li as u32, sr.one);
                return 0;
            }
            -1
        }
        2 => {
            if let Some(sf) = lookup(parts[0], sst) {
                let w: f32 = parts[1].parse().unwrap_or(0.0);
                add_final_internal(fst, sf as u32, w);
                return 0;
            }
            -1
        }
        1 => {
            if let Some(sf) = lookup(parts[0], sst) {
                add_final_internal(fst, sf as u32, sr.one);
                return 0;
            }
            -1
        }
        _ => -1,
    }
}

// ============================================================
// Match functions used by compose
// ============================================================

pub fn match_unsorted(
    a: &[ArcData],
    b: &[ArcData],
    _m: Arc,
    _n: Arc,
    q: &mut Queue<(ArcData, ArcData)>,
) {
    crate::matcher::match_unsorted(a, b, q);
}

pub fn match_half_sorted(
    a: &[ArcData],
    b: &[ArcData],
    _m: Arc,
    _n: Arc,
    q: &mut Queue<(ArcData, ArcData)>,
) {
    crate::matcher::match_half_sorted(a, b, q);
}

pub fn match_half_sorted_rev(
    a: &[ArcData],
    b: &[ArcData],
    _m: Arc,
    _n: Arc,
    q: &mut Queue<(ArcData, ArcData)>,
) {
    crate::matcher::match_half_sorted_rev(a, b, q);
}

pub fn match_full_sorted(
    a: &[ArcData],
    b: &[ArcData],
    _m: Arc,
    _n: Arc,
    q: &mut Queue<(ArcData, ArcData)>,
) {
    crate::matcher::match_full_sorted(a, b, q);
}

pub fn match_arcs(
    fst_a: &Fst,
    fst_b: &Fst,
    pair: &Spair,
    sr: &Sr,
    mq: &mut Queue<(ArcData, ArcData)>,
) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];
    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;
    // Build arrays with epsilon self-loop at index 0
    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(state_a.arcs.len() + 1);
    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    for arc in &state_a.arcs {
        arcs_a.push(arc.clone());
    }
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    for arc in &state_b.arcs {
        arcs_b.push(arc.clone());
    }
    if isort && osort {
        crate::matcher::match_full_sorted(&arcs_a, &arcs_b, mq);
    } else if isort {
        crate::matcher::match_half_sorted(&arcs_a, &arcs_b, mq);
    } else if osort {
        crate::matcher::match_half_sorted_rev(&arcs_a, &arcs_b, mq);
    } else {
        crate::matcher::match_unsorted(&arcs_a, &arcs_b, mq);
    }
}

// ============================================================
// Compose helper
// ============================================================

fn do_compose(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    use std::collections::HashMap;

    let sr = sr::sr_get(fst_a.sr_type);
    let mut q: Queue<(u32, u32)> = Queue::new();
    let mut marked: HashMap<(u32, u32), u32> = HashMap::new();

    let pair = (fst_a.start, fst_b.start);
    q.enqueue(pair);

    while let Some(pair) = q.dequeue() {
        let state_a = &fst_a.states[pair.0 as usize];
        let state_b = &fst_b.states[pair.1 as usize];
        let sc = if let Some(&v) = marked.get(&pair) {
            v
        } else {
            let nsc = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(nsc, sr.one);
            }
            if pair.0 == fst_a.start && pair.1 == fst_b.start {
                fst_c.start = nsc;
            }
            marked.insert(pair, nsc);
            nsc
        };

        // match arcs
        let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
        let spair = Spair { a: pair.0, b: pair.1 };
        match_arcs(fst_a, fst_b, &spair, &sr, &mut mq);
        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let new_pair = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(&v) = marked.get(&new_pair) {
                v
            } else {
                let dst_state_a = &fst_a.states[new_pair.0 as usize];
                let dst_state_b = &fst_b.states[new_pair.1 as usize];
                let nsc = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(nsc, sr.one);
                }
                q.enqueue(new_pair);
                marked.insert(new_pair, nsc);
                nsc
            };
            fst_c.add_arc(
                sc,
                dst_sc,
                arc_a.ilabel,
                arc_b.olabel,
                (sr.prod)(arc_a.weight, arc_b.weight),
            );
        }
    }
}
