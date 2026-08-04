use std::fs::File;
use std::io::{self, Read, Write};
use crate::sr::Sr;
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::collections::VecDeque;
pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;
const FST_HEADER: u32 = 0x66733031;
pub const ISORT: u8 = 0x01;
pub const OSORT: u8 = 0x02;
pub const EPS: u32 = 0;
const EPS_L: i32 = -1;
pub const START_STATE: &str = "<start>";

pub const SR_TROPICAL: u8 = 0;
pub const SR_REAL: u8 = 1;

#[derive(Clone)]
pub struct Fst {
    pub start: State,
    pub n_states: State,
    pub n_max: State,
    pub sr_type: u8,
    pub flags: u8,
    pub states: Vec<StateData>,
}
#[derive(Clone)]
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
        if self.n_states > self.n_max {
            self.n_max = self.n_states * 2;
        }
        self.states.push(StateData {
            n_arcs: 0,
            n_max: 0,
            weight: 0.0,
            final_state: false,
            arcs: Vec::new(),
        });
        self.n_states - 1
    }
    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let state = &mut self.states[src as usize];
        state.n_arcs += 1;
        if state.n_arcs > state.n_max {
            state.n_max = state.n_arcs * 2;
        }
        state.arcs.push(ArcData {
            state: dst,
            ilabel: il,
            olabel: ol,
            weight,
        });
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        let state = &mut self.states[s as usize];
        state.final_state = true;
        state.weight = w;
    }
    pub fn print(&self) {
        crate::print::fst_print(self, &mut std::io::stdout()).unwrap();
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        crate::print::fst_print_sym(
            self, Some(ist), Some(ost), Some(sst), &mut std::io::stdout()
        ).unwrap();
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_ne_bytes())?;
        fout.write_all(&self.start.to_ne_bytes())?;
        fout.write_all(&self.n_states.to_ne_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;
        for s in 0..self.n_states as usize {
            let state = &self.states[s];
            fout.write_all(&state.weight.to_ne_bytes())?;
            fout.write_all(&state.n_arcs.to_ne_bytes())?;
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            fout.write_all(&final_int.to_ne_bytes())?;
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                fout.write_all(&arc.state.to_ne_bytes())?;
                fout.write_all(&arc.weight.to_ne_bytes())?;
                fout.write_all(&arc.ilabel.to_ne_bytes())?;
                fout.write_all(&arc.olabel.to_ne_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut buf4 = [0u8; 4];
        let mut buf1 = [0u8; 1];
        fin.read_exact(&mut buf4)?;
        let header = u32::from_ne_bytes(buf4);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_ne_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_ne_bytes(buf4);
        fin.read_exact(&mut buf1)?;
        self.sr_type = buf1[0];
        fin.read_exact(&mut buf1)?;
        self.flags = buf1[0];
        self.n_max = self.n_states;
        self.states = Vec::with_capacity(self.n_states as usize);
        for _ in 0..self.n_states as usize {
            fin.read_exact(&mut buf4)?;
            let weight = f32::from_ne_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let n_arcs = u32::from_ne_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let final_int = i32::from_ne_bytes(buf4);
            let final_state = final_int != 0;
            let mut state = StateData {
                n_arcs,
                n_max: n_arcs,
                weight,
                final_state,
                arcs: Vec::with_capacity(n_arcs as usize),
            };
            for _ in 0..n_arcs as usize {
                fin.read_exact(&mut buf4)?;
                let astate = u32::from_ne_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let aweight = f32::from_ne_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let ilabel = u32::from_ne_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let olabel = u32::from_ne_bytes(buf4);
                state.arcs.push(ArcData {
                    state: astate,
                    weight: aweight,
                    ilabel,
                    olabel,
                });
            }
            self.states.push(state);
        }
        Ok(())
    }
    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        let mut fout = File::create(filename)?;
        self.write(&mut fout)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut fin = File::open(filename)?;
        self.read(&mut fin)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        // Stub - read whole file and compile from string
        use std::io::Read;
        let mut s = String::new();
        let mut f = fin;
        let _ = f.read_to_string(&mut s);
        self.compile_str(&s)
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        crate::compile::fst_compile_str(self, str_data);
        self.clone()
    }
    pub fn get_n_arcs(&self) -> Arc {
        let mut n: Arc = 0;
        for s in 0..self.n_states as usize {
            n += self.states[s].n_arcs;
        }
        n
    }
    pub fn arc_sort(&mut self, sort_outer: i32) {
        crate::sort::fst_arc_sort(self, sort_outer != 0);
    }
    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        // Append cloned states from other; remap arcs' destinations
        for s in 0..other.n_states as usize {
            let mut new_state = other.states[s].clone();
            for arc in new_state.arcs.iter_mut() {
                arc.state += offset;
            }
            self.states.push(new_state);
        }
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        self.stack(other);
        self.clone()
    }
    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }
    pub fn draw_sym(&self, fout: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }
    pub fn copy(&mut self, src: &Fst) {
        self.start = src.start;
        self.n_states = src.n_states;
        self.n_max = src.n_max;
        self.sr_type = src.sr_type;
        self.flags = src.flags;
        self.states = src.states.clone();
    }
    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }
    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        path.clone()
    }
    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        self.clone()
    }
    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        self.clone()
    }
    pub fn compose(&mut self, fst_a: &Fst, fst_b: &mut Fst) {
        // Self plays the role of fst_c
        compose_fsts(fst_a, fst_b, self);
    }
    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for s in 0..self.n_states as usize {
            let state = &mut self.states[s];
            for a in 0..state.n_arcs as usize {
                let arc = &mut state.arcs[a];
                if dir == 0 {
                    if arc.ilabel == old { arc.ilabel = new; }
                } else {
                    if arc.olabel == old { arc.olabel = new; }
                }
            }
        }
    }
}
pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..m as usize {
        for j in 0..n as usize {
            if a[i].olabel == b[j].ilabel && _match(a, b, i, j) {
                q.enqueue((a[i].clone(), b[j].clone()));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mm = m as i64;
    let nn = n as i64;
    for i in 0..mm as usize {
        let mut l: i64 = 0;
        let mut h: i64 = nn - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if a[i].olabel > b[mid].ilabel {
                l = mid as i64 + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 { break; }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && a[i].olabel == b[ll - 1].ilabel { ll -= 1; }
                while (hh as i64) < h && a[i].olabel == b[hh + 1].ilabel { hh += 1; }
                while ll <= hh {
                    if _match(a, b, i, ll) {
                        q.enqueue((a[i].clone(), b[ll].clone()));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mm = m as i64;
    let nn = n as i64;
    for i in 0..nn as usize {
        let mut l: i64 = 0;
        let mut h: i64 = mm - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if b[i].ilabel > a[mid].olabel {
                l = mid as i64 + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 { break; }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && b[i].ilabel == a[ll - 1].olabel { ll -= 1; }
                while (hh as i64) < h && b[i].ilabel == a[hh + 1].olabel { hh += 1; }
                while ll <= hh {
                    if _match(a, b, ll, i) {
                        q.enqueue((a[ll].clone(), b[i].clone()));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < m as usize && j < n as usize {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n as usize && a[i].olabel == b[t].ilabel {
                if _match(a, b, i, t) {
                    q.enqueue((a[i].clone(), b[t].clone()));
                }
                t += 1;
            }
            i += 1;
        }
    }
}
fn _match(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}
pub fn match_arcs(fst_a: &Fst, fst_b: &Fst, pair: &Spair, sr: &Sr, mq: &mut Queue<(ArcData, ArcData)>) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];

    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;

    let m = state_a.n_arcs + 1;
    let n = state_b.n_arcs + 1;

    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(m as usize);
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(n as usize);

    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    for arc in &state_a.arcs {
        arcs_a.push(arc.clone());
    }
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
        match_full_sorted(&arcs_a, &arcs_b, m, n, mq);
    } else if isort || osort {
        if isort {
            match_half_sorted(&arcs_a, &arcs_b, m, n, mq);
        } else {
            match_half_sorted_rev(&arcs_a, &arcs_b, m, n, mq);
        }
    } else {
        match_unsorted(&arcs_a, &arcs_b, m, n, mq);
    }
}

pub fn compose_fsts(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    use std::collections::HashMap;
    let sr = crate::sr::sr_get(fst_a.sr_type);
    fst_c.empty();
    fst_c.sr_type = fst_a.sr_type;

    let mut q: VecDeque<(State, State)> = VecDeque::new();
    let mut marked: HashMap<(State, State), State> = HashMap::new();

    let init_pair = (fst_a.start, fst_b.start);
    q.push_back(init_pair);

    while let Some(pair) = q.pop_front() {
        let state_a = &fst_a.states[pair.0 as usize];
        let state_b = &fst_b.states[pair.1 as usize];

        let sc = if let Some(&existing) = marked.get(&pair) {
            existing
        } else {
            let new_sc = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(new_sc, sr.one);
            }
            if pair.0 == fst_a.start && pair.1 == fst_b.start {
                fst_c.start = new_sc;
            }
            marked.insert(pair, new_sc);
            new_sc
        };

        // Match arcs
        let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
        let spair = Spair { a: pair.0, b: pair.1 };
        match_arcs(fst_a, fst_b, &spair, &sr, &mut mq);

        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let new_pair = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(&existing) = marked.get(&new_pair) {
                existing
            } else {
                let dst_state_a = &fst_a.states[new_pair.0 as usize];
                let dst_state_b = &fst_b.states[new_pair.1 as usize];
                let new_dst_sc = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(new_dst_sc, sr.one);
                }
                q.push_back(new_pair);
                marked.insert(new_pair, new_dst_sc);
                new_dst_sc
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
