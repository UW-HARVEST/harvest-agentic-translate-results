use std::fs::File;
use std::io::{self, Read, Write};
use crate::sr::Sr;
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
pub const SR_TROPICAL: u8 = 0;
pub const SR_REAL: u8 = 1;
pub const START_STATE: &str = "<start>";

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
#[derive(Clone, Copy)]
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

impl Default for Fst {
    fn default() -> Self {
        Self::new()
    }
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
        self.n_states - 1
    }
    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let state = &mut self.states[src as usize];
        state.n_arcs += 1;
        state.arcs.push(ArcData {
            state: dst,
            weight,
            ilabel: il,
            olabel: ol,
        });
        if state.n_arcs > state.n_max {
            state.n_max = state.n_arcs * 2;
        }
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
        for s in 0..self.n_states as usize {
            let state = &self.states[s];
            fout.write_all(&state.weight.to_le_bytes())?;
            fout.write_all(&state.n_arcs.to_le_bytes())?;
            // C uses `int final` - 4 bytes
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            fout.write_all(&final_int.to_le_bytes())?;
            for a in &state.arcs {
                fout.write_all(&a.state.to_le_bytes())?;
                fout.write_all(&a.weight.to_le_bytes())?;
                fout.write_all(&a.ilabel.to_le_bytes())?;
                fout.write_all(&a.olabel.to_le_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut buf4 = [0u8; 4];
        fin.read_exact(&mut buf4)?;
        let header = u32::from_le_bytes(buf4);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_le_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_le_bytes(buf4);
        let mut buf1 = [0u8; 1];
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
                fin.read_exact(&mut buf4)?;
                let st = u32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let w = f32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let il = u32::from_le_bytes(buf4);
                fin.read_exact(&mut buf4)?;
                let ol = u32::from_le_bytes(buf4);
                arcs.push(ArcData { state: st, weight: w, ilabel: il, olabel: ol });
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
        if let Some(parent) = std::path::Path::new(filename).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).ok();
            }
        }
        let mut f = File::create(filename)?;
        self.write(&mut f)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut f = File::open(filename)?;
        self.read(&mut f)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        let mut br = io::BufReader::new(fin);
        crate::compile::fst_compile(self, &mut br, ist, ost, sst, is_acc)
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        crate::compile::fst_compile_str(self, str_data)
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
        let total = self.n_states + other.n_states;
        for i in 0..other.n_states as usize {
            let mut new_state = other.states[i].clone();
            for arc in new_state.arcs.iter_mut() {
                arc.state += offset;
            }
            self.states.push(new_state);
        }
        self.n_states = total;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        // Union: stack b onto a; merge b's start arcs into a's start (with shifts);
        // clear b's start state arcs in stacked region; add eps arc to b's shifted start.
        let offset = self.n_states;
        let a_start = self.start;
        let b_start = other.start;
        // Stack
        self.stack(other);
        // Add b's arcs (shifted) onto self.states[a_start], all targeting b's shifted start
        for arc in other.states[b_start as usize].arcs.iter() {
            self.add_arc(
                a_start,
                b_start + offset,
                arc.ilabel + 1,
                arc.olabel,
                arc.ilabel as f32,
            );
        }
        // Add eps arc from a.start to b's shifted start
        let sr = crate::sr::sr_get(self.sr_type);
        self.add_arc(a_start, b_start + offset, 0, 0, sr.one);
        // Remove b's start arcs from the stacked region
        let stacked_b_start = (b_start + offset) as usize;
        self.states[stacked_b_start].arcs.clear();
        self.states[stacked_b_start].n_arcs = 0;
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
    pub fn copy(&mut self, copy_from: &mut Fst) {
        // The test does: fst_b.copy(&mut fst_a)
        // So fst_b becomes a copy of fst_a
        self.start = copy_from.start;
        self.n_states = copy_from.n_states;
        self.n_max = copy_from.n_max;
        self.sr_type = copy_from.sr_type;
        self.flags = copy_from.flags;
        self.states = copy_from.states.clone();
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
        fst_compose_impl(fst_a, fst_b, self);
    }
    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for state in self.states.iter_mut() {
            for arc in state.arcs.iter_mut() {
                if dir == 0 {
                    if arc.ilabel == old {
                        arc.ilabel = new;
                    }
                } else if arc.olabel == old {
                    arc.olabel = new;
                }
            }
        }
    }
}

// Helpers exported from this module for tests/other modules
pub fn match_unsorted(a: &[ArcData], b: &[ArcData], _m: Arc, _n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    crate::matcher::match_unsorted(a, b, q);
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], _m: Arc, _n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    crate::matcher::match_half_sorted(a, b, q);
}
pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], _m: Arc, _n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    crate::matcher::match_half_sorted_rev(a, b, q);
}
pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], _m: Arc, _n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    crate::matcher::match_full_sorted(a, b, q);
}
pub fn match_arcs(fst_a: &Fst, fst_b: &Fst, pair: &Spair, sr: &Sr, mq: &mut Queue<(ArcData, ArcData)>) {
    let state_a = &fst_a.states[pair.a as usize];
    let state_b = &fst_b.states[pair.b as usize];
    let osort = (fst_a.flags & OSORT) != 0;
    let isort = (fst_b.flags & ISORT) != 0;
    let mut arcs_a: Vec<ArcData> = Vec::with_capacity(state_a.arcs.len() + 1);
    let mut arcs_b: Vec<ArcData> = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    arcs_a.extend_from_slice(&state_a.arcs);
    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    arcs_b.extend_from_slice(&state_b.arcs);
    if isort && osort {
        crate::matcher::match_full_sorted(&arcs_a, &arcs_b, mq);
    } else if isort || osort {
        if isort {
            crate::matcher::match_half_sorted(&arcs_a, &arcs_b, mq);
        } else {
            crate::matcher::match_half_sorted_rev(&arcs_a, &arcs_b, mq);
        }
    } else {
        crate::matcher::match_unsorted(&arcs_a, &arcs_b, mq);
    }
}

pub fn fst_compose_impl(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    use std::collections::HashMap;
    let sr = crate::sr::sr_get(fst_a.sr_type);
    let mut q: Queue<(u32, u32)> = Queue::new();
    let mut mq: Queue<(ArcData, ArcData)> = Queue::new();
    let mut marked: HashMap<(u32, u32), u32> = HashMap::new();
    let init_pair = (fst_a.start, fst_b.start);
    q.enqueue(init_pair);
    while let Some(pair) = q.dequeue() {
        let state_a = &fst_a.states[pair.0 as usize];
        let state_b = &fst_b.states[pair.1 as usize];
        let sc = if let Some(&existing) = marked.get(&pair) {
            existing
        } else {
            let new_state = fst_c.add_state();
            if state_a.final_state && state_b.final_state {
                fst_c.set_final(new_state, sr.one);
            }
            if pair.0 == fst_a.start && pair.1 == fst_b.start {
                fst_c.start = new_state;
            }
            marked.insert(pair, new_state);
            new_state
        };
        let spair = Spair { a: pair.0, b: pair.1 };
        match_arcs(fst_a, fst_b, &spair, &sr, &mut mq);
        while let Some(mi) = mq.dequeue() {
            let arc_a = mi.0;
            let arc_b = mi.1;
            let new_pair = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(&existing) = marked.get(&new_pair) {
                existing
            } else {
                let dst_state_a = &fst_a.states[new_pair.0 as usize];
                let dst_state_b = &fst_b.states[new_pair.1 as usize];
                let new_state = fst_c.add_state();
                if dst_state_a.final_state && dst_state_b.final_state {
                    fst_c.set_final(new_state, sr.one);
                }
                q.enqueue(new_pair);
                marked.insert(new_pair, new_state);
                new_state
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
