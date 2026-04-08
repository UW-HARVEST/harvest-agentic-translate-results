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
const ISORT: u8 = 0x01;
const OSORT: u8 = 0x02;
const EPS: u32 = 0;
const EPS_L: i32 = -1;
pub const START_STATE: &str = "<start>";
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
            sr_type: 0, // SR_TROPICAL
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
        state.arcs.push(ArcData { state: dst, weight, ilabel: il, olabel: ol });
        state.n_arcs += 1;
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        self.states[s as usize].final_state = true;
        self.states[s as usize].weight = w;
    }
    pub fn print(&self) {
        crate::print::fst_print(self, &mut io::stdout()).ok();
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut io::stdout()).ok();
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_le_bytes())?;
        fout.write_all(&self.start.to_le_bytes())?;
        fout.write_all(&self.n_states.to_le_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;
        for state in &self.states {
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
            fin.read_exact(&mut buf4)?;
            let final_int = i32::from_le_bytes(buf4);
            let mut arcs = Vec::with_capacity(n_arcs as usize);
            for _ in 0..n_arcs {
                let mut abuf = [0u8; 4];
                fin.read_exact(&mut abuf)?;
                let astate = u32::from_le_bytes(abuf);
                fin.read_exact(&mut abuf)?;
                let aweight = f32::from_le_bytes(abuf);
                fin.read_exact(&mut abuf)?;
                let ailabel = u32::from_le_bytes(abuf);
                fin.read_exact(&mut abuf)?;
                let aolabel = u32::from_le_bytes(abuf);
                arcs.push(ArcData { state: astate, weight: aweight, ilabel: ailabel, olabel: aolabel });
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
        let mut f = File::create(filename)?;
        self.write(&mut f)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut f = File::open(filename)?;
        self.read(&mut f)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        // Delegate to compile module - but signature returns Self, so we build in place
        // This is a stub that matches the signature; real work is in compile.rs
        let mut fst = Fst::new();
        std::mem::swap(self, &mut fst);
        fst
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        let mut fst = Fst::new();
        std::mem::swap(self, &mut fst);
        fst
    }
    pub fn get_n_arcs(&self) -> Arc {
        self.states.iter().map(|s| s.n_arcs).sum()
    }
    pub fn arc_sort(&mut self, sort_outer: i32) {
        if sort_outer == 0 {
            self.flags |= ISORT;
            for state in self.states.iter_mut() {
                state.arcs.sort_by(|a, b| a.ilabel.cmp(&b.ilabel));
            }
        } else {
            self.flags |= OSORT;
            for state in self.states.iter_mut() {
                state.arcs.sort_by(|a, b| a.olabel.cmp(&b.olabel));
            }
        }
    }
    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        for state in &other.states {
            let mut new_arcs: Vec<ArcData> = state.arcs.iter().map(|a| {
                ArcData { state: a.state + offset, weight: a.weight, ilabel: a.ilabel, olabel: a.olabel }
            }).collect();
            self.states.push(StateData {
                n_arcs: state.n_arcs,
                n_max: state.n_max,
                weight: state.weight,
                final_state: state.final_state,
                arcs: new_arcs,
            });
        }
        self.n_states += other.n_states;
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        // The C code doesn't have a union implementation shown, but the signature exists
        // Stack and return self
        self.stack(other);
        let mut result = Fst::new();
        std::mem::swap(self, &mut result);
        result
    }
    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }
    pub fn draw_sym(&self, fout: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }
    pub fn copy(&self, copy: &mut Fst) {
        copy.start = self.start;
        copy.n_states = self.n_states;
        copy.n_max = self.n_max;
        copy.sr_type = self.sr_type;
        copy.flags = self.flags;
        copy.states = self.states.iter().map(|s| StateData {
            n_arcs: s.n_arcs,
            n_max: s.n_max,
            weight: s.weight,
            final_state: s.final_state,
            arcs: s.arcs.clone(),
        }).collect();
    }
    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }
    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        let mut result = Fst::new();
        std::mem::swap(path, &mut result);
        result
    }
    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        let mut result = Fst::new();
        std::mem::swap(self, &mut result);
        result
    }
    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        let mut result = Fst::new();
        std::mem::swap(self, &mut result);
        result
    }
    pub fn compose(&self, fst_b: &Fst, fst_c: &mut Fst) {
        use crate::sr::sr_get;
        use crate::queue::Queue;
        use std::collections::HashMap;

        let sr = sr_get(self.sr_type);
        let mut q: Queue<(State, State)> = Queue::new();
        let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
        let mut marked: HashMap<(State, State), State> = HashMap::new();

        q.enqueue((self.start, fst_b.start));

        while let Some(pair) = q.dequeue() {
            let state_a = &self.states[pair.0 as usize];
            let state_b = &fst_b.states[pair.1 as usize];

            let sc = if let Some(&existing) = marked.get(&pair) {
                existing
            } else {
                let sc = fst_c.add_state();
                if state_a.final_state && state_b.final_state {
                    fst_c.set_final(sc, sr.one);
                }
                if pair.0 == self.start && pair.1 == fst_b.start {
                    fst_c.start = sc;
                }
                marked.insert(pair, sc);
                sc
            };

            let spair = Spair { a: pair.0, b: pair.1 };
            match_arcs(self, fst_b, &spair, &sr, &mut mq);

            while let Some(mi) = mq.dequeue() {
                let (arc_a, arc_b) = mi;
                let dst_pair = (arc_a.state, arc_b.state);

                let dst_sc = if let Some(&existing) = marked.get(&dst_pair) {
                    existing
                } else {
                    let dst_state_a = &self.states[dst_pair.0 as usize];
                    let dst_state_b = &fst_b.states[dst_pair.1 as usize];
                    let dst_sc = fst_c.add_state();
                    if dst_state_a.final_state && dst_state_b.final_state {
                        fst_c.set_final(dst_sc, sr.one);
                    }
                    q.enqueue(dst_pair);
                    marked.insert(dst_pair, dst_sc);
                    dst_sc
                };

                fst_c.add_arc(sc, dst_sc, arc_a.ilabel, arc_b.olabel, (sr.prod)(arc_a.weight, arc_b.weight));
            }
        }
    }
    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for state in self.states.iter_mut() {
            for arc in state.arcs.iter_mut() {
                if dir == 0 {
                    if arc.ilabel == old { arc.ilabel = new; }
                } else {
                    if arc.olabel == old { arc.olabel = new; }
                }
            }
        }
    }
}

fn _match(a: &[ArcData], b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
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
    let m = m as usize;
    let n = n as usize;
    for i in 0..m {
        let mut l: usize = 0;
        let mut h: usize = n.wrapping_sub(1);
        if n == 0 { continue; }
        while l <= h {
            let mid = (l + h) >> 1;
            if a[i].olabel > b[mid].ilabel {
                l = mid + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && a[i].olabel == b[ll - 1].ilabel { ll -= 1; }
                while hh < h && a[i].olabel == b[hh + 1].ilabel { hh += 1; }
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
    let m = m as usize;
    let n = n as usize;
    for i in 0..n {
        let mut l: usize = 0;
        let mut h: usize = m.wrapping_sub(1);
        if m == 0 { continue; }
        while l <= h {
            let mid = (l + h) >> 1;
            if b[i].ilabel > a[mid].olabel {
                l = mid + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && b[i].ilabel == a[ll - 1].olabel { ll -= 1; }
                while hh < h && b[i].ilabel == a[hh + 1].olabel { hh += 1; }
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
    let m = m as usize;
    let n = n as usize;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < m && j < n {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && a[i].olabel == b[t].ilabel {
                if _match(a, b, i, t) {
                    q.enqueue((a[i].clone(), b[t].clone()));
                }
                t += 1;
            }
            i += 1;
        }
    }
}
pub fn match_arcs(fst_a: &Fst, fst_b: &Fst, pair: &Spair, sr: &Sr, mq: &mut Queue<(ArcData, ArcData)>) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];
    let osort = fst_a.flags & OSORT != 0;
    let isort = fst_b.flags & ISORT != 0;

    let loop_a = ArcData { state: pair.a, ilabel: EPS, olabel: EPS, weight: sr.one };
    let loop_b = ArcData { state: pair.b, ilabel: EPS, olabel: EPS, weight: sr.one };

    let mut arcs_a = vec![loop_a];
    arcs_a.extend(state_a.arcs.iter().cloned());
    let mut arcs_b = vec![loop_b];
    arcs_b.extend(state_b.arcs.iter().cloned());

    let m = arcs_a.len() as Arc;
    let n = arcs_b.len() as Arc;

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
