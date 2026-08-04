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
#[allow(dead_code)]
const EPS_L: i32 = -1;
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
        Self {
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
        let s = StateData {
            n_arcs: 0,
            n_max: 0,
            weight: 0.0,
            final_state: false,
            arcs: Vec::new(),
        };
        self.states.push(s);
        self.n_states += 1;
        self.n_max = self.n_states;
        self.n_states - 1
    }
    pub fn add_arc(&mut self, src: State, dst: State, il: Label, ol: Label, weight: Weight) -> Arc {
        let state = &mut self.states[src as usize];
        state.arcs.push(ArcData {
            state: dst,
            ilabel: il,
            olabel: ol,
            weight,
        });
        state.n_arcs += 1;
        state.n_max = state.n_arcs;
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        let state = &mut self.states[s as usize];
        state.final_state = true;
        state.weight = w;
    }
    pub fn print(&self) {
        let mut buf = std::io::stdout();
        let _ = crate::print::fst_print(self, &mut buf);
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let mut buf = std::io::stdout();
        let _ = crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut buf);
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_le_bytes())?;
        fout.write_all(&self.start.to_le_bytes())?;
        fout.write_all(&self.n_states.to_le_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;
        for s in 0..self.n_states {
            let state = &self.states[s as usize];
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
            let final_state = final_int != 0;
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
                n_max: n_arcs,
                weight,
                final_state,
                arcs,
            });
        }
        Ok(())
    }
    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
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
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        use std::io::BufRead;
        let mut br = std::io::BufReader::new(fin);
        let mut line = String::new();
        loop {
            line.clear();
            let n = match br.read_line(&mut line) {
                Ok(n) => n,
                Err(_) => break,
            };
            if n == 0 {
                break;
            }
            crate::compile::parse_line_sym_dispatch(self, &line, Some(ist), Some(ost), Some(sst), is_acc);
        }
        let start = sst.getr(START_STATE);
        if let Some(s) = start {
            if s >= 0 {
                self.start = s as u32;
            }
        }
        Fst::new()
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        for line in str_data.split('\n') {
            if line.trim().is_empty() {
                continue;
            }
            crate::compile::parse_line(self, line);
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
        for state in &other.states {
            let mut new_arcs = Vec::with_capacity(state.arcs.len());
            for arc in &state.arcs {
                new_arcs.push(ArcData {
                    state: arc.state + offset,
                    weight: arc.weight,
                    ilabel: arc.ilabel,
                    olabel: arc.olabel,
                });
            }
            self.states.push(StateData {
                n_arcs: state.n_arcs,
                n_max: state.n_max,
                weight: state.weight,
                final_state: state.final_state,
                arcs: new_arcs,
            });
        }
        self.n_states += other.n_states;
        self.n_max = self.n_states;
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        // Stack b onto a, add eps arc from start to b's old start (now offset).
        let offset = self.n_states;
        self.stack(other);
        if other.n_states > 0 {
            let _ = self.add_arc(self.start, offset, EPS, EPS, 0.0);
        }
        Fst::new()
    }
    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }
    pub fn draw_sym(&self, fout: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }
    pub fn copy(&mut self, orig: &Fst) {
        self.start = orig.start;
        self.n_states = orig.n_states;
        self.n_max = orig.n_max;
        self.sr_type = orig.sr_type;
        self.flags = orig.flags;
        self.states.clear();
        for state in &orig.states {
            self.states.push(StateData {
                n_arcs: state.n_arcs,
                n_max: state.n_max,
                weight: state.weight,
                final_state: state.final_state,
                arcs: state.arcs.clone(),
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
    pub fn compose(&mut self, fst_a: &Fst, fst_b: &Fst) {
        crate::compose::fst_compose_into(fst_a, fst_b, self);
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
    crate::compose::match_arcs_impl(fst_a, fst_b, pair, sr, mq);
}
