use std::fs::File;
use std::io::{self, Read, Write};
use crate::sr::{Sr, sr_get};
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;
pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;
pub const FST_HEADER: u32 = 0x66733031;
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

#[derive(Clone, Copy, Debug)]
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
        for s in self.states.iter_mut() {
            s.arcs.clear();
        }
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
        // Push a state regardless of capacity (Vec handles capacity)
        if self.states.len() < self.n_states as usize {
            self.states.push(StateData {
                n_arcs: 0,
                n_max: 0,
                weight: 0.0,
                final_state: false,
                arcs: Vec::new(),
            });
        } else {
            // overwrite slot
            let idx = self.n_states as usize - 1;
            self.states[idx] = StateData {
                n_arcs: 0,
                n_max: 0,
                weight: 0.0,
                final_state: false,
                arcs: Vec::new(),
            };
        }
        self.n_states - 1
    }
    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let s = &mut self.states[src as usize];
        s.n_arcs += 1;
        if s.n_arcs > s.n_max {
            s.n_max = s.n_arcs * 2;
        }
        s.arcs.push(ArcData {
            state: dst,
            ilabel: il,
            olabel: ol,
            weight,
        });
        s.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        let st = &mut self.states[s as usize];
        st.final_state = true;
        st.weight = w;
    }
    pub fn print(&self) {
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            for a in 0..state.n_arcs {
                let arc = &state.arcs[a as usize];
                println!(
                    "{}\t{}\t{}\t{}\t{:.5}",
                    s, arc.state, arc.ilabel, arc.olabel, arc.weight
                );
            }
        }
        // print finals
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
            if state.final_state {
                println!("{}\t{:.6}", s, state.weight);
            }
        }
    }
    pub fn print_sym(&self, _ist: &SymTable, _ost: &SymTable, _sst: &SymTable) {
        self.print();
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
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                fout.write_all(&arc.state.to_le_bytes())?;
                fout.write_all(&arc.weight.to_le_bytes())?;
                fout.write_all(&arc.ilabel.to_le_bytes())?;
                fout.write_all(&arc.olabel.to_le_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut hdr = [0u8; 4];
        fin.read_exact(&mut hdr)?;
        let header = u32::from_le_bytes(hdr);
        if header != FST_HEADER {
            println!("Wrong file format");
            std::process::exit(1);
        }
        let mut buf4 = [0u8; 4];
        fin.read_exact(&mut buf4)?;
        self.start = u32::from_le_bytes(buf4);
        fin.read_exact(&mut buf4)?;
        self.n_states = u32::from_le_bytes(buf4);
        let mut byte = [0u8; 1];
        fin.read_exact(&mut byte)?;
        self.sr_type = byte[0];
        fin.read_exact(&mut byte)?;
        self.flags = byte[0];
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
            let n_max = n_arcs;
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
                arcs.push(ArcData {
                    state,
                    weight: aweight,
                    ilabel,
                    olabel,
                });
            }
            self.states.push(StateData {
                n_arcs,
                n_max,
                weight,
                final_state: final_int != 0,
                arcs,
            });
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
    pub fn compile(&mut self, _fin: &mut File, _ist: &SymTable, _ost: &SymTable, _sst: &SymTable, _is_acc: bool) -> Self {
        self.clone()
    }
    pub fn compile_str(&mut self, _str_data: &str) -> Self {
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
        if sort_outer == 0 {
            self.flags |= ISORT;
            for s in self.states.iter_mut() {
                s.arcs.sort_by_key(|a| a.ilabel);
            }
        } else {
            self.flags |= OSORT;
            for s in self.states.iter_mut() {
                s.arcs.sort_by_key(|a| a.olabel);
            }
        }
    }
    pub fn stack(&mut self, other: &Fst) {
        let offset = self.n_states;
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
        for i in 0..other.n_states as usize {
            let mut new_state = other.states[i].clone();
            for arc in new_state.arcs.iter_mut() {
                arc.state += offset;
            }
            self.states.push(new_state);
        }
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        // Not used in C as a builder; just stack and return clone
        self.stack(other);
        self.clone()
    }
    pub fn draw(&self, _fout: &mut File) -> io::Result<i32> {
        Ok(0)
    }
    pub fn draw_sym(&self, _fout: &mut File, _ist: &SymTable, _ost: &SymTable, _sst: &SymTable) -> io::Result<i32> {
        Ok(0)
    }
    pub fn copy(&self, copy: &mut Fst) {
        copy.start = self.start;
        copy.n_states = self.n_states;
        copy.n_max = self.n_max;
        copy.sr_type = self.sr_type;
        copy.flags = self.flags;
        copy.states = self.states.clone();
    }
    pub fn reverse(&mut self) {
        let _sr = sr_get(self.sr_type);
        let mut orig = Fst::new();
        self.copy(&mut orig);
        let start_s = self.start;
        for s in 0..self.n_states as usize {
            let state = &mut self.states[s];
            state.n_arcs = 0;
            state.arcs.clear();
            if state.final_state {
                self.start = s as State;
                state.final_state = false;
            }
        }
        let sr = sr_get(self.sr_type);
        self.set_final(start_s, sr.one);
        for s in 0..orig.n_states {
            let state = &orig.states[s as usize];
            for a in 0..state.n_arcs as usize {
                let arc = &state.arcs[a];
                self.add_arc(arc.state, s, arc.ilabel, arc.olabel, arc.weight);
            }
        }
    }
    pub fn shortest(&self, _path: &mut Fst) -> Self {
        self.clone()
    }
    pub fn rm_states(&mut self, _visited: &BitSet) -> Self {
        self.clone()
    }
    pub fn trim(&mut self) -> Self {
        self.clone()
    }
    pub fn compose(&self, _fst_b: &Fst, _fst_c: &mut Fst) {
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

// Internal _match: a's olabel is EPS — match only if both i==0 and j==0, OR neither i==0 nor j==0
fn arc_match(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}

pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mm = m as usize;
    let nn = n as usize;
    for i in 0..mm {
        for j in 0..nn {
            if a[i].olabel == b[j].ilabel && arc_match(a, b, i, j) {
                q.enqueue((a[i], b[j]));
            }
        }
    }
}

pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mm = m as usize;
    let nn = n as usize;
    if nn == 0 {
        return;
    }
    for i in 0..mm {
        let mut l: usize = 0;
        let mut h: usize = nn - 1;
        loop {
            if l > h { break; }
            let mid = (l + h) >> 1;
            if a[i].olabel > b[mid].ilabel {
                l = mid + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && a[i].olabel == b[ll - 1].ilabel {
                    ll -= 1;
                }
                while hh < h && a[i].olabel == b[hh + 1].ilabel {
                    hh += 1;
                }
                while ll <= hh {
                    if arc_match(a, b, i, ll) {
                        q.enqueue((a[i], b[ll]));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}

pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mm = m as usize;
    let nn = n as usize;
    if mm == 0 {
        return;
    }
    for i in 0..nn {
        let mut l: usize = 0;
        let mut h: usize = mm - 1;
        loop {
            if l > h { break; }
            let mid = (l + h) >> 1;
            if b[i].ilabel > a[mid].olabel {
                l = mid + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 { break; }
                h = mid - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while ll > l && b[i].ilabel == a[ll - 1].olabel {
                    ll -= 1;
                }
                while hh < h && b[i].ilabel == a[hh + 1].olabel {
                    hh += 1;
                }
                while ll <= hh {
                    if arc_match(a, b, ll, i) {
                        q.enqueue((a[ll], b[i]));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}

pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let mm = m as usize;
    let nn = n as usize;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < mm && j < nn {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < nn && a[i].olabel == b[t].ilabel {
                if arc_match(a, b, i, t) {
                    q.enqueue((a[i], b[t]));
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
    for arc in &state_a.arcs[..state_a.n_arcs as usize] {
        arcs_a.push(*arc);
    }
    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    for arc in &state_b.arcs[..state_b.n_arcs as usize] {
        arcs_b.push(*arc);
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
