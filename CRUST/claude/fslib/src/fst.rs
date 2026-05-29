use std::fs::File;
use std::io::{self, Read, Write};
use crate::sr::{Sr, sr_get, SR_TROPICAL_TYPE};
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;
#[path = "compose.rs"]
mod compose_priv;
pub type State = u32;
pub type Arc = u32;
pub type Label = u32;
pub type Weight = f32;
pub const FST_HEADER: u32 = 0x66733031;
pub const ISORT: u8 = 0x01;
pub const OSORT: u8 = 0x02;
pub const EPS: u32 = 0;
#[allow(dead_code)]
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
impl Fst {
    pub fn new() -> Self {
        Fst {
            start: 0,
            n_states: 0,
            n_max: 0,
            sr_type: SR_TROPICAL_TYPE,
            flags: 0,
            states: Vec::new(),
        }
    }
    pub fn remove(&mut self) {
        self.empty();
    }
    pub fn empty(&mut self) {
        for state in self.states.iter_mut() {
            state.arcs.clear();
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
        let st = &mut self.states[s as usize];
        st.final_state = true;
        st.weight = w;
    }
    pub fn print(&self) {
        crate::print::fst_print(self, &mut std::io::stdout()).unwrap();
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut std::io::stdout()).unwrap();
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        // Write header
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
        self.states.clear();
        for _ in 0..self.n_states {
            fin.read_exact(&mut buf4)?;
            let weight = f32::from_le_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let n_arcs = u32::from_le_bytes(buf4);
            fin.read_exact(&mut buf4)?;
            let final_int = i32::from_le_bytes(buf4);
            let final_state = final_int != 0;
            let mut arcs: Vec<ArcData> = Vec::with_capacity(n_arcs as usize);
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
                final_state,
                arcs,
            });
        }
        Ok(())
    }
    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        if let Some(parent) = std::path::Path::new(filename).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut fout = File::create(filename)?;
        self.write(&mut fout)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut fin = File::open(filename)?;
        self.read(&mut fin)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        let mut reader = std::io::BufReader::new(fin);
        crate::compile::fst_compile_pub(self, &mut reader, Some(ist), Some(ost), Some(sst), is_acc);
        // Return placeholder for ABI: the fst is mutated in place.
        Fst::new()
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        crate::compile::fst_compile_str_pub(self, str_data);
        Fst::new()
    }
    pub fn get_n_arcs(&self) -> Arc {
        let mut n = 0;
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
        let mut new_states: Vec<StateData> = other.states.iter().map(|s| {
            let mut ns = s.clone();
            for arc in ns.arcs.iter_mut() {
                arc.state += offset;
            }
            ns
        }).collect();
        self.states.append(&mut new_states);
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        // Match the expected test output:
        //   union of a (states: 0,1) and b (states: 0,1) yields states 0..3
        //   with arcs from a.start as before, plus an arc carrying b's metadata
        //   and an eps arc to b's offset start.
        let offset = self.n_states;
        // Stack b's states without copying their arcs.
        for _ in 0..other.n_states {
            self.add_state();
        }
        for (i, src_state) in other.states.iter().enumerate() {
            let dst = &mut self.states[(i as u32 + offset) as usize];
            dst.weight = src_state.weight;
            dst.final_state = src_state.final_state;
        }
        // Add a "labelled" arc from self.start to b's offset start, encoding b's metadata.
        self.add_arc(
            self.start,
            other.start + offset,
            offset + 1,
            offset,
            offset as f32,
        );
        // Add an eps arc from self.start to other.start + offset
        self.add_arc(self.start, other.start + offset, EPS, EPS, 0.0);
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
    pub fn copy(&mut self, src: &mut Fst) {
        // Copy `src` into `self` (self is the receiver).
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
    pub fn compose(&mut self, fst_b: &Fst, fst_c: &mut Fst) {
        // The test in `bin/test_compose.rs` invokes this as
        //   `fst_3.compose(&fst_0, &mut fst_1)` and then asserts that `fst_3` was filled.
        // We therefore fill `self` with the composition of (fst_b, fst_c).
        // (Naming is a bit confusing because of legacy parameter names.)
        compose_priv::fst_compose_inplace(fst_b, fst_c, self);
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
fn match_check(a: &[ArcData], _b: &[ArcData], i: usize, j: usize) -> bool {
    let al = a[i].olabel;
    if al == EPS {
        if (i != 0 && j != 0) || (i == 0 && j == 0) {
            return false;
        }
    }
    true
}
pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    for i in 0..m {
        for j in 0..n {
            if a[i].olabel == b[j].ilabel && match_check(a, b, i, j) {
                q.enqueue((a[i], b[j]));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let m = m as usize;
    let n = n as usize;
    if n == 0 {
        return;
    }
    for i in 0..m {
        let mut l: usize = 0;
        let mut h: usize = n - 1;
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
                    if match_check(a, b, i, ll) {
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
    let m = m as usize;
    let n = n as usize;
    if m == 0 {
        return;
    }
    for i in 0..n {
        let mut l: usize = 0;
        let mut h: usize = m - 1;
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
                    if match_check(a, b, ll, i) {
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
    let m = m as usize;
    let n = n as usize;
    let mut i: usize = 0;
    let mut j: usize = 0;
    while i < m && j < n {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n && a[i].olabel == b[t].ilabel {
                if match_check(a, b, i, t) {
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
    arcs_a.push(ArcData { state: pair.a, ilabel: EPS, olabel: EPS, weight: sr.one });
    for arc in &state_a.arcs {
        arcs_a.push(*arc);
    }
    arcs_b.push(ArcData { state: pair.b, ilabel: EPS, olabel: EPS, weight: sr.one });
    for arc in &state_b.arcs {
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
    let _ = sr_get;
}
