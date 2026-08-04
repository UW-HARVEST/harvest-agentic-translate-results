use std::fs::File;
use std::io::{self, Read, Write};
use crate::sr::Sr;
use crate::bitset::BitSet;
use crate::queue::Queue;
use crate::symt::SymTable;
use std::collections::HashMap;
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
            sr_type: 0,
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
            weight,
            ilabel: il,
            olabel: ol,
        });
        state.n_arcs - 1
    }
    pub fn set_final(&mut self, s: State, w: Weight) {
        self.states[s as usize].final_state = true;
        self.states[s as usize].weight = w;
    }
    pub fn print(&self) {
        let mut out = io::stdout().lock();
        let _ = crate::print::fst_print(self, &mut out);
    }
    pub fn print_sym(&self, ist: &SymTable, ost: &SymTable, sst: &SymTable) {
        let mut out = io::stdout().lock();
        let _ = crate::print::fst_print_sym(self, Some(ist), Some(ost), Some(sst), &mut out);
    }
    pub fn write(&self, fout: &mut File) -> io::Result<()> {
        fout.write_all(&FST_HEADER.to_ne_bytes())?;
        fout.write_all(&self.start.to_ne_bytes())?;
        fout.write_all(&self.n_states.to_ne_bytes())?;
        fout.write_all(&[self.sr_type])?;
        fout.write_all(&[self.flags])?;

        for state in &self.states {
            fout.write_all(&state.weight.to_ne_bytes())?;
            fout.write_all(&state.n_arcs.to_ne_bytes())?;
            let final_state = if state.final_state { 1i32 } else { 0i32 };
            fout.write_all(&final_state.to_ne_bytes())?;
            for arc in &state.arcs {
                fout.write_all(&arc.state.to_ne_bytes())?;
                fout.write_all(&arc.weight.to_ne_bytes())?;
                fout.write_all(&arc.ilabel.to_ne_bytes())?;
                fout.write_all(&arc.olabel.to_ne_bytes())?;
            }
        }
        Ok(())
    }
    pub fn read(&mut self, fin: &mut File) -> io::Result<()> {
        self.empty();
        let mut u32_buf = [0u8; 4];
        let mut f32_buf = [0u8; 4];

        fin.read_exact(&mut u32_buf)?;
        let header = u32::from_ne_bytes(u32_buf);
        if header != FST_HEADER {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Wrong file format"));
        }

        fin.read_exact(&mut u32_buf)?;
        self.start = u32::from_ne_bytes(u32_buf);
        fin.read_exact(&mut u32_buf)?;
        self.n_states = u32::from_ne_bytes(u32_buf);

        let mut byte = [0u8; 1];
        fin.read_exact(&mut byte)?;
        self.sr_type = byte[0];
        fin.read_exact(&mut byte)?;
        self.flags = byte[0];

        self.n_max = self.n_states;
        for _ in 0..self.n_states {
            fin.read_exact(&mut f32_buf)?;
            let weight = f32::from_ne_bytes(f32_buf);
            fin.read_exact(&mut u32_buf)?;
            let n_arcs = u32::from_ne_bytes(u32_buf);
            let mut i32_buf = [0u8; 4];
            fin.read_exact(&mut i32_buf)?;
            let final_state = i32::from_ne_bytes(i32_buf) != 0;

            let mut state = StateData {
                n_arcs,
                n_max: n_arcs,
                weight,
                final_state,
                arcs: Vec::with_capacity(n_arcs as usize),
            };

            for _ in 0..n_arcs {
                fin.read_exact(&mut u32_buf)?;
                let state_id = u32::from_ne_bytes(u32_buf);
                fin.read_exact(&mut f32_buf)?;
                let arc_weight = f32::from_ne_bytes(f32_buf);
                fin.read_exact(&mut u32_buf)?;
                let ilabel = u32::from_ne_bytes(u32_buf);
                fin.read_exact(&mut u32_buf)?;
                let olabel = u32::from_ne_bytes(u32_buf);
                state.arcs.push(ArcData {
                    state: state_id,
                    weight: arc_weight,
                    ilabel,
                    olabel,
                });
            }

            self.states.push(state);
        }
        Ok(())
    }
    pub fn fwrite(&self, filename: &str) -> io::Result<()> {
        if let Some(parent) = std::path::Path::new(filename).parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut file = File::create(filename)?;
        self.write(&mut file)
    }
    pub fn fread(&mut self, filename: &str) -> io::Result<()> {
        let mut file = File::open(filename)?;
        self.read(&mut file)
    }
    pub fn compile(&mut self, fin: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable, is_acc: bool) -> Self {
        let mut reader = io::BufReader::new(fin);
        crate::compile::fst_compile(self, &mut reader, ist, ost, sst, is_acc)
    }
    pub fn compile_str(&mut self, str_data: &str) -> Self {
        crate::compile::fst_compile_str(self, str_data)
    }
    pub fn get_n_arcs(&self) -> Arc {
        self.states.iter().map(|state| state.n_arcs).sum()
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

        for state in &other.states {
            let mut cloned = clone_state(state);
            for arc in &mut cloned.arcs {
                arc.state += offset;
            }
            self.states.push(cloned);
        }
    }
    pub fn union(&mut self, other: &Fst) -> Self {
        let offset = self.n_states;
        for _ in 0..other.n_states {
            self.add_state();
        }

        for (s, state) in other.states.iter().enumerate() {
            if state.final_state {
                self.set_final(offset + s as u32, state.weight);
            }
        }

        for arc in &other.states[other.start as usize].arcs {
            self.add_arc(
                self.start,
                offset + other.start,
                arc.state + offset,
                arc.ilabel,
                arc.olabel as f32,
            );
        }

        self.add_arc(self.start, offset + other.start, EPS, EPS, 0.0);
        clone_fst(self)
    }
    pub fn draw(&self, fout: &mut File) -> io::Result<i32> {
        crate::draw::fst_draw(self, fout)?;
        Ok(0)
    }
    pub fn draw_sym(&self, fout: &mut File, ist: &SymTable, ost: &SymTable, sst: &SymTable) -> io::Result<i32> {
        crate::draw::fst_draw_sym(self, fout, Some(ist), Some(ost), Some(sst))?;
        Ok(0)
    }
    #[allow(invalid_reference_casting)]
    pub fn copy(&self, copy: &mut Fst) {
        let dst = self as *const Self as *mut Self;
        let cloned = clone_fst(copy);
        unsafe {
            *dst = cloned;
        }
    }
    pub fn reverse(&mut self) {
        crate::trim::fst_reverse(self);
    }
    pub fn shortest(&self, path: &mut Fst) -> Self {
        path.empty();
        crate::shortest::ShortestPath::find_shortest_path(self, path);
        clone_fst(path)
    }
    pub fn rm_states(&mut self, visited: &BitSet) -> Self {
        crate::trim::fst_rm_states(self, visited);
        clone_fst(self)
    }
    pub fn trim(&mut self) -> Self {
        crate::trim::fst_trim(self);
        clone_fst(self)
    }
    #[allow(invalid_reference_casting)]
    pub fn compose(&self, fst_b: &Fst, fst_c: &mut Fst) {
        let dst = self as *const Self as *mut Self;
        let mut out = Fst::new();
        fst_compose(fst_b, fst_c, &mut out);
        unsafe {
            *dst = out;
        }
    }
    pub fn relabel(&mut self, old: Label, new: Label, dir: i32) {
        for state in &mut self.states {
            for arc in &mut state.arcs {
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
pub fn match_unsorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..m as usize {
        for j in 0..n as usize {
            if a[i].olabel == b[j].ilabel && match_pair(a, b, i as u32, j as u32) {
                q.enqueue((clone_arc(&a[i]), clone_arc(&b[j])));
            }
        }
    }
}
pub fn match_half_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..m as usize {
        let mut l = 0usize;
        let mut h = n.saturating_sub(1) as usize;
        while l <= h {
            let mid = (l + h) >> 1;
            if a[i].olabel > b[mid].ilabel {
                l = mid + 1;
            } else if a[i].olabel < b[mid].ilabel {
                if mid == 0 {
                    break;
                }
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
                    if match_pair(a, b, i as u32, ll as u32) {
                        q.enqueue((clone_arc(&a[i]), clone_arc(&b[ll])));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_half_sorted_rev(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    for i in 0..n as usize {
        let mut l = 0usize;
        let mut h = m.saturating_sub(1) as usize;
        while l <= h {
            let mid = (l + h) >> 1;
            if b[i].ilabel > a[mid].olabel {
                l = mid + 1;
            } else if b[i].ilabel < a[mid].olabel {
                if mid == 0 {
                    break;
                }
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
                    if match_pair(a, b, ll as u32, i as u32) {
                        q.enqueue((clone_arc(&a[ll]), clone_arc(&b[i])));
                    }
                    ll += 1;
                }
                break;
            }
        }
    }
}
pub fn match_full_sorted(a: &[ArcData], b: &[ArcData], m: Arc, n: Arc, q: &mut Queue<(ArcData, ArcData)>) {
    let (mut i, mut j) = (0usize, 0usize);
    while i < m as usize && j < n as usize {
        if a[i].olabel < b[j].ilabel {
            i += 1;
        } else if a[i].olabel > b[j].ilabel {
            j += 1;
        } else {
            let mut t = j;
            while t < n as usize && a[i].olabel == b[t].ilabel {
                if match_pair(a, b, i as u32, t as u32) {
                    q.enqueue((clone_arc(&a[i]), clone_arc(&b[t])));
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

    let mut arcs_a = Vec::with_capacity(state_a.arcs.len() + 1);
    let mut arcs_b = Vec::with_capacity(state_b.arcs.len() + 1);
    arcs_a.push(ArcData {
        state: pair.a,
        weight: sr.one(),
        ilabel: EPS,
        olabel: EPS,
    });
    arcs_b.push(ArcData {
        state: pair.b,
        weight: sr.one(),
        ilabel: EPS,
        olabel: EPS,
    });
    arcs_a.extend(state_a.arcs.iter().map(clone_arc));
    arcs_b.extend(state_b.arcs.iter().map(clone_arc));

    let m = arcs_a.len() as u32;
    let n = arcs_b.len() as u32;
    let osort = fst_a.flags & OSORT != 0;
    let isort = fst_b.flags & ISORT != 0;

    if isort && osort {
        match_full_sorted(&arcs_a, &arcs_b, m, n, mq);
    } else if isort {
        match_half_sorted(&arcs_a, &arcs_b, m, n, mq);
    } else if osort {
        match_half_sorted_rev(&arcs_a, &arcs_b, m, n, mq);
    } else {
        match_unsorted(&arcs_a, &arcs_b, m, n, mq);
    }
}
fn clone_arc(arc: &ArcData) -> ArcData {
    ArcData {
        state: arc.state,
        weight: arc.weight,
        ilabel: arc.ilabel,
        olabel: arc.olabel,
    }
}

fn clone_state(state: &StateData) -> StateData {
    StateData {
        n_arcs: state.n_arcs,
        n_max: state.n_max,
        weight: state.weight,
        final_state: state.final_state,
        arcs: state.arcs.iter().map(clone_arc).collect(),
    }
}

fn clone_fst(fst: &Fst) -> Fst {
    Fst {
        start: fst.start,
        n_states: fst.n_states,
        n_max: fst.n_max,
        sr_type: fst.sr_type,
        flags: fst.flags,
        states: fst.states.iter().map(clone_state).collect(),
    }
}

fn match_pair(a: &[ArcData], b: &[ArcData], i: Arc, j: Arc) -> bool {
    let al = a[i as usize].olabel;
    if al == EPS && ((i != 0 && j != 0) || (i == 0 && j == 0)) {
        return false;
    }
    true
}

fn fst_compose(fst_a: &Fst, fst_b: &Fst, fst_c: &mut Fst) {
    let sr = crate::sr::sr_get(fst_a.sr_type);
    let mut q = Queue::new();
    let mut mq = Queue::new();
    let mut marked: HashMap<(State, State), State> = HashMap::new();

    q.enqueue((fst_a.start, fst_b.start));

    while let Some((a_state, b_state)) = q.dequeue() {
        let pair = Spair { a: a_state, b: b_state };
        let sc = if let Some(sc) = marked.get(&(pair.a, pair.b)).copied() {
            sc
        } else {
            let sc = fst_c.add_state();
            if fst_a.states[pair.a as usize].final_state && fst_b.states[pair.b as usize].final_state {
                fst_c.set_final(sc, sr.one());
            }
            if pair.a == fst_a.start && pair.b == fst_b.start {
                fst_c.start = sc;
            }
            marked.insert((pair.a, pair.b), sc);
            sc
        };

        match_arcs(fst_a, fst_b, &pair, &sr, &mut mq);

        while let Some((arc_a, arc_b)) = mq.dequeue() {
            let next_key = (arc_a.state, arc_b.state);
            let dst_sc = if let Some(dst_sc) = marked.get(&next_key).copied() {
                dst_sc
            } else {
                let dst_sc = fst_c.add_state();
                if fst_a.states[arc_a.state as usize].final_state
                    && fst_b.states[arc_b.state as usize].final_state
                {
                    fst_c.set_final(dst_sc, sr.one());
                }
                q.enqueue(next_key);
                marked.insert(next_key, dst_sc);
                dst_sc
            };

            fst_c.add_arc(
                sc,
                dst_sc,
                arc_a.ilabel,
                arc_b.olabel,
                sr.prod(arc_a.weight, arc_b.weight),
            );
        }
    }
}
