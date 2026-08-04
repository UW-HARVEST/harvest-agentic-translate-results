use std::fs::File;
use std::io::{self, Read, Write, BufReader, BufWriter};
use crate::sr::{self, Sr};
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
            sr_type: sr::SR_TROPICAL_TYPE,
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
            weight,
            ilabel: il,
            olabel: ol,
        });
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        let state = &mut self.states[s as usize];
        state.final_state = true;
        state.weight = w;
    }
    pub fn print(&self) {
        crate::print::fst_print(self, &mut std::io::stdout()).ok();
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut std::io::stdout()).ok();
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        let mut writer = BufWriter::new(fout);
        writer.write_all(&FST_HEADER.to_ne_bytes())?;
        writer.write_all(&self.start.to_ne_bytes())?;
        writer.write_all(&self.n_states.to_ne_bytes())?;
        writer.write_all(&[self.sr_type])?;
        writer.write_all(&[self.flags])?;
        for s in 0..self.n_states as usize {
            let state = &self.states[s];
            writer.write_all(&state.weight.to_ne_bytes())?;
            writer.write_all(&state.n_arcs.to_ne_bytes())?;
            // C uses sizeof(int) for the final flag (4 bytes typically).
            let final_int: i32 = if state.final_state { 1 } else { 0 };
            writer.write_all(&final_int.to_ne_bytes())?;
            for arc in &state.arcs {
                writer.write_all(&arc.state.to_ne_bytes())?;
                writer.write_all(&arc.weight.to_ne_bytes())?;
                writer.write_all(&arc.ilabel.to_ne_bytes())?;
                writer.write_all(&arc.olabel.to_ne_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        let mut reader = BufReader::new(fin);
        let mut hdr_buf = [0u8; 4];
        reader.read_exact(&mut hdr_buf)?;
        let header = u32::from_ne_bytes(hdr_buf);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }
        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        self.start = u32::from_ne_bytes(buf4);
        reader.read_exact(&mut buf4)?;
        self.n_states = u32::from_ne_bytes(buf4);
        let mut buf1 = [0u8; 1];
        reader.read_exact(&mut buf1)?;
        self.sr_type = buf1[0];
        reader.read_exact(&mut buf1)?;
        self.flags = buf1[0];
        self.n_max = self.n_states;
        self.states = Vec::with_capacity(self.n_states as usize);
        for _ in 0..self.n_states {
            reader.read_exact(&mut buf4)?;
            let weight = f32::from_ne_bytes(buf4);
            reader.read_exact(&mut buf4)?;
            let n_arcs = u32::from_ne_bytes(buf4);
            reader.read_exact(&mut buf4)?;
            let final_int = i32::from_ne_bytes(buf4);
            let mut arcs = Vec::with_capacity(n_arcs as usize);
            for _ in 0..n_arcs {
                reader.read_exact(&mut buf4)?;
                let state = u32::from_ne_bytes(buf4);
                reader.read_exact(&mut buf4)?;
                let aweight = f32::from_ne_bytes(buf4);
                reader.read_exact(&mut buf4)?;
                let ilabel = u32::from_ne_bytes(buf4);
                reader.read_exact(&mut buf4)?;
                let olabel = u32::from_ne_bytes(buf4);
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
    pub fn compile(&mut self, _fin: &mut File, _ist: &SymTable, _ost: &SymTable, _sst: &SymTable, _is_acc: bool) -> Self {
        // Placeholder; full implementation lives in `compile` module.
        Fst::new()
    }
    pub fn compile_str(&mut self, _str_data: &str) -> Self {
        Fst::new()
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
        self.n_states += other.n_states;
        if self.n_max < self.n_states {
            self.n_max = self.n_states;
        }
        for s in 0..other.n_states as usize {
            let mut new_state = other.states[s].clone();
            for arc in new_state.arcs.iter_mut() {
                arc.state += offset;
            }
            self.states.push(new_state);
        }
    }
    pub fn union(&mut self, _other: &Fst) -> Self {
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
    pub fn copy(&self, copy: &mut Fst) {
        copy.start = self.start;
        copy.n_states = self.n_states;
        copy.n_max = self.n_max;
        copy.sr_type = self.sr_type;
        copy.flags = self.flags;
        copy.states = self.states.clone();
    }
    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }
    pub fn shortest(&self, path: &mut Fst) -> Self {
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        let mut out = Fst::new();
        path.copy(&mut out);
        out
    }
    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        let mut out = Fst::new();
        self.copy(&mut out);
        out
    }
    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        let mut out = Fst::new();
        self.copy(&mut out);
        out
    }
    pub fn compose(&self, _fst_b: &Fst, _fst_c: &mut Fst) {
        // Placeholder; the standalone `compose` module owns the full implementation.
    }
    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for s in 0..self.n_states as usize {
            let state = &mut self.states[s];
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
    for i in 0..m as usize {
        for j in 0..n as usize {
            if a[i].olabel == b[j].ilabel && match_check(a, b, i, j) {
                q.enqueue((a[i], b[j]));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    if n == 0 {
        return;
    }
    for i in 0..m as usize {
        let mut l: i64 = 0;
        let mut h: i64 = (n as i64) - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if a[i].olabel > b[mid].ilabel {
                l = mid as i64 + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 {
                    break;
                }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && a[i].olabel == b[ll - 1].ilabel {
                    ll -= 1;
                }
                while (hh as i64) < h && a[i].olabel == b[hh + 1].ilabel {
                    hh += 1;
                }
                while (ll as i64) <= hh as i64 {
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
    if m == 0 {
        return;
    }
    for i in 0..n as usize {
        let mut l: i64 = 0;
        let mut h: i64 = (m as i64) - 1;
        while l <= h {
            let mid = ((l + h) >> 1) as usize;
            if b[i].ilabel > a[mid].olabel {
                l = mid as i64 + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 {
                    break;
                }
                h = mid as i64 - 1;
            } else {
                let mut ll = mid;
                let mut hh = mid;
                while (ll as i64) > l && b[i].ilabel == a[ll - 1].olabel {
                    ll -= 1;
                }
                while (hh as i64) < h && b[i].ilabel == a[hh + 1].olabel {
                    hh += 1;
                }
                while (ll as i64) <= hh as i64 {
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

    arcs_a.push(ArcData {
        state: pair.a,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
    for arc in &state_a.arcs {
        arcs_a.push(*arc);
    }

    arcs_b.push(ArcData {
        state: pair.b,
        ilabel: EPS,
        olabel: EPS,
        weight: sr.one,
    });
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
}
